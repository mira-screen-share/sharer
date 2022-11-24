use std::{ptr, slice};
use std::time::Duration;

use async_trait::async_trait;
use block::{Block, ConcreteBlock};
use failure::format_err;
use libc::c_void;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

use crate::{OutputSink, ScreenCapture};
use crate::capture::frame::YUVFrame;
use crate::capture::macos::config::Config as CaptureConfig;
use crate::capture::macos::display::Display;
use crate::capture::macos::ffi::{CFRelease, CGDisplayStreamCreateWithDispatchQueue, CGDisplayStreamFrameStatus, CGDisplayStreamRef, CGDisplayStreamStart, CGDisplayStreamStop, CGDisplayStreamUpdateGetDropCount, CGDisplayStreamUpdateRef, CGError, CVPixelBufferCreateWithIOSurface, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeight, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress, CVPixelBufferRelease, CVPixelBufferUnlockBaseAddress, dispatch_queue_create, dispatch_release, DispatchQueue, FrameAvailableHandler, IOSurfaceRef};
use crate::capture::macos::ffi::CGDisplayStreamFrameStatus::{FrameComplete, Stopped};
use crate::capture::macos::ffi::PixelFormat::YCbCr420Full;
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;

pub struct MacOSScreenCapture<'a> {
    stream: CGDisplayStreamRef,
    queue: DispatchQueue,
    receiver: Receiver<YUVFrame>,
    config: &'a Config,
}

unsafe impl Send for MacOSScreenCapture<'_> {}

pub type GraphicsCaptureItem = Display;

impl<'a> MacOSScreenCapture<'a> {
    pub fn new(display: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
        let format = YCbCr420Full;
        let (sender, receiver) = tokio::sync::mpsc::channel::<YUVFrame>(1);
        let sender = Box::into_raw(Box::new(sender));

        let handler: FrameAvailableHandler =
            ConcreteBlock::new(
                move |status: CGDisplayStreamFrameStatus,
                      display_time: u64,
                      frame_surface: IOSurfaceRef,
                      update_ref: CGDisplayStreamUpdateRef| {
                    unsafe {
                        frame_available_handler(
                            display_time,
                            sender,
                            status,
                            frame_surface,
                            update_ref,
                        )
                    }
                },
            ).copy();

        let queue = unsafe {
            dispatch_queue_create(b"app.mirashare\0".as_ptr() as *const i8, ptr::null_mut())
        };

        let stream = unsafe {
            let capture_config = CaptureConfig {
                cursor: true,
                letterbox: true,
                throttle: 1. / (config.max_fps as f64),
                queue_length: 3,
            }
                .build();
            let stream = CGDisplayStreamCreateWithDispatchQueue(
                display.id(),
                display.width(),
                display.height(),
                format,
                capture_config,
                queue,
                &*handler as *const Block<_, _> as *const c_void,
            );
            CFRelease(capture_config);
            stream
        };

        match unsafe { CGDisplayStreamStart(stream) } {
            CGError::Success => Ok(Self {
                stream,
                queue,
                receiver,
                config,
            }),
            x => Err(format_err!("Failed to start capture: {:?}", x)),
        }
    }
}

#[async_trait]
impl ScreenCapture for MacOSScreenCapture<'_> {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {
        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / self.config.max_fps) as u64));

        while let Some(frame) = self.receiver.recv().await {
            let frame_time = frame.display_time as f64;
            profiler.accept_frame(frame_time as i64);
            profiler.done_preprocessing();
            let encoded = encoder
                .encode(
                    FrameData::NV12(&frame),
                    frame_time as i64,
                )
                .unwrap();
            let encoded_len = encoded.len();
            profiler.done_encoding();
            output.write(encoded).await.unwrap();
            profiler.done_processing(encoded_len);
            ticker.tick().await;
        }

        Ok(())
    }
}

unsafe fn frame_available_handler(
    display_time: u64,
    sender: *mut Sender<YUVFrame>,
    status: CGDisplayStreamFrameStatus,
    frame_surface: IOSurfaceRef,
    update_ref: CGDisplayStreamUpdateRef,
) {
    match status {
        Stopped => {
            let _ = Box::from_raw(sender);
            return;
        }
        FrameComplete => {
            if sender.is_null() {
                return;
            }
        }
        _ => return,
    }

    let mut pixel_buffer = ptr::null_mut();
    if CVPixelBufferCreateWithIOSurface(
        ptr::null(),
        frame_surface,
        ptr::null_mut(),
        &mut pixel_buffer,
    ) != 0 {
        error!("CVPixelBufferCreateWithIOSurface failed");
        return;
    }

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let width = CVPixelBufferGetWidth(pixel_buffer);
    let height = CVPixelBufferGetHeight(pixel_buffer);

    let luminance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 0);
    let luminance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 0);
    let luminance_bytes = slice::from_raw_parts(
        luminance_bytes_address as *mut u8,
        height * luminance_stride,
    ).to_vec();

    let chrominance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 1);
    let chrominance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 1);
    let chrominance_bytes = slice::from_raw_parts(
        chrominance_bytes_address as *mut u8,
        height * chrominance_stride / 2,
    ).to_vec();

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    let capture_frame = YUVFrame {
        display_time,
        width: width as i32,
        height: height as i32,
        luminance_bytes,
        luminance_stride: luminance_stride as i32,
        chrominance_bytes,
        chrominance_stride: chrominance_stride as i32,
    };

    if let Ok(permit) = (*sender).try_reserve() {
        permit.send(capture_frame);
    }

    let dropped_frames = CGDisplayStreamUpdateGetDropCount(update_ref);
    if dropped_frames > 0 {
        warn!("{} {}", dropped_frames, "drop frames");
    }

    if !pixel_buffer.is_null() {
        CVPixelBufferRelease(pixel_buffer);
    }
}

impl Drop for MacOSScreenCapture<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = CGDisplayStreamStop(self.stream);
            CFRelease(self.stream);
            dispatch_release(self.queue);
        }
    }
}
