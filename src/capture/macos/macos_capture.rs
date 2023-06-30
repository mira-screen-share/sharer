use std::sync::Arc;
use std::time::Duration;

use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::Encoder;
use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::Mutex;

use crate::capture::macos::screen_recorder::ScreenRecorder;
use crate::capture::Display;
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};

pub struct MacOSCapture<'a> {
    config: &'a Config,
}

unsafe impl Send for MacOSCapture<'_> {}

pub type GraphicsCaptureItem = Display;

impl<'a> MacOSCapture<'a> {
    pub fn new(_display: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
        // TODO select display
        // TODO hot-update config
        Ok(Self { config })
    }
}

#[async_trait]
impl ScreenCapture for MacOSCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: Arc<Mutex<impl OutputSink + Send + ?Sized>>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {
        let (video_tx, mut video_rx) = tokio::sync::mpsc::channel(1);
        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel(1);

        let mut audio_encoder = None;
        let mut recorder = ScreenRecorder::new();
        recorder.set_max_fps(self.config.max_fps as u8);
        recorder.start(video_tx, audio_tx).await;
        let output_audio_clone = output.clone();

        tokio::spawn(async move {
            while let Some(pcm_buffer) = audio_rx.recv().await {
                let audio_encoder = audio_encoder.get_or_insert_with(|| {
                    AudioEncoder::builder("libopus")
                        .unwrap()
                        .sample_rate(pcm_buffer.sample_rate as _)
                        .channel_layout(
                            ChannelLayout::from_channels(pcm_buffer.channels as u32).unwrap(),
                        )
                        .sample_format(pcm_buffer.sample_format())
                        .set_option("frame_duration", pcm_buffer.frame_duration)
                        .build()
                        .unwrap()
                });
                let mut audio_frame = AudioFrameMut::silence(
                    audio_encoder.codec_parameters().channel_layout(),
                    audio_encoder.codec_parameters().sample_format(),
                    audio_encoder.codec_parameters().sample_rate(),
                    pcm_buffer.sample_size,
                );
                pcm_buffer.write_samples_into(&mut audio_frame);
                audio_encoder.push(audio_frame.freeze()).unwrap();

                let mut ret: Vec<u8> = Vec::new();
                while let Some(packet) = audio_encoder.take().unwrap() {
                    ret.extend(packet.data());
                }
                output_audio_clone
                    .lock()
                    .await
                    .write_audio(
                        Bytes::from(ret),
                        Duration::from_millis(pcm_buffer.frame_duration as u64),
                    )
                    .await
                    .unwrap();
            }
        });

        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / self.config.max_fps) as u64));

        while let Some(frame) = video_rx.recv().await {
            let frame_time = frame.display_time as f64;
            profiler.accept_frame(frame_time as i64);
            profiler.done_preprocessing();
            let encoded = encoder
                .encode(FrameData::NV12(&frame), frame_time as i64)
                .unwrap();
            let encoded_len = encoded.len();
            profiler.done_encoding();
            output.lock().await.write(encoded).await.unwrap();
            profiler.done_processing(encoded_len);
            ticker.tick().await;
        }

        Ok(())
    }
}
