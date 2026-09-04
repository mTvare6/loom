use saq_dsp::ŚaqEngine;
use pipewire as pw;
use pw::{
    filter::{Filter, FilterBox, FilterFlags, FilterPort, FilterPortFlags},
    properties::properties,
    spa::utils::Direction,
};
use std::sync::Arc;

pub trait AudioControls: Send + Sync + 'static {
    fn volume(&self) -> f32;
    fn spatial_mix(&self) -> f32;
    fn is_bypassed(&self) -> bool;
}

struct Processor<'f> {
    ports: [FilterPort<'f>; 4],
    engine: Box<ŚaqEngine>,
    state: Arc<dyn AudioControls>,
    spatial: f32,
}

pub fn run_audio_engine(state: Arc<dyn AudioControls>) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let filter = FilterBox::new(
        &core,
        "saq",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Filter",
            *pw::keys::MEDIA_ROLE => "DSP",
            *pw::keys::MEDIA_CLASS => "Audio/Sink",
            *pw::keys::NODE_NAME => "saq_virtual_sink",
            *pw::keys::NODE_DESCRIPTION => "Śaq",
        },
    )?;

    let processor = Processor {
        ports: [
            add_port(&filter, Direction::Input, "input_FL", "FL")?,
            add_port(&filter, Direction::Input, "input_FR", "FR")?,
            add_port(&filter, Direction::Output, "output_FL", "FL")?,
            add_port(&filter, Direction::Output, "output_FR", "FR")?,
        ],
        engine: ŚaqEngine::new(48000.0),
        state,
        spatial: -999.0,
    };

    let _listener = filter
        .add_local_listener_with_user_data(processor)
        .process(|_, processor, position| {
            let Ok(n_samples) = position.clock.duration.try_into() else {
                return;
            };
            let [input_left, input_right, output_left, output_right] = &mut processor.ports;
            let buffers = unsafe {
                (
                    input_left.dsp_buffer::<f32>(n_samples),
                    input_right.dsp_buffer::<f32>(n_samples),
                    output_left.dsp_buffer::<f32>(n_samples),
                    output_right.dsp_buffer::<f32>(n_samples),
                )
            };
            let (Some(input_left), Some(input_right), Some(output_left), Some(output_right)) =
                buffers
            else {
                return;
            };

            let volume = processor.state.volume();
            let spatial = processor.state.spatial_mix();
            let bypassed = processor.state.is_bypassed();

            if !bypassed && (spatial - processor.spatial).abs() > 0.01 {
                processor.engine.update_params(spatial);
                processor.spatial = spatial;
            }

            for i in 0..n_samples as usize {
                let (left, right) = if bypassed {
                    (input_left[i], input_right[i])
                } else {
                    processor.engine.process(input_left[i], input_right[i])
                };
                output_left[i] = left * volume;
                output_right[i] = right * volume;
            }
        })
        .register()?;

    filter.connect(FilterFlags::RT_PROCESS, &mut [])?;

    mainloop.run();
    Ok(())
}

fn add_port<'f>(
    filter: &'f Filter,
    direction: Direction,
    name: &str,
    channel: &str,
) -> Result<FilterPort<'f>, pw::Error> {
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
