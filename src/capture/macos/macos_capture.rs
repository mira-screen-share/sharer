use std::sync::Arc;
use std::time::Duration;

use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::Encoder;
use async_trait::async_trait;
use bytes::Bytes;
use tokio::select;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::capture::display::DisplaySelector;
use crate::capture::macos::pcm_buffer::PCMBuffer;
use crate::capture::macos::screen_recorder::ScreenRecorder;
use crate::capture::{DisplayInfo, ScreenCaptureImpl, YUVFrame};
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};

pub struct MacOSCapture {
    config: Config,
    recorder: ScreenRecorder,
}

#[async_trait]
impl ScreenCapture for MacOSCapture {
    fn new(config: Config) -> Result<ScreenCaptureImpl> {
        // TODO hot-reload config

        let mut recorder = ScreenRecorder::new();
        recorder.set_max_fps(config.max_fps as u8);
        recorder.monitor_available_content();

        Ok(Self { config, recorder })
    }

    fn display(&self) -> &dyn DisplayInfo {
        &self.recorder
    }

    async fn start_capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: Arc<Mutex<impl OutputSink + Send + ?Sized>>,
        mut profiler: PerformanceProfiler,
        shutdown_token: CancellationToken,
    ) -> Result<()> {
        let (video_tx, mut video_rx) = tokio::sync::mpsc::channel::<YUVFrame>(1);
        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<PCMBuffer>(1);

        let output_audio = output.clone();
        let cancel_audio = shutdown_token.clone();
        tokio::spawn(async move {
            let mut audio_encoder_opt = None;
            loop {
                select! {
                    Some(pcm_buffer) = audio_rx.recv() => {
                        let audio_encoder = audio_encoder_opt.get_or_insert_with(|| {
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
                        output_audio
                            .lock()
                            .await
                            .write_audio(
                                Bytes::from(ret),
                                Duration::from_millis(pcm_buffer.frame_duration as u64),
                            )
                            .await
                            .unwrap();
                    }
                    _ = cancel_audio.cancelled() => {
                        break;
                    }
                }
            }
        });

        let cancel_video = shutdown_token.clone();
        let max_fps = self.config.max_fps;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis((1000 / max_fps) as u64));
            loop {
                select! {
                    Some(frame) = video_rx.recv() => {
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
                    _ = cancel_video.cancelled() => {
                        break;
                    }
                }
            }
        });

        self.recorder.start(video_tx, audio_tx);

        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        self.recorder.stop();
        Ok(())
    }
}

impl DisplaySelector for MacOSCapture {
    type Display = <ScreenRecorder as DisplaySelector>::Display;

    fn available_displays(&mut self) -> Result<Vec<Self::Display>> {
        self.recorder.available_displays()
    }

    fn select_display(&mut self, display: &Self::Display) -> Result<()> {
        self.recorder.select_display(display)
    }

    fn selected_display(&self) -> Result<Option<Self::Display>> {
        self.recorder.selected_display()
    }
}
