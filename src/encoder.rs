use crate::result::Result;
use ac_ffmpeg::codec::video::{VideoEncoder, VideoFrame, VideoFrameMut};
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use std::ptr::null_mut;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{mem, slice};

use x264_sys::{
    x264_encoder_close, x264_encoder_encode, x264_encoder_open, x264_nal_t,
    x264_param_apply_profile, x264_param_default_preset, x264_picture_alloc, x264_picture_clean,
    x264_picture_t, x264_t, X264_CSP_I420, X264_TYPE_AUTO, X264_TYPE_IDR,
};

pub trait Encode {
    fn encode(&mut self, y: &[u8], u: &[u8], v: &[u8]) -> Result<Vec<u8>>;
}

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame: VideoFrame,
    frame_idx: usize,
    w: u32,
    h: u32,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32) -> Self {
        let time_base = TimeBase::new(1, 30);

        let pixel_format = video::frame::get_pixel_format("yuv420p");

        // create a black video frame with a given resolution
        let frame = VideoFrameMut::black(pixel_format, w as _, h as _)
            .with_time_base(time_base)
            .freeze();

        let mut encoder = VideoEncoder::builder("h264_nvenc")
            .unwrap()
            .pixel_format(pixel_format)
            .set_option("profile", "baseline")
            .set_option("compression_level", 8)
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
            frame_idx: 0,
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Encode for FfmpegEncoder {
    fn encode(&mut self, y: &[u8], u: &[u8], v: &[u8]) -> Result<Vec<u8>> {
        let tb = TimeBase::new(1, 30);
        let mut frame_timestamp = Timestamp::new(self.frame_idx as i64, tb);
        let mut frame = VideoFrameMut::black(
            video::frame::get_pixel_format("yuv420p"),
            self.w as _,
            self.h as _,
        )
        .with_time_base(tb)
        .with_pts(frame_timestamp);
        let mut planes = frame.planes_mut();
        planes[0].data_mut().copy_from_slice(y);
        planes[1].data_mut().copy_from_slice(u);
        planes[2].data_mut().copy_from_slice(v);

        self.encoder.push(frame.freeze())?;
        self.frame_idx += 1;
        let res = self.encoder.take()?;
        match res {
            Some(frame) => Ok(frame.data().to_owned()),
            None => Ok(Vec::new()),
        }
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        unsafe {}
    }
}
