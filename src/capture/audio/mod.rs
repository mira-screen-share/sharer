use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::Encoder;
use anyhow::anyhow;
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::output::OutputSink;
use crate::Result;

pub struct AudioCapture {
    encoder: AudioEncoder,
    sender: Sender<Bytes>,
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

#[allow(dead_code)]
impl AudioCapture {
    fn write_input_data<T>(&mut self, input: &[T])
    where
        T: cpal::Sample,
    {
        let sample_size = self.encoder.samples_per_frame().unwrap();

        let mut frame = AudioFrameMut::silence(
            self.encoder.codec_parameters().channel_layout(),
            self.encoder.codec_parameters().sample_format(),
            self.encoder.codec_parameters().sample_rate(),
            sample_size,
        );

        let plane = &mut frame.planes_mut()[0];
        let data = plane.data_mut();
        let samples: &mut [T] = unsafe {
            std::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut T,
                data.len() / std::mem::size_of::<T>(),
            )
        };

        // copy from input to ffmpeg buffer
        samples[..input.len()].copy_from_slice(input);

        self.encoder.push(frame.freeze()).unwrap();

        let mut ret = Vec::new();

        while let Some(packet) = self.encoder.take().unwrap() {
            ret.extend(packet.data());
        }

        self.sender.send(Bytes::from(ret)).unwrap();
    }

    pub fn capture(
        output: Arc<Mutex<dyn OutputSink + Send>>,
        cancel: CancellationToken,
    ) -> Result<()> {
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
            .channel_layout(ChannelLayout::from_channels(2).unwrap())
            .sample_format(convert_sample_format(config.sample_format()))
            .set_option("frame_duration", "10")
            .build()
            .unwrap();

        info!("Begin recording audio");

        let (sender, receiver) = std::sync::mpsc::channel();

        tokio::spawn(async move {
            loop {
                let data = receiver.recv().map_or_else(|_| None, Some);
                if data.is_none() {
                    info!("Audio capture stopped");
                    break;
                }
                let mut output = output.lock().await;
                output
                    .write_audio(data.unwrap(), Duration::from_millis(10))
                    .await
                    .unwrap();
            }
        });

        let handle = tokio::runtime::Handle::current();
        thread::spawn(move || {
            let mut capturer = AudioCapture { encoder, sender };
            let err_fn = |err| error!("an error occurred on audio stream: {}", err);

            let stream = match config.sample_format() {
                SampleFormat::I8 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i8>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::I16 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i16>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::I32 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i32>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<f32>(data),
                    err_fn,
                    None,
                )?,
                _ => {
                    return Err(anyhow!("unsupported sample format"));
                }
            };

            stream.play()?;

            tokio::task::block_in_place(move || {
                handle.block_on(async move { cancel.cancelled().await });
            });
            stream.pause()?;
            Ok(())
        });
        Ok(())
    }
}
