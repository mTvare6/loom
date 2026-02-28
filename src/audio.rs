use crate::dsp::Biquad;
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

    let ring_buffer = HeapRb::<f32>::new(16384);
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
        .process(move |stream, _user_data| match stream.dequeue_buffer() {
            None => return,
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                let valid_bytes = datas[0].chunk().size() as usize;

                if let Some(in_slice) = datas[0].data() {
                    let valid_slice = &in_slice[..valid_bytes];
                    for in_bytes in valid_slice.chunks_exact(4) {
                        let sample = f32::from_le_bytes(in_bytes.try_into().unwrap());
                        let _ = producer.try_push(sample);
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

    let mut dsp_l = Biquad::new();
    let mut dsp_r = Biquad::new();

    let mut last_bass = -999.0_f32;

    let _playback_listener = playback_stream
        .add_local_listener::<i32>()
        .process(move |stream, _user_data| match stream.dequeue_buffer() {
            None => return,
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                let mut processed_bytes = 0;

                let live_volume = shared_state.volume();
                let live_bass = shared_state.bass_db();

                if (live_bass - last_bass).abs() > 0.1 {
                    dsp_l.set_low_shelf(48000.0, 120.0, live_bass);
                    dsp_r.set_low_shelf(48000.0, 120.0, live_bass);
                    last_bass = live_bass;
                }

                if let Some(out_slice) = datas[0].data() {
                    let available_samples = consumer.occupied_len();
                    let bytes_to_write = (available_samples * 4).min(out_slice.len());
                    let valid_out_slice = &mut out_slice[..bytes_to_write];

                    let mut is_left_channel = true;

                    for out_bytes in valid_out_slice.chunks_exact_mut(4) {
                        let mut sample = consumer.try_pop().unwrap_or(0.0);

                        if is_left_channel {
                            sample = dsp_l.process(sample);
                        } else {
                            sample = dsp_r.process(sample);
                        }
                        is_left_channel = !is_left_channel;

                        sample *= live_volume;

                        out_bytes.copy_from_slice(&sample.to_le_bytes());
                    }
                    processed_bytes = bytes_to_write;
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

    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
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
