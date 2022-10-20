use crate::result::Result;
use std::ptr::null_mut;
use std::{mem, slice};

use x264_sys::{
    x264_encoder_close, x264_encoder_encode, x264_encoder_open, x264_nal_t,
    x264_param_apply_profile, x264_param_default_preset, x264_picture_alloc, x264_picture_clean,
    x264_picture_t, x264_t, X264_CSP_I420,
};

pub trait Encoder {
    fn encode(&mut self, y: &[u8], u: &[u8], v: &[u8]) -> Result<&[u8]>;
}

pub struct X264Encoder {
    encoder: *mut x264_t,
    pic_in: x264_picture_t,
    pic_out: mem::MaybeUninit<x264_picture_t>,
    nal: *const x264_nal_t,
    nal_size: i32,
}

unsafe impl Send for X264Encoder {}

impl X264Encoder {
    pub fn new(w: u32, h: u32) -> Self {
        let mut par = unsafe {
            let mut par = mem::MaybeUninit::uninit();
            x264_param_default_preset(
                par.as_mut_ptr(),
                b"ultrafast\0".as_ptr() as *const i8,
                b"zerolatency\0".as_ptr() as *const i8,
            );
            let mut par = par.assume_init();
            x264_param_apply_profile(&mut par as *mut _, b"baseline\0".as_ptr() as *const i8);

            par.i_width = w as i32;
            par.i_height = h as i32;
            //par.i_fps_num = 30;
            //par.i_threads = 4;
            par.i_csp = X264_CSP_I420 as i32;
            par
        };
        let pic_in = unsafe {
            let mut pic_in = mem::MaybeUninit::<x264_picture_t>::uninit();
            x264_picture_alloc(pic_in.as_mut_ptr(), par.i_csp, par.i_width, par.i_height);
            pic_in.assume_init()
        };
        Self {
            encoder: unsafe { x264_encoder_open(&mut par) },
            pic_in,
            pic_out: mem::MaybeUninit::<x264_picture_t>::uninit(),
            nal: null_mut(),
            nal_size: 0,
        }
    }
}

impl Encoder for X264Encoder {
    fn encode(&mut self, y: &[u8], u: &[u8], v: &[u8]) -> Result<&[u8]> {
        self.pic_in.img.plane = [
            y.as_ptr() as *mut u8,
            u.as_ptr() as *mut u8,
            v.as_ptr() as *mut u8,
            null_mut(),
        ];
        //pic_in.i_pts = ((frame_ms - start_relative_time.unwrap()) as f64 / (1.0 / 60.0 * 1000.0)).round() as i64;
        let frame_size = unsafe {
            x264_encoder_encode(
                self.encoder,
                &mut self.nal as *mut _ as *mut _,
                &mut self.nal_size,
                &mut self.pic_in,
                self.pic_out.as_mut_ptr(),
            )
        };
        return Ok(unsafe { slice::from_raw_parts((*self.nal).p_payload, frame_size as usize) });
    }
}

impl Drop for X264Encoder {
    fn drop(&mut self) {
        unsafe {
            x264_picture_clean(&mut self.pic_in);
            x264_encoder_close(self.encoder);
        }
    }
}
