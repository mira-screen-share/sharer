use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use bytes::Bytes;
use itertools::enumerate;

use crate::capture::YUVFrame;
use crate::config::EncoderConfig;
use crate::encoder::frame_pool::FramePool;
use crate::result::Result;

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    pixel_format: String,
    w: usize,
    h: usize,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

#[allow(dead_code)]
pub enum FrameData<'a> {
    NV12(&'a YUVFrame),
    BGR0(&'a [u8]),
}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32, encoder_config: &EncoderConfig) -> Self {
        let time_base = TimeBase::new(1, 90_000);

        let pixel_format = video::frame::get_pixel_format(&encoder_config.pixel_format);

        let mut encoder = VideoEncoder::builder(&encoder_config.encoder)
            .unwrap()
            .pixel_format(pixel_format)
            .width(w as _)
            .height(h as _)
            .time_base(time_base);

        for option in &encoder_config.options {
            encoder = encoder.set_option(option.0, option.1);
        }

        let encoder = encoder.build().unwrap();

        Self {
            encoder,
            pixel_format: encoder_config.pixel_format.clone(),
            frame_pool: FramePool::new(w, h, time_base, pixel_format),
            force_idr: Arc::new(AtomicBool::new(false)),
            w: w as usize,
            h: h as usize,
        }
    }

    pub fn encode(&mut self, frame_data: FrameData, frame_time: i64) -> Result<Bytes> {
        let mut frame = self.frame_pool.take();
        let time_base = frame.time_base();
        frame = frame
            .with_pts(Timestamp::new(
                if cfg!(target_os = "windows") {
                    (frame_time as f64 * 9. / 1000.) as i64
                } else if cfg!(target_os = "macos") {
                    (frame_time as f64 * 9. / 1e5) as i64
                } else {
                    panic!("Unsupported OS")
                },
                time_base,
            ))
            .with_picture_type(
                if self
                    .force_idr
                    .swap(false, std::sync::atomic::Ordering::Relaxed)
                {
                    video::frame::PictureType::I
                } else {
                    video::frame::PictureType::None
                },
            );

        match frame_data {
            FrameData::NV12(nv12) => {
                assert_eq!(self.pixel_format, "nv12");
                let encoder_buffer_len = frame.planes_mut()[0].data_mut().len();
                let encoder_line_size = encoder_buffer_len / self.h as usize;

                self.copy_nv12(
                    &nv12.luminance_bytes,
                    nv12.luminance_stride as usize,
                    encoder_line_size,
                    frame.planes_mut()[0].data_mut(),
                );
                self.copy_nv12(
                    &nv12.chrominance_bytes,
                    nv12.chrominance_stride as usize,
                    encoder_line_size,
                    frame.planes_mut()[1].data_mut(),
                );
            }
            FrameData::BGR0(bgr0) => match self.pixel_format.as_str() {
                "bgra" => {
                    frame.planes_mut()[0].data_mut().copy_from_slice(bgr0);
                }
                _ => unimplemented!(),
            },
        }
        let frame = frame.freeze();
        self.encoder.push(frame.clone())?;
        self.frame_pool.put(frame);
        let mut ret = Vec::new();
        while let Some(packet) = self.encoder.take()? {
            ret.extend(packet.data());
        }
        Ok(Bytes::from(ret))
    }

    fn copy_nv12(
        &self,
        source: &[u8],
        stride: usize,
        encoder_line_size: usize,
        destination: &mut [u8],
    ) {
        // fast path
        if stride == encoder_line_size {
            destination.copy_from_slice(source);
            return;
        }

        for (r, row) in enumerate(source.chunks(stride)) {
            destination[r * encoder_line_size..r * encoder_line_size + self.w as usize]
                .copy_from_slice(&row[..self.w as usize])
        }
    }
}
