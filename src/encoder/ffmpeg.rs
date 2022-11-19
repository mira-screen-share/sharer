use crate::result::Result;
use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};

use crate::capture::BGR0YUVConverter;
use crate::config::EncoderConfig;
use crate::encoder::frame_pool::FramePool;
use bytes::Bytes;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    bgr0_to_yuv: BGR0YUVConverter,
    pixel_format: String,
    w: usize,
    h: usize,
    pub force_idr: Arc<AtomicBool>,
}

unsafe impl Send for FfmpegEncoder {}

pub enum FrameData<'a> {
    NV12(&'a [u8]),
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
            encoder = encoder.set_option(&option.0, option.1);
        }

        let encoder = encoder.build().unwrap();

        Self {
            encoder,
            pixel_format: encoder_config.pixel_format.clone(),
            bgr0_to_yuv: BGR0YUVConverter::new(w as usize, h as usize),
            frame_pool: FramePool::new(w, h, time_base, pixel_format),
            force_idr: Arc::new(AtomicBool::new(false)),
            w: w as usize,h: h as usize,
        }
    }


    pub fn encode(&mut self, frame_data: FrameData, frame_time: i64) -> Result<Bytes> {
        let mut frame = self.frame_pool.take();
        let time_base = frame.time_base();
        frame = frame
            .with_pts(Timestamp::new(
                (frame_time as f64 * 9. / 1000.) as i64,
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
                frame.planes_mut()[0].data_mut().copy_from_slice(&nv12[0..self.w*self.h]);
                frame.planes_mut()[1].data_mut().copy_from_slice(&nv12[self.w*self.h..]);
            }
            FrameData::BGR0(bgr0) => {
                match self.pixel_format.as_str() {
                    "yuv420p" => {
                        self.bgr0_to_yuv.convert(
                            bgr0,
                            frame
                                .planes_mut()
                                .iter_mut()
                                .map(|p| p.data_mut())
                                .collect(),
                        );
                    }
                    "bgra" => {
                        frame.planes_mut()[0].data_mut().copy_from_slice(bgr0);
                    }
                    _ => unimplemented!(),
                }
            }
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
}
