use saq_dsp::{Mode, SpatialEngine, SurroundEngine};
use pipewire as pw;
use pw::{
    channel,
    filter::{Filter, FilterFlags, FilterPortFlags, FilterRc, PortHandle},
    properties::properties,
    spa::utils::Direction,
};
use std::{rc::Rc, sync::Arc};

mod routing;

pub trait AudioControls: Send + Sync + 'static {
    fn volume(&self) -> f32;
    fn mode(&self) -> Mode;
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
    spatial: Box<SpatialEngine>,
    surround: Box<SurroundEngine>,
    state: Arc<dyn AudioControls>,
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
        "saq",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Filter",
            *pw::keys::MEDIA_ROLE => "DSP",
            *pw::keys::MEDIA_CLASS => "Audio/Sink",
            *pw::keys::NODE_NAME => "saq_virtual_sink",
            *pw::keys::NODE_DESCRIPTION => "Śaq",
            *pw::keys::NODE_VIRTUAL => "true",
        },
    )?;

    let mut spatial = SpatialEngine::new(48000.0);
    spatial.update_params(1.0);

    let mut processor = Processor {
        filter: filter.clone(),
        ports: [
            add_port(&filter, Direction::Input, "input_FL", "FL")?,
            add_port(&filter, Direction::Input, "input_FR", "FR")?,
            add_port(&filter, Direction::Output, "output_FL", "FL")?,
            add_port(&filter, Direction::Output, "output_FR", "FR")?,
        ],
        spatial,
        surround: SurroundEngine::new(),
        state,
    };

    let _listener = filter
        .add_local_listener()
        .process(move |position| {
            let Ok(n_samples) = position.clock.duration.try_into() else {
                return;
            };
            let [input_left, input_right, output_left, output_right] = &mut processor.ports;
            let buffers = unsafe {
                (
                    processor.filter.dsp_buffer::<f32>(input_left, n_samples),
                    processor.filter.dsp_buffer::<f32>(input_right, n_samples),
                    processor.filter.dsp_buffer::<f32>(output_left, n_samples),
                    processor.filter.dsp_buffer::<f32>(output_right, n_samples),
                )
            };
            let (Some(input_left), Some(input_right), Some(output_left), Some(output_right)) =
                buffers
            else {
                return;
            };

            let volume = processor.state.volume();
            let mode = processor.state.mode();

            for i in 0..n_samples as usize {
                let (left, right) = match mode {
                    Mode::Off => (input_left[i], input_right[i]),
                    Mode::Spatial => processor.spatial.process(input_left[i], input_right[i]),
                    Mode::SurroundSound => processor.surround.process(input_left[i], input_right[i]),
                };
                output_left[i] = left * volume;
                output_right[i] = right * volume;
            }
        })
        .register()?;

    filter.connect(FilterFlags::RT_PROCESS, &mut [])?;

    let routing = Rc::new(routing::Routing::new(&core)?);
    let mainloop_weak = mainloop.downgrade();
    let routing_weak = Rc::downgrade(&routing);
    let _shutdown = shutdown_rx.0.attach(mainloop.loop_(), move |_| {
        if let Some(routing) = routing_weak.upgrade() {
            routing.restore_default_sink();
        }
        if let Some(mainloop) = mainloop_weak.upgrade() {
            mainloop.quit();
        }
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
