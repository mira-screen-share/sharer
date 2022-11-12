use crate::result::Result;
use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};

use crate::config::EncoderConfig;
use crate::encoder::frame_pool::FramePool;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32, encoder_config: &EncoderConfig) -> Self {
        let time_base = TimeBase::new(1, 10_000);

        let pixel_format = video::frame::get_pixel_format(if encoder_config.yuv_input {
            "yuv420p"
        } else {
            "bgra"
        });

        let mut encoder = VideoEncoder::builder(&encoder_config.encoder)
            .unwrap()
            .pixel_format(pixel_format)
            .width(w as _)
            .height(h as _)
            .time_base(time_base);

        for option in &encoder_config.options {
            encoder = encoder.set_option(&option.0, option.1);
        }

        let encoder = encoder.build().unwrap();

        Self {
            encoder,
            frame_pool: FramePool::new(w, h, time_base, pixel_format),
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn encode(&mut self, input_planes: &[&[u8]], frame_time: i64) -> Result<Vec<u8>> {
        let mut frame = self.frame_pool.take();
        let time_base = frame.time_base();
        frame = frame.with_pts(Timestamp::new(
            (frame_time as f64 / 1000.) as i64,
            time_base,
        ));

        input_planes.iter().enumerate().for_each(|(i, plane)| {
            frame
                .planes_mut()
                .iter_mut()
                .nth(i)
                .unwrap()
                .data_mut()
                .copy_from_slice(plane);
        });

        let frame = frame.freeze();
        self.encoder.push(frame.clone())?;
        self.frame_pool.put(frame);
        let mut ret = Vec::new();
        while let Some(a) = self.encoder.take()? {
            ret.extend(a.data());
        }
        Ok(ret)
    }
}
