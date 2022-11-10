use crate::result::Result;
use ac_ffmpeg::codec::video::{VideoEncoder, VideoFrame, VideoFrameMut};
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::packet::Packet;
use ac_ffmpeg::time::{TimeBase, Timestamp};
use std::os::raw::{c_int, c_void};
use std::ptr::null_mut;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{mem, ptr, slice};

struct VideoEncoderExposed {
    pub ptr: *mut c_void,
    pub time_base: TimeBase,
}

extern "C" {
    fn ffw_encoder_push_frame(encoder: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_frame_get_plane_data(frame: *mut c_void, index: usize) -> *mut u8;
    fn ffw_frame_new_black(pixel_format: c_int, width: c_int, height: c_int) -> *mut c_void;
    fn ffw_encoder_take_packet(encoder: *mut c_void, packet: *mut *mut c_void) -> c_int;
    fn ffw_packet_get_size(packet: *const c_void) -> c_int;
    fn ffw_packet_get_data(packet: *mut c_void) -> *mut c_void;
}

unsafe fn encoder_take(ptr: *mut c_void) -> Option<&'static [u8]> {
    let mut pptr = ptr::null_mut();
    let ret = ffw_encoder_take_packet(ptr, &mut pptr);
    if ret == 0 {
        return None;
    }
    if ret == -1 {
        panic!("!");
    }
    let data = ffw_packet_get_data(pptr) as *const u8;
    let size = ffw_packet_get_size(pptr) as usize;
    return Some(slice::from_raw_parts(data, size));
}

pub trait Encode {
    fn encode(&mut self, bgra: &[u8]) -> Result<&[u8]>;
}

pub struct FfmpegEncoder {
    encoder: *mut c_void,
    frame: *mut c_void,
    w: u32,
    h: u32,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32, fps: u32) -> Self {
        let pixel_format = video::frame::get_pixel_format("bgra");

        let frame = unsafe { ffw_frame_new_black(mem::transmute(pixel_format), w as _, h as _) };

        let mut encoder = VideoEncoder::builder("h264_nvenc")
            .unwrap()
            .pixel_format(pixel_format)
            .set_option("profile", "baseline")
            .set_option("compression_level", 8)
            .width(w as _)
            .height(h as _)
            .time_base(TimeBase::new(1, fps))
            .build()
            .unwrap();

        let encoder_exposed: VideoEncoderExposed = unsafe { mem::transmute(encoder) };

        Self {
            encoder: encoder_exposed.ptr,
            frame,
            w,
            h,
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Encode for FfmpegEncoder {
    fn encode(&mut self, bgra: &[u8]) -> Result<&[u8]> {
        let data = unsafe {
            slice::from_raw_parts_mut(ffw_frame_get_plane_data(self.frame, 0), bgra.len())
        };
        data.copy_from_slice(bgra);

        let frame = unsafe {
            ffw_encoder_push_frame(self.encoder, self.frame);
            encoder_take(self.encoder)
        };
        return Ok(frame.unwrap_or_else(|| &[]));
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        unsafe {}
    }
}
