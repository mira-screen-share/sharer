use crate::result::Result;
use ac_ffmpeg::codec::video::{PixelFormat, VideoEncoder, VideoFrame, VideoFrameMut};
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use std::collections::VecDeque;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

struct FramePool {
    frames: VecDeque<VideoFrame>,
    w: u32,
    h: u32,
    time_base: TimeBase,
    pixel_format: PixelFormat,
}

impl FramePool {
    fn new(w: u32, h: u32, time_base: TimeBase, pixel_format: PixelFormat) -> Self {
        Self {
            frames: VecDeque::new(),
            w,
            h,
            time_base,
            pixel_format,
        }
    }

    /// Put a given frame back to the pool after it was used.
    fn put(&mut self, frame: VideoFrame) {
        self.frames.push_back(frame);
    }

    /// Take a writable frame from the pool or allocate a new one if necessary.
    fn take(&mut self) -> VideoFrameMut {
        if let Some(frame) = self.frames.pop_front() {
            match frame.try_into_mut() {
                Ok(frame) => return frame,
                Err(frame) => self.frames.push_front(frame),
            }
        }

        VideoFrameMut::black(self.pixel_format, self.w as _, self.h as _)
            .with_time_base(self.time_base)
    }
}

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32, _fps: u32) -> Self {
        let time_base = TimeBase::new(1, 10_000);

        let pixel_format = video::frame::get_pixel_format("bgra"); // yuv420p

        let encoder = VideoEncoder::builder("h264_nvenc") // libx264
            .unwrap()
            .pixel_format(pixel_format)
            .set_option("profile", "baseline")
            .set_option("preset", "p4")
            .set_option("tune", "ll")
            .set_option("zerolatency", true)
            //.set_option("preset", "ultrafast")
            //.set_option("tune", "zerolatency")
            //.set_option("compression_level", 8)
            .width(w as _)
            .height(h as _)
            .time_base(time_base)
            .build()
            .unwrap();

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
                .copy_from_slice(input_planes[i]);
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
