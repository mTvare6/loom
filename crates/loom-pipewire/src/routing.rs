// SPDX-License-Identifier: MPL-2.0

use pipewire as pw;
use pw::spa::utils::result::AsyncSeq;
use pw::{
    core::CoreRc,
    link::Link,
    metadata::{Metadata, MetadataListener},
    properties::properties,
    proxy::ProxyT,
    registry::{GlobalObject, RegistryRc},
    types::ObjectType,
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

const FILTER_NODE_NAME: &str = "loom_virtual_sink";
const DEFAULT_METADATA_NAME: &str = "default";
const DEFAULT_SINK_KEY: &str = "default.audio.sink";
const DEFAULT_SINK_TYPE: &str = "Spa:String:JSON";
const TARGET_SINK_KEY: &str = "loom.target.audio.sink";

pub(crate) struct Routing {
    // FIXME: Decide if storing semantics matters given _state has it
    _registry: RegistryRc,
    _listener: pw::registry::Listener,
    _state: Rc<RefCell<State>>,
}

impl Drop for Routing {
    fn drop(&mut self) {
        let mut state = self._state.borrow_mut();
        state.prepare_shutdown();
        state.finish_shutdown();
    }
}

impl Routing {
    pub(crate) fn new(core: &CoreRc) -> Result<Self, pw::Error> {
        let registry = core.get_registry_rc()?;
        let state = Rc::new(RefCell::new(State::new(core.clone(), registry.clone())));
        let registry_weak = registry.downgrade();
        let state_for_global = state.clone();
        let state_for_remove = state.clone();

        let listener = registry
            .add_listener_local()
            .global(move |object| {
                let Some(registry) = registry_weak.upgrade() else {
                    return;
                };

                if is_default_metadata(object) {
                    match bind_default_metadata(&registry, object, &state_for_global) {
                        Ok(binding) => {
                            let mut state = state_for_global.borrow_mut();
                            state.metadata = Some(binding);
                            state.reconcile();
                        }
                        Err(error) => {
                            eprintln!("Loom could not observe PipeWire defaults: {error}")
                        }
                    }
                } else {
                    state_for_global.borrow_mut().add_global(object);
                }
            })
            .global_remove(move |id| state_for_remove.borrow_mut().remove_global(id))
            .register();

        Ok(Self {
            _registry: registry,
            _listener: listener,
            _state: state,
        })
    }

    pub(crate) fn begin_shutdown(&self) -> Result<AsyncSeq, pw::Error> {
        let core = {
            let mut state = self._state.borrow_mut();
            state.prepare_shutdown();
            state.core.clone()
        };
        core.sync(0)
    }

    pub(crate) fn finish_shutdown(&self) -> Result<AsyncSeq, pw::Error> {
        let core = {
            let mut state = self._state.borrow_mut();
            state.finish_shutdown();
            state.core.clone()
        };
        core.sync(0)
    }

    pub(crate) fn release_shutdown_links(&self) {
        self._state.borrow_mut().shutdown_links.clear();
    }
}

struct MetadataBinding {
    id: u32,
    // Listener must be removed before it's proxy is
    _listener: MetadataListener,
    _metadata: Metadata,
}

#[derive(Clone, Debug)]
struct Node {
    id: u32,
    name: String,
    media_class: String,
    priority: u32,
}

impl Node {
    fn is_player(&self) -> bool {
        self.media_class == "Stream/Output/Audio"
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Channel {
    Left,
    Right,
    Mono,
}

impl Channel {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "FL" => Some(Self::Left),
            "FR" => Some(Self::Right),
            "MONO" => Some(Self::Mono),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug)]
struct Port {
    id: u32,
    node_id: u32,
    direction: PortDirection,
    channel: Channel,
}

#[derive(Clone, Copy, Debug)]
struct ExistingLink {
    id: u32,
    output_node: u32,
    output_port: u32,
    input_node: u32,
    input_port: u32,
}

struct State {
    core: CoreRc,
    registry: RegistryRc,
    nodes: HashMap<u32, Node>,
    ports: HashMap<u32, Port>,
    links: HashMap<u32, ExistingLink>,
    metadata: Option<MetadataBinding>,
    default_sink_name: Option<String>,
    default_sink_value: Option<String>,
    saved_sink_name: Option<String>,
    saved_sink_value: Option<String>,
    using_fallback_sink: bool,
    filter_is_default: bool,
    output_links: HashMap<Channel, Link>,
    player_links: HashMap<(u32, Channel), Link>,
    shutdown_links: Vec<Link>,
    linked_sink: Option<u32>,
    shutting_down: bool,
    shutdown_links_created: bool,
}

impl State {
    fn new(core: CoreRc, registry: RegistryRc) -> Self {
        Self {
            core,
            registry,
            nodes: HashMap::new(),
            ports: HashMap::new(),
            links: HashMap::new(),
            metadata: None,
            default_sink_name: None,
            default_sink_value: None,
            saved_sink_name: None,
            saved_sink_value: None,
            using_fallback_sink: false,
            filter_is_default: false,
            output_links: HashMap::new(),
            player_links: HashMap::new(),
            shutdown_links: Vec::new(),
            linked_sink: None,
            shutting_down: false,
            shutdown_links_created: false,
        }
    }

    fn add_global(&mut self, object: &GlobalObject<&pw::spa::utils::dict::DictRef>) {
        let Some(props) = object.props else {
            return;
        };

        match object.type_ {
            ObjectType::Node => {
                let Some(name) = props.get(*pw::keys::NODE_NAME) else {
                    return;
                };
                self.nodes.insert(
                    object.id,
                    Node {
                        id: object.id,
                        name: name.to_owned(),
                        media_class: props.get(*pw::keys::MEDIA_CLASS).unwrap_or("").to_owned(),
                        priority: props
                            .get("priority.session")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                    },
                );
            }
            ObjectType::Port => {
                let Some(node_id) = props
                    .get(*pw::keys::NODE_ID)
                    .and_then(|value| value.parse().ok())
                else {
                    return;
                };
                let direction = match props.get(*pw::keys::PORT_DIRECTION) {
                    Some("in") => PortDirection::Input,
                    Some("out") => PortDirection::Output,
                    _ => return,
                };
                let Some(channel) = props.get(*pw::keys::AUDIO_CHANNEL).and_then(Channel::parse)
                else {
                    return;
                };
                self.ports.insert(
                    object.id,
                    Port {
                        id: object.id,
                        node_id,
                        direction,
                        channel,
                    },
                );
            }
            ObjectType::Link => {
                let Some(output_node) = props
                    .get("link.output.node")
                    .and_then(|value| value.parse().ok())
                else {
                    return;
                };
                let Some(input_node) = props
                    .get("link.input.node")
                    .and_then(|value| value.parse().ok())
                else {
                    return;
                };
                let Some(output_port) = props
                    .get("link.output.port")
                    .and_then(|value| value.parse().ok())
                else {
                    return;
                };
                let Some(input_port) = props
                    .get("link.input.port")
                    .and_then(|value| value.parse().ok())
                else {
                    return;
                };
                self.links.insert(
                    object.id,
                    ExistingLink {
                        id: object.id,
                        output_node,
                        output_port,
                        input_node,
                        input_port,
                    },
                );
            }
            _ => return,
        }

        self.reconcile();
    }

    fn remove_global(&mut self, id: u32) {
        if self.links.remove(&id).is_some() {
            self.player_links
                .retain(|_, link| link.upcast_ref().id() != id);
            self.output_links
                .retain(|_, link| link.upcast_ref().id() != id);
        }

        if self
            .metadata
            .as_ref()
            .is_some_and(|binding| binding.id == id)
        {
            self.metadata = None;
            self.filter_is_default = false;
        }

        let removed_filter = self
            .nodes
            .get(&id)
            .is_some_and(|node| node.name == FILTER_NODE_NAME);
        if self.nodes.remove(&id).is_some() {
            self.player_links
                .retain(|(player_id, _), _| *player_id != id);
            if removed_filter || self.linked_sink == Some(id) {
                self.clear_output_links();
            }
        }

        if let Some(port) = self.ports.remove(&id) {
            let filter_id = self
                .nodes
                .values()
                .find(|node| node.name == FILTER_NODE_NAME)
                .map(|node| node.id);
            if filter_id == Some(port.node_id) || self.linked_sink == Some(port.node_id) {
                self.clear_output_links();
            } else {
                self.player_links.remove(&(port.node_id, port.channel));
            }
        }

        self.reconcile();
    }

    fn set_default_sink(&mut self, value: Option<&str>) {
        let name = value.and_then(default_sink_name);
        if name.as_deref() == Some(FILTER_NODE_NAME) {
            self.filter_is_default = true;
            self.reconcile();
            return;
        }

        self.filter_is_default = false;
        self.using_fallback_sink = false;
        if name != self.default_sink_name {
            self.default_sink_name = name;
            self.default_sink_value = value.map(str::to_owned);
            self.clear_output_links();
        }
        self.save_target_sink();
        self.reconcile();
    }

    fn set_saved_sink(&mut self, value: Option<&str>) {
        let name = value.and_then(default_sink_name);
        if name.as_deref() == Some(FILTER_NODE_NAME) {
            return;
        }
        self.saved_sink_name = name;
        self.saved_sink_value = value.map(str::to_owned);
        self.reconcile();
    }

    fn reconcile(&mut self) {
        if self.shutting_down {
            return;
        }
        self.recover_target_sink();
        self.route_players();
        self.route_output();
    }

    fn recover_target_sink(&mut self) {
        if !self.filter_is_default
            || (self.default_sink_name.is_some() && !self.using_fallback_sink)
        {
            return;
        }

        let saved = self.saved_sink_name.as_ref().and_then(|name| {
            self.nodes
                .values()
                .find(|node| node.name == *name && node.media_class == "Audio/Sink")
        });
        let fallback = || {
            self.nodes
                .values()
                .filter(|node| node.name != FILTER_NODE_NAME && node.media_class == "Audio/Sink")
                .max_by_key(|node| node.priority)
        };
        let using_fallback = saved.is_none();
        let Some(target) = saved.or_else(fallback) else {
            return;
        };
        let target_name = target.name.clone();

        if self.default_sink_name.as_deref() == Some(target_name.as_str()) {
            return;
        }

        let value = if using_fallback {
            format!("{{\"name\":\"{target_name}\"}}")
        } else {
            self.saved_sink_value
                .clone()
                .expect("a saved sink name always has a value")
        };
        self.default_sink_name = Some(target_name);
        self.default_sink_value = Some(value);
        self.using_fallback_sink = using_fallback;

        self.clear_output_links();
        self.restore_default_to_earlier_sink();
    }

    fn save_target_sink(&mut self) {
        let (Some(binding), Some(value)) = (&self.metadata, &self.default_sink_value) else {
            return;
        };
        if self.saved_sink_value.as_deref() == Some(value) {
            return;
        }
        self.saved_sink_name = self.default_sink_name.clone();
        self.saved_sink_value = Some(value.clone());
        // Call PipeWire to set default
        binding
            ._metadata
            .set_property(0, TARGET_SINK_KEY, Some(DEFAULT_SINK_TYPE), Some(value));
    }

    fn restore_default_to_earlier_sink(&mut self) {
        if !self.filter_is_default {
            return;
        }
        let (Some(binding), Some(value)) = (&self.metadata, &self.default_sink_value) else {
            return;
        };
        binding
            ._metadata
            .set_property(0, DEFAULT_SINK_KEY, Some(DEFAULT_SINK_TYPE), Some(value));
    }

    fn prepare_shutdown(&mut self) {
        // Coallesce multiple signals here
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;

        // Some requests and operations here can be rejected
        // TODO: Figure out a way for better error handling and
        // re-attempts at requests.

        // TODO: Defensive code here, uncomment if for some reason someone's code breaks
        // self.redirect_to_earlier_sink();
        // self.disconnect_sources_from_self();
        // self.disconnect_self_from_sink();
        self.restore_default_to_earlier_sink();
    }

    fn finish_shutdown(&mut self) {
        if self.shutdown_links_created {
            return;
        }
        self.shutdown_links_created = true;

        let sink_id = self.linked_sink.or_else(|| {
            self.default_sink_name.as_ref().and_then(|name| {
                self.nodes
                    .values()
                    .find(|node| node.name == *name && node.media_class == "Audio/Sink")
                    .map(|node| node.id)
            })
        });
        let Some(sink_id) = sink_id else {
            eprintln!("Loom could not restore player links: the target output is unavailable");
            return;
        };

        let player_ids: Vec<u32> = self
            .nodes
            .values()
            .filter(|node| node.is_player())
            .map(|node| node.id)
            .collect();

        let player_count = player_ids.len();
        for player_id in player_ids {
            for channel in [Channel::Left, Channel::Right] {
                let output = find_port(&self.ports, player_id, PortDirection::Output, channel)
                    .or_else(|| {
                        find_port(&self.ports, player_id, PortDirection::Output, Channel::Mono)
                    });
                let input = find_port(&self.ports, sink_id, PortDirection::Input, channel);
                let (Some(output), Some(input)) = (output, input) else {
                    continue;
                };
                if self
                    .links
                    .values()
                    .any(|link| link.output_port == output.id && link.input_port == input.id)
                {
                    continue;
                }

                match create_link(&self.core, output, input, false, true) {
                    Ok(link) => self.shutdown_links.push(link),
                    Err(error) => {
                        eprintln!("Loom could not restore a direct player link: {error}")
                    }
                }
            }
        }
        if player_count > 0 && self.shutdown_links.is_empty() {
            eprintln!("Loom could not restore any direct player links");
        }
    }

    fn route_players(&mut self) {
        // Validate using derived metadata than cache
        let Some(filter_id) = self
            .nodes
            .values()
            .find(|node| node.name == FILTER_NODE_NAME)
            .map(|node| node.id)
        else {
            return;
        };

        let player_ids: Vec<u32> = self
            .nodes
            .values()
            .filter(|node| node.is_player())
            .map(|node| node.id)
            .collect();

        for player_id in &player_ids {
            for channel in [Channel::Left, Channel::Right] {
                let key = (*player_id, channel);
                let output = find_port(&self.ports, *player_id, PortDirection::Output, channel)
                    .or_else(|| {
                        find_port(
                            &self.ports,
                            *player_id,
                            PortDirection::Output,
                            Channel::Mono,
                        )
                    });
                let input = find_port(&self.ports, filter_id, PortDirection::Input, channel);
                let (Some(output), Some(input)) = (output, input) else {
                    continue;
                };

                if self.player_links.contains_key(&key)
                    || self
                        .links
                        .values()
                        .any(|link| link.output_port == output.id && link.input_port == input.id)
                {
                    continue;
                }

                match create_link(&self.core, output, input, false, false) {
                    Ok(link) => {
                        self.player_links.insert(key, link);
                    }
                    Err(error) => {
                        eprintln!("Loom could not connect an audio player: {error}")
                    }
                }
            }
        }

        // The replacement links are established first so the stream always has
        // a live destination while its direct sink links are removed.
        let bypass_links: Vec<u32> = self
            .links
            .values()
            .filter(|link| {
                player_ids.contains(&link.output_node)
                    && self
                        .links
                        .values()
                        .filter(|candidate| {
                            candidate.output_node == link.output_node
                                && candidate.input_node == filter_id
                        })
                        .count()
                        >= 2
                    && link.input_node != filter_id
                    && self
                        .nodes
                        .get(&link.input_node)
                        .is_some_and(|node| node.media_class == "Audio/Sink")
            })
            .map(|link| link.id)
            .collect();
        for link_id in bypass_links {
            let _ = self.registry.destroy_global(link_id);
            self.links.remove(&link_id);
        }
    }

    fn route_output(&mut self) {
        let Some(filter_id) = self
            .nodes
            .values()
            .find(|node| node.name == FILTER_NODE_NAME)
            .map(|node| node.id)
        else {
            return;
        };
        let Some(sink_id) = self.default_sink_name.as_ref().and_then(|name| {
            self.nodes
                .values()
                .find(|node| node.name == *name && node.media_class == "Audio/Sink")
                .map(|node| node.id)
        }) else {
            return;
        };

        if self.linked_sink != Some(sink_id) {
            self.clear_output_links();
            self.linked_sink = Some(sink_id);
        }

        for channel in [Channel::Left, Channel::Right] {
            if self.output_links.contains_key(&channel) {
                continue;
            }
            let output = find_port(&self.ports, filter_id, PortDirection::Output, channel);
            let input = find_port(&self.ports, sink_id, PortDirection::Input, channel);
            let (Some(output), Some(input)) = (output, input) else {
                continue;
            };

            match create_link(&self.core, output, input, true, false) {
                Ok(link) => {
                    self.output_links.insert(channel, link);
                }
                Err(error) => {
                    eprintln!("Loom could not connect to the current audio output: {error}")
                }
            }
        }
    }

    fn clear_output_links(&mut self) {
        self.output_links.clear();
        self.linked_sink = None;
    }
}

fn find_port(
    ports: &HashMap<u32, Port>,
    node_id: u32,
    direction: PortDirection,
    channel: Channel,
) -> Option<Port> {
    ports
        .values()
        .find(|port| {
            port.node_id == node_id && port.direction == direction && port.channel == channel
        })
        .copied()
}

fn create_link(
    core: &CoreRc,
    output: Port,
    input: Port,
    passive: bool,
    linger: bool,
) -> Result<Link, pw::Error> {
    let output_node = output.node_id.to_string();
    let output_port = output.id.to_string();
    let input_node = input.node_id.to_string();
    let input_port = input.id.to_string();
    let passive = passive.to_string();
    let linger = if linger { "1" } else { "0" };

    core.create_object(
        "link-factory",
        &properties! {
            "link.output.node" => output_node,
            "link.output.port" => output_port,
            "link.input.node" => input_node,
            "link.input.port" => input_port,
            *pw::keys::LINK_PASSIVE => passive,
            *pw::keys::OBJECT_LINGER => linger,
        },
    )
}

fn default_sink_name(value: &str) -> Option<String> {
    let marker = "\"name\"";
    let after_key = value.split_once(marker)?.1;
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let quoted = after_colon.strip_prefix('"')?;
    let end = quoted.find('"')?;
    Some(quoted[..end].to_owned())
}

fn is_default_metadata(object: &GlobalObject<&pw::spa::utils::dict::DictRef>) -> bool {
    object.type_ == ObjectType::Metadata
        && object.props.and_then(|props| props.get("metadata.name")) == Some(DEFAULT_METADATA_NAME)
}

fn bind_default_metadata(
    registry: &RegistryRc,
    object: &GlobalObject<&pw::spa::utils::dict::DictRef>,
    state: &Rc<RefCell<State>>,
) -> Result<MetadataBinding, pw::Error> {
    let metadata: Metadata = registry.bind(object)?;
    let weak_state = Rc::downgrade(state);
    let listener = metadata
        .add_listener_local()
        .property(move |_subject, key, _type, value| {
            if let Some(state) = weak_state.upgrade() {
                match key {
                    Some(DEFAULT_SINK_KEY) => state.borrow_mut().set_default_sink(value),
                    Some(TARGET_SINK_KEY) => state.borrow_mut().set_saved_sink(value),
                    _ => {}
                }
            }
            0
        })
        .register();

    Ok(MetadataBinding {
        id: object.id,
        _listener: listener,
        _metadata: metadata,
    })
}
