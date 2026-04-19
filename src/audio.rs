use crate::dsp::LoomEngine;
use crate::state::AudioState;
use pipewire as pw;
use pw::properties::properties;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};
use std::convert::TryInto;
use std::sync::Arc;

pub fn run_audio_engine(shared_state: Arc<AudioState>) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    let ring_buffer = HeapRb::<f32>::new(32768);
    let (mut producer, mut consumer) = ring_buffer.split();


    let capture_props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CLASS => "Audio/Sink",
        *pw::keys::NODE_NAME => "loom_virtual_sink",
        *pw::keys::NODE_DESCRIPTION => "Loom",
    };

    let capture_stream = pw::stream::StreamBox::new(&core, "loom_capture", capture_props)?;

    let _capture_listener = capture_stream
        .add_local_listener::<i32>()
        .process(move |stream, _| match stream.dequeue_buffer() {
            None => return,
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                let valid_bytes = datas[0].chunk().size() as usize;
                if let Some(in_slice) = datas[0].data() {
                    for in_bytes in in_slice[..valid_bytes].chunks_exact(4) {
                        let _ = producer.try_push(f32::from_le_bytes(in_bytes.try_into().unwrap()));
                    }
                }
            }
        })
        .register()?;

    let playback_props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "DSP",
        *pw::keys::NODE_NAME => "loom_hardware_out",
    };

    let playback_stream = pw::stream::StreamBox::new(&core, "loom_playback", playback_props)?;

    let mut engine = LoomEngine::new(48000.0);
    let mut last_spatial = -999.0_f32;

    let _playback_listener = playback_stream
        .add_local_listener::<i32>()
        .process(move |stream, _| match stream.dequeue_buffer() {
            None => return,
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                let live_volume = shared_state.volume();
                let live_spatial = shared_state.spatial_mix();
                let is_bypassed = shared_state.is_bypassed();

                if !is_bypassed && (live_spatial - last_spatial).abs() > 0.01 {
                    engine.update_params(live_spatial);
                    last_spatial = live_spatial;
                }

                let mut processed_bytes = 0;
                if let Some(out_slice) = datas[0].data() {
                    let stereo_frames = (consumer.occupied_len() / 2).min(out_slice.len() / 8);
                    let valid_out_slice = &mut out_slice[..stereo_frames * 8];

                    for frame in valid_out_slice.chunks_exact_mut(8) {
                        let l = consumer.try_pop().unwrap_or(0.0);
                        let r = consumer.try_pop().unwrap_or(0.0);

                        let (out_l, out_r) = if is_bypassed {
                            (l * live_volume, r * live_volume)
                        } else {
                            let (pl, pr) = engine.process(l, r);
                            (pl * live_volume, pr * live_volume)
                        };

                        frame[0..4].copy_from_slice(&out_l.to_le_bytes());
                        frame[4..8].copy_from_slice(&out_r.to_le_bytes());
                    }
                    processed_bytes = stereo_frames * 8;
                }

                let chunk = datas[0].chunk_mut();
                *chunk.size_mut() = processed_bytes as u32;
                *chunk.stride_mut() = 4;
            }
        })
        .register()?;

    let mut audio_info = pw::spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(pw::spa::param::audio::AudioFormat::F32LE);
    audio_info.set_channels(2);

    let mut position = [0; pw::spa::param::audio::MAX_CHANNELS];
    position[0] = pw::spa::sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = pw::spa::sys::SPA_AUDIO_CHANNEL_FR;
    audio_info.set_position(position);

    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();
    let mut params = [pw::spa::pod::Pod::from_bytes(&values).unwrap()];

    let flags = pw::stream::StreamFlags::AUTOCONNECT
        | pw::stream::StreamFlags::MAP_BUFFERS
        | pw::stream::StreamFlags::RT_PROCESS;
    capture_stream.connect(pw::spa::utils::Direction::Input, None, flags, &mut params)?;
    playback_stream.connect(pw::spa::utils::Direction::Output, None, flags, &mut params)?;

    mainloop.run();
    Ok(())
}
