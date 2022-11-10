use crate::capture::BGR0YUVConverter;
use crate::result::Result;
use ac_ffmpeg::codec::video::{VideoEncoder, VideoFrame, VideoFrameMut};
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use std::ptr::null_mut;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::{mem, slice};

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame: VideoFrame,
    frame_idx: usize,
    w: u32,
    h: u32,
    time_base: TimeBase,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32, fps: u32) -> Self {
        let time_base = TimeBase::new(1, 10_000);

        let pixel_format = video::frame::get_pixel_format("yuv420p");

        // create a black video frame with a given resolution
        let frame = VideoFrameMut::black(pixel_format, w as _, h as _)
            .with_time_base(time_base)
            .freeze();

        let mut encoder = VideoEncoder::builder("h264_nvenc") // libx264
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
            frame,
            w,
            h,
            time_base,
            frame_idx: 0,
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn encode(&mut self, bgra: &BGR0YUVConverter, frame_time: i64) -> Result<Vec<u8>> {
        let mut frame = VideoFrameMut::black(
            video::frame::get_pixel_format("yuv420p"),
            self.w as _,
            self.h as _,
        )
        .with_pts(Timestamp::new(
            (frame_time as f64 / 1000.) as i64,
            self.time_base,
        ));
        let mut planes = frame.planes_mut();
        planes[0].data_mut().copy_from_slice(bgra.y());
        planes[1].data_mut().copy_from_slice(bgra.u());
        planes[2].data_mut().copy_from_slice(bgra.v());

        self.encoder.push(frame.freeze())?;
        self.frame_idx += 1;
        let mut ret = Vec::new();
        let mut n = 0;
        while let Some(a) = self.encoder.take()? {
            ret.extend(a.data());
            n += 1;
        }
        //println!("{}", n);
        return Ok(ret);
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        unsafe {}
    }
}
