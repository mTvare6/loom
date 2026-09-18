// SPDX-License-Identifier: MPL-2.0

use loom_dsp::{
    AmbienceEngine, FidelityEngine, Mode, NightEngine, PitchEngine, SpatialFilterEngine,
    SpatialStereoEngine, SpatialSurroundEngine, SurroundEngine,
};
use pipewire as pw;
use pw::{
    channel,
    filter::{Filter, FilterFlags, FilterPortFlags, FilterRc, PortHandle},
    properties::properties,
    spa::utils::Direction,
};

use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

mod routing;

// FIXME: Track add_buffer events and use that limit instead
// of relying on un-exporting MAX_BUFFER limit from source
// https://gitlab.freedesktop.org/pipewire/pipewire/-/raw/1.6.8/src/pipewire/filter.c#L29
const MAX_PIPEWIRE_PORT_BUFFERS: u8 = 64;
const SHUTDOWN_LINK_SETTLE_TIME: Duration = Duration::from_millis(50);

pub trait AudioControls: Send + Sync + 'static {
    fn volume(&self) -> f32;
    fn mode(&self) -> Mode;
    fn pitch_enabled(&self) -> bool;
    fn pitch_semitones(&self) -> f32;
    fn subwoofer(&self) -> f32;
}

pub struct ShutdownTransmitter(channel::Sender<()>);
pub struct ShutdownReceiver(channel::Receiver<()>);

pub fn shutdown_channel() -> (ShutdownTransmitter, ShutdownReceiver) {
    let (tx, rx) = channel::channel();
    (ShutdownTransmitter(tx), ShutdownReceiver(rx))
}

impl ShutdownTransmitter {
    pub fn shutdown(&self) {
        let _ = self.0.send(());
    }
}

struct Processor {
    filter: FilterRc,
    ports: [PortHandle; 4],
    spatial_filter: Box<SpatialFilterEngine>,
    spatial_stereo: Box<SpatialStereoEngine>,
    spatial_surround: Box<SpatialSurroundEngine>,
    surround: Box<SurroundEngine>,
    ambience: Box<AmbienceEngine>,
    fidelity: Box<FidelityEngine>,
    night: Box<NightEngine>,
    pitch: Box<PitchEngine>,
    pitch_was_enabled: bool,
    input_already_disconnected: bool,
    output_buffer_resets_left: u8,
    active_mode: Mode,
    state: Arc<dyn AudioControls>,
}

impl Processor {
    fn reset_all(&mut self) {
        self.spatial_filter.reset();
        self.spatial_stereo.reset();
        self.spatial_surround.reset();
        self.surround.reset();
        self.ambience.reset();
        self.fidelity.reset();
        self.night.reset();
        self.pitch.reset();
    }

    fn switch_mode(&mut self, mode: Mode) {
        if mode == self.active_mode {
            return;
        }

        match self.active_mode {
            Mode::Off => {}
            Mode::SpatialFilter => self.spatial_filter.reset(),
            Mode::SpatialStereo => self.spatial_stereo.reset(),
            Mode::SpatialSurround => self.spatial_surround.reset(),
            Mode::Surround3d => self.surround.reset(),
            Mode::Ambience => self.ambience.reset(),
            Mode::Fidelity => self.fidelity.reset(),
            Mode::Night => self.night.reset(),
        }
        self.active_mode = mode;
    }
}

pub fn run_audio_engine(
    state: Arc<dyn AudioControls>,
    shutdown_rx: ShutdownReceiver,
) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let filter = FilterRc::new(
        core.clone(),
        "loom",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Filter",
            *pw::keys::MEDIA_ROLE => "DSP",
            *pw::keys::MEDIA_CLASS => "Audio/Sink",
            *pw::keys::NODE_NAME => "loom_virtual_sink",
            *pw::keys::NODE_DESCRIPTION => "Loom",
            *pw::keys::NODE_VIRTUAL => "true",
        },
    )?;

    // TODO: Generalize to 96k or 44.1k if input advertises that.
    let mut spatial_filter = SpatialFilterEngine::new(48000.0);
    // TODO: Add controls and parametrise every filter.
    spatial_filter.update_params(1.0);

    let mut processor = Processor {
        filter: filter.clone(),
        ports: [
            add_port(&filter, Direction::Input, "input_FL", "FL")?,
            add_port(&filter, Direction::Input, "input_FR", "FR")?,
            add_port(&filter, Direction::Output, "output_FL", "FL")?,
            add_port(&filter, Direction::Output, "output_FR", "FR")?,
        ],
        spatial_filter,
        spatial_stereo: SpatialStereoEngine::new(),
        spatial_surround: SpatialSurroundEngine::new(),
        surround: SurroundEngine::new(),
        ambience: AmbienceEngine::new(),
        fidelity: FidelityEngine::new(),
        night: NightEngine::new(),
        pitch: PitchEngine::new(),
        pitch_was_enabled: false,
        input_already_disconnected: true,
        output_buffer_resets_left: 0,
        active_mode: Mode::Off,
        state,
    };

    let _listener = filter
        .add_local_listener()
        .process(move |position| {
            let Ok(n_samples) = position.clock.duration.try_into() else {
                return;
            };
            let volume = processor.state.volume();
            let mode = processor.state.mode();
            let subwoofer = processor.state.subwoofer();
            processor.switch_mode(mode);

            let [input_left, input_right, output_left, output_right] = &mut processor.ports;
            let buffers = unsafe {
                (
                    processor.filter.dsp_buffer::<f32>(input_left, n_samples),
                    processor.filter.dsp_buffer::<f32>(input_right, n_samples),
                    processor.filter.dsp_buffer::<f32>(output_left, n_samples),
                    processor.filter.dsp_buffer::<f32>(output_right, n_samples),
                )
            };
            let (input_left, input_right, Some(output_left), Some(output_right)) = buffers else {
                return;
            };
            let (Some(input_left), Some(input_right)) = (input_left, input_right) else {
                // No input left anymore
                let input_just_disconnected = !processor.input_already_disconnected;
                if input_just_disconnected {
                    processor.input_already_disconnected = true;
                    processor.output_buffer_resets_left = MAX_PIPEWIRE_PORT_BUFFERS;
                }
                if processor.output_buffer_resets_left > 0 {
                    // At 8,192 bytes per callback and 48k per second
                    // 384kBps if spent zeroing. They are coallesed to save bandwidth
                    output_left.fill(0.0);
                    output_right.fill(0.0);
                    processor.output_buffer_resets_left -= 1;
                }
                if input_just_disconnected {
                    processor.reset_all();
                }
                return;
            };
            processor.input_already_disconnected = false;
            processor.output_buffer_resets_left = 0;

            for i in 0..n_samples as usize {
                let (left, right) = match mode {
                    Mode::Off => (input_left[i], input_right[i]),
                    Mode::SpatialFilter => processor
                        .spatial_filter
                        .process(input_left[i], input_right[i]),
                    Mode::SpatialStereo => processor
                        .spatial_stereo
                        .process(input_left[i], input_right[i]),
                    Mode::SpatialSurround => processor
                        .spatial_surround
                        .process(input_left[i], input_right[i]),
                    Mode::Surround3d => {
                        processor
                            .surround
                            .process(input_left[i], input_right[i], subwoofer)
                    }
                    Mode::Ambience => processor.ambience.process(input_left[i], input_right[i]),
                    Mode::Fidelity => processor.fidelity.process(input_left[i], input_right[i]),
                    Mode::Night => processor.night.process(input_left[i], input_right[i]),
                };
                output_left[i] = left * volume;
                output_right[i] = right * volume;
            }

            let pitch_enabled = processor.state.pitch_enabled();
            if pitch_enabled {
                if !processor.pitch_was_enabled {
                    processor.pitch.reset();
                }
                processor.pitch.process(
                    output_left,
                    output_right,
                    processor.state.pitch_semitones(),
                );
            }
            processor.pitch_was_enabled = pitch_enabled;
        })
        .register()?;

    filter.connect(FilterFlags::RT_PROCESS, &mut [])?;

    let routing = Rc::new(routing::Routing::new(&core)?);

    let requested_pending_seq = Rc::new(Cell::new(None));

    // Mostly defensive here given we have single sync
    // Can use Cell<bool> if finalized
    let completed_pending_seq = Rc::new(Cell::new(None));

    let (finalize_tx, finalize_rx) = channel::channel();

    let request_shutdown = Rc::new({
        let mainloop_weak = mainloop.downgrade();
        let routing_weak = Rc::downgrade(&routing);

        let disconnect_sync_pending = requested_pending_seq.clone();
        let relink_sync_pending = completed_pending_seq.clone();

        move || {
            if disconnect_sync_pending.get().is_some() || relink_sync_pending.get().is_some() {
                return;
            }
            let Some(routing) = routing_weak.upgrade() else {
                if let Some(mainloop) = mainloop_weak.upgrade() {
                    mainloop.quit();
                }
                return;
            };
            match routing.begin_shutdown() {
                Ok(sequence) => disconnect_sync_pending.set(Some(sequence)),
                Err(_) => {
                    if let Some(mainloop) = mainloop_weak.upgrade() {
                        mainloop.quit();
                    }
                }
            }
        }
    });

    let _core_listener = core
        .add_listener_local()
        .done({
            let routing_weak = Rc::downgrade(&routing);

            let bending_shutdown_seq_guard = requested_pending_seq.clone();
            let final_shutdown_seq_guard = completed_pending_seq.clone();

            let finalize_tx = finalize_tx.clone();

            move |id, sequence| {
                if id != pw::core::PW_ID_CORE {
                    return;
                }

                // The final two-state solution
                if bending_shutdown_seq_guard
                    .get()
                    .is_some_and(|pending| pending == sequence)
                {
                    bending_shutdown_seq_guard.set(None);

                    let next = routing_weak
                        .upgrade()
                        .ok_or(pw::Error::CreationFailed)
                        .and_then(|routing| routing.finish_shutdown());

                    if let Ok(sequence) = next {
                        final_shutdown_seq_guard.set(Some(sequence));
                        return;
                    }
                } else if final_shutdown_seq_guard
                    .get()
                    .is_some_and(|pending| pending == sequence)
                {
                    final_shutdown_seq_guard.set(None);
                } else {
                    return;
                }

                std::thread::spawn({
                    let finalize_tx = finalize_tx.clone();
                    move || {
                        std::thread::sleep(SHUTDOWN_LINK_SETTLE_TIME);
                        let _ = finalize_tx.send(());
                    }
                });
            }
        })
        .register();

    let _finalize = finalize_rx.attach(mainloop.loop_(), {
        let mainloop_weak = mainloop.downgrade();
        let routing_weak = Rc::downgrade(&routing);

        move |_| {
            if let Some(routing) = routing_weak.upgrade() {
                routing.release_shutdown_links();
            }

            if let Some(mainloop) = mainloop_weak.upgrade() {
                mainloop.quit();
            }
        }
    });

    let _shutdown = shutdown_rx.0.attach(mainloop.loop_(), {
        let request_shutdown = request_shutdown.clone();

        move |_| request_shutdown()
    });
    mainloop.run();
    Ok(())
}

fn add_port(
    filter: &Filter,
    direction: Direction,
    name: &str,
    channel: &str,
) -> Result<PortHandle, pw::Error> {
    filter.add_port(
        direction,
        FilterPortFlags::MAP_BUFFERS,
        properties! {
            *pw::keys::FORMAT_DSP => "32 bit float mono audio",
            *pw::keys::PORT_NAME => name,
            *pw::keys::AUDIO_CHANNEL => channel,
        },
        &mut [],
    )
}
