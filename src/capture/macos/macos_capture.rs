use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use std::{ops, ptr, slice};

use async_trait::async_trait;
use block::{Block, ConcreteBlock};
use failure::{format_err, Error};
use libc::c_void;
use tokio::sync::mpsc::Receiver;

use crate::capture::macos::config::Config as CaptureConfig;
use crate::capture::macos::display::Display;
use crate::capture::macos::ffi::CGDisplayStreamFrameStatus::FrameComplete;
use crate::capture::macos::ffi::PixelFormat::{Argb8888, YCbCr420Full, YCbCr420Video};
use crate::capture::macos::ffi::{
    dispatch_queue_create, dispatch_release, CFRelease, CGDisplayStreamCreateWithDispatchQueue,
    CGDisplayStreamRef, CGDisplayStreamStart, CGDisplayStreamStop, CGError, DispatchQueue,
    FrameAvailableHandler,
};
use crate::capture::macos::frame::Frame;
use crate::config::Config;
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;
use crate::{OutputSink, ScreenCapture};

pub struct MacOSScreenCapture<'a> {
    stream: CGDisplayStreamRef,
    queue: DispatchQueue,
    display: Display,
    receiver: Receiver<Frame>,
    config: &'a Config,
}

pub struct RFrame<'a>(Frame, PhantomData<&'a [u8]>);

unsafe impl Send for MacOSScreenCapture<'_> {}

unsafe impl Send for Frame {}

unsafe impl Send for RFrame<'_> {}

impl<'a> Deref for RFrame<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

pub type GraphicsCaptureItem = Display;

impl<'a> MacOSScreenCapture<'a> {
    pub fn new(display: GraphicsCaptureItem, config: &'a Config) -> Result<Self> {
        let format = YCbCr420Full;
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Frame>(1);

        let handler: FrameAvailableHandler =
            ConcreteBlock::new(move |status, display_time, surface, _| unsafe {
                if status == FrameComplete {
                    if let Ok(permit) = sender.try_reserve() {
                        permit.send(Frame::new(surface, display_time));
                    }
                }
            })
            .copy();

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
                display,
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
            profiler.done_conversion();
            let encoded = encoder
                .encode(
                    FrameData::NV12(&RFrame(frame, PhantomData)),
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

impl Drop for MacOSScreenCapture<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = CGDisplayStreamStop(self.stream);
            CFRelease(self.stream);
            dispatch_release(self.queue);
        }
    }
}
