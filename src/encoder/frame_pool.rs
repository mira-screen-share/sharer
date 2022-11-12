use ac_ffmpeg::codec::video::{PixelFormat, VideoFrame, VideoFrameMut};
use ac_ffmpeg::time::TimeBase;
use std::collections::VecDeque;

pub(crate) struct FramePool {
    frames: VecDeque<VideoFrame>,
    w: u32,
    h: u32,
    time_base: TimeBase,
    pixel_format: PixelFormat,
}

impl FramePool {
    pub fn new(w: u32, h: u32, time_base: TimeBase, pixel_format: PixelFormat) -> Self {
        Self {
            frames: VecDeque::new(),
            w,
            h,
            time_base,
            pixel_format,
        }
    }

    /// Put a given frame back to the pool after it was used.
    pub fn put(&mut self, frame: VideoFrame) {
        self.frames.push_back(frame);
    }

    /// Take a writable frame from the pool or allocate a new one if necessary.
    pub fn take(&mut self) -> VideoFrameMut {
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
