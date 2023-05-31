use crate::output::OutputSink;
use crate::Result;
use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::Encoder;
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AudioCapture {
    encoder: AudioEncoder,
    output: Arc<Mutex<dyn OutputSink + Send>>,
}

fn convert_sample_format(format: SampleFormat) -> ac_ffmpeg::codec::audio::SampleFormat {
    get_sample_format(match format {
        SampleFormat::F32 => "flt",
        SampleFormat::I16 => "s16",
        SampleFormat::I32 => "s32",
        _ => {
            panic!("Unsupported sample format: {:?}", format);
        }
    })
}

impl AudioCapture {
    fn write_input_data<T>(&mut self, input: &[T])
    where
        T: cpal::Sample,
    {
        let mut frame = AudioFrameMut::silence(
            self.encoder.codec_parameters().channel_layout(),
            self.encoder.codec_parameters().sample_format(),
            self.encoder.codec_parameters().sample_rate(),
            input.len() as _,
        );
        info!(
            "data len = {}; input len = {}",
            frame.planes_mut()[0].data_mut().len(),
            input.len()
        );
        //copy_from_slice(input);
        self.encoder.push(frame.freeze()).unwrap();
        let mut ret = Vec::new();
        while let Some(packet) = self.encoder.take().unwrap() {
            ret.extend(packet.data());
        }
        self.output.blocking_lock().write_audio(Bytes::from(ret));
    }

    pub fn capture(output: Arc<Mutex<dyn OutputSink + Send>>) -> Result<Stream> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("Failed to get default output device");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        info!("Audio config: {:?}", config);

        let encoder = AudioEncoder::builder("libopus")
            .unwrap()
            .sample_rate(config.sample_rate().0 as _)
            .channel_layout(ChannelLayout::from_channels(config.channels() as _).unwrap())
            .sample_format(convert_sample_format(config.sample_format()))
            .build()
            .unwrap();

        info!("Begin recording audio");

        let mut capturer = AudioCapture { encoder, output };

        let err_fn = |err| error!("an error occurred on audio stream: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| capturer.write_input_data::<i8>(data),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| capturer.write_input_data::<i16>(data),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| capturer.write_input_data::<i32>(data),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| capturer.write_input_data::<f32>(data),
                err_fn,
                None,
            )?,
            _ => {
                return Err(failure::err_msg("unsupported sample format"));
            }
        };

        stream.play()?;
        Ok(stream)
    }
}
