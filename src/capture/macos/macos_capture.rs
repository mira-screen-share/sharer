use std::{ops, ptr, slice};
use std::marker::PhantomData;
use std::ops::Deref;
use std::time::Duration;

use async_trait::async_trait;
use block::{Block, ConcreteBlock};
use failure::{Error, format_err};
use libc::c_void;
use tokio::sync::mpsc::Receiver;

use crate::{OutputSink, ScreenCapture};
use crate::capture::macos::config::Config;
use crate::capture::macos::display::Display;
use crate::capture::macos::ffi::{CFRelease, CGDisplayStreamCreateWithDispatchQueue, CGDisplayStreamRef, CGDisplayStreamStart, CGDisplayStreamStop, CGError, dispatch_queue_create, dispatch_release, DispatchQueue, FrameAvailableHandler, PixelFormat};
use crate::capture::macos::ffi::CGDisplayStreamFrameStatus::FrameComplete;
use crate::capture::macos::ffi::PixelFormat::Argb8888;
use crate::capture::macos::frame::Frame;
use crate::encoder::FfmpegEncoder;
use crate::performance_profiler::PerformanceProfiler;
use crate::result::Result;

pub struct MacOSScreenCapture {
    stream: CGDisplayStreamRef,
    queue: DispatchQueue,

    width: usize,
    height: usize,
    format: PixelFormat,
    display: Display,
    receiver: Receiver<Frame>,
}

impl MacOSScreenCapture {
    pub fn new(
        display: Display,
        width: usize,
        height: usize,
        // format: PixelFormat,
        // config: Config,
    ) -> Result<Self> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Frame>(1);

        let handler: FrameAvailableHandler =
            ConcreteBlock::new(move |status, _, surface, _| unsafe {
                use crate::capture::macos::ffi::CGDisplayStreamFrameStatus::*;
                if status == FrameComplete {
                    sender.try_send(Frame::new(surface)).unwrap();
                }
            }).copy();

        let queue = unsafe {
            dispatch_queue_create(
                b"quadrupleslap.scrap\0".as_ptr() as *const i8,
                ptr::null_mut(),
            )
        };

        let stream = unsafe {
            let config_d: Config = Default::default();
            let config = config_d.build();
            let stream = CGDisplayStreamCreateWithDispatchQueue(
                display.id(),
                width,
                height,
                Argb8888,
                config,
                queue,
                &*handler as *const Block<_, _> as *const c_void,
            );
            CFRelease(config);
            stream
        };

        match unsafe { CGDisplayStreamStart(stream) } {
            CGError::Success => Ok(Self {
                stream,
                queue,
                width,
                height,
                display,
                receiver,
                format: Argb8888,
            }),
            x => Err(failure::format_err!("Failed to start capture: {:?}", x)),
        }
    }
}

pub struct RFrame<'a>(
    Frame,
    PhantomData<&'a [u8]>,
);

impl<'a> ops::Deref for RFrame<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.0
    }
}

#[async_trait]
impl ScreenCapture for MacOSScreenCapture {
    async fn capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        mut output: Box<impl OutputSink + Send + ?Sized>,
        mut profiler: PerformanceProfiler,
    ) -> Result<()> {
        let mut ticker =
            tokio::time::interval(Duration::from_millis((1000 / 60) as u64));

        while let Some(frame) = self.receiver.recv().await {
            let frame = RFrame(frame, PhantomData);


            // let frame_time = frame.SystemRelativeTime()?.Duration;
            // profiler.accept_frame(frame.SystemRelativeTime()?.Duration);
            // let (resource, frame) = unsafe { self.get_frame_content(frame)? };
            // profiler.done_preprocessing();
            // profiler.done_conversion();
            let encoded = encoder.encode(frame.deref(), 0).unwrap();
            // let encoded_len = encoded.len();
            // profiler.done_encoding();
            output.write(encoded).await.unwrap();
            // unsafe {
            //     self.d3d_context.Unmap(&resource, 0);
            // }
            // profiler.done_processing(encoded_len);
            ticker.tick().await;
        }

        Ok(())
    }
}

impl Drop for MacOSScreenCapture {
    fn drop(&mut self) {
        unsafe {
            //TODO: Maybe it should wait until `Stopped` before releasing?
            let _ = CGDisplayStreamStop(self.stream);
            CFRelease(self.stream);
            dispatch_release(self.queue);
        }
    }
}
