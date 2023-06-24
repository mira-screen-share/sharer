use std::ffi::c_void;
use std::ops::Deref;
use std::ptr::null_mut;
use std::slice;
use std::sync::{Arc, Barrier, Once};

use apple_sys::CoreMedia::{
    CMSampleBufferGetImageBuffer, CMSampleBufferGetNumSamples,
    CMSampleBufferGetPresentationTimeStamp, CMSampleBufferRef, CVPixelBufferGetBaseAddressOfPlane,
    CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeight, CVPixelBufferGetWidth,
    CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
};
use apple_sys::ScreenCaptureKit::{
    dispatch_queue_create, id, INSError, INSObject, ISCStream, NSError, NSObject,
    NSString_NSStringDeprecated, PNSObject, SCContentFilter, SCStream, SCStreamConfiguration,
    SCStreamOutputType_SCStreamOutputTypeAudio, SCStreamOutputType_SCStreamOutputTypeScreen,
};
use block::Block;
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::{from_nsstring, objc_closure};
use crate::capture::macos::ffi::UnsafeSendable;
use crate::capture::YUVFrame;

pub struct CaptureEngine {
    stream: Option<SCStream>,
    output: Option<StreamOutput>,
}

unsafe impl Send for CaptureEngine {}

impl Drop for CaptureEngine {
    fn drop(&mut self) {
        unsafe {
            self.stream.take().map(|stream| stream.release());
            self.output.take().map(|output| output.release());
        }
    }
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self {
            stream: None,
            output: None,
        }
    }

    pub unsafe fn start_capture(
        &mut self,
        config: SCStreamConfiguration,
        filter: SCContentFilter,
        video_tx: Sender<YUVFrame>,
    ) {
        // TODO audio capture
        let (audio_tx, mut audio_rx) = mpsc::channel::<i32>(1);

        let mut output = StreamOutput(StreamOutput::alloc().init());
        output.set_on_output_frame(OutputHandler {
            video_sender: video_tx,
            audio_sender: audio_tx,
        });

        tokio::spawn(async move {
            while let Some(_) = audio_rx.recv().await {
                // TODO
            }
        });

        let stream = SCStream(SCStream::alloc().initWithFilter_configuration_delegate_(
            filter,
            config,
            output.0 as _,
        ));

        stream.addStreamOutput_type_sampleHandlerQueue_error_(
            output.0 as _,
            SCStreamOutputType_SCStreamOutputTypeScreen,
            dispatch_queue_create(
                b"app.mirashare.screen\0".as_ptr() as *const _,
                NSObject(null_mut()),
            ),
            NSError(NSError::alloc().init()).0 as _,
        );

        stream.addStreamOutput_type_sampleHandlerQueue_error_(
            output.0 as _,
            SCStreamOutputType_SCStreamOutputTypeAudio,
            dispatch_queue_create(
                b"app.mirashare.audio\0".as_ptr() as *const _,
                NSObject(null_mut()),
            ),
            NSError(NSError::alloc().init()).0 as _,
        );

        self.output.replace(output).map(|output| output.release());
        self.stream.replace(stream).map(|stream| stream.release());
        self.stream
            .unwrap()
            .startCaptureWithCompletionHandler_(objc_closure!(move |error: id| {
                if error.is_null() {
                    info!("Started capturing.");
                } else {
                    let error = from_nsstring!(NSError(error).localizedDescription());
                    panic!("Error starting capturing: {:?}", error);
                }
            }));

        filter.finalize();
        filter.release();
        config.finalize();
        config.release();
    }

    pub async unsafe fn stop_capture(&mut self) {
        if let Some(stream) = &self.stream {
            let barrier = Arc::new(Barrier::new(2));
            let barrier_clone = barrier.clone();
            unsafe {
                stream.stopCaptureWithCompletionHandler_(objc_closure!(move |error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        error!("Error stopping capture: {}", error);
                    }
                    barrier_clone.wait();
                }));
            }
            barrier.wait();
        }
    }

    pub async unsafe fn update(
        &mut self,
        param: UnsafeSendable<(SCStreamConfiguration, SCContentFilter)>,
    ) {
        if let Some(stream) = &self.stream {
            let barrier = Arc::new(Barrier::new(3));
            let barrier_conf = barrier.clone();
            let barrier_filter = barrier.clone();
            let (config, filter) = param.0;
            stream.updateConfiguration_completionHandler_(
                config,
                objc_closure!(move |error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        error!("Failed to update the stream session: {}", error);
                    }
                    barrier_conf.wait();
                }),
            );
            stream.updateContentFilter_completionHandler_(
                filter,
                objc_closure!(move |error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        error!("Failed to update the stream session: {}", error);
                    }
                    barrier_filter.wait();
                }),
            );
            barrier.wait();

            filter.finalize();
            filter.release();
            config.finalize();
            config.release();
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct StreamOutput(pub id);

pub struct OutputHandler {
    pub video_sender: Sender<YUVFrame>,
    pub audio_sender: Sender<i32>,
}

pub struct ErrorHandler {
    pub error_sender: Sender<String>,
}

impl StreamOutput {
    pub fn alloc() -> Self {
        static REGISTER: Once = Once::new();
        REGISTER.call_once(Self::register);
        Self(unsafe { msg_send!(class!(StreamOutput), alloc) })
    }

    fn register() {
        let mut decl = ClassDecl::new("StreamOutput", class!(NSObject)).unwrap();
        decl.add_ivar::<*mut c_void>("on_output_frame");
        decl.add_ivar::<*mut c_void>("on_stopped_with_error");
        unsafe {
            decl.add_method(
                sel!(stream:didOutputSampleBuffer:ofType:),
                stream_output as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object, u8),
            );
            decl.add_method(
                sel!(stream:didStopWithError:),
                stream_delegate as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );
        }
        decl.register();
    }

    fn set_on_output_frame(&mut self, output_handler: OutputHandler) {
        unsafe {
            let obj = &mut *(self.0 as *mut _ as *mut Object);
            obj.set_ivar(
                "on_output_frame",
                Box::into_raw(Box::new(output_handler)) as *mut c_void,
            );
        }
    }

    fn set_on_stopped_with_error(&mut self, error_handler: ErrorHandler) {
        unsafe {
            let obj = &mut *(self.0 as *mut _ as *mut Object);
            obj.set_ivar(
                "on_stopped_with_error",
                Box::into_raw(Box::new(error_handler)) as *mut c_void,
            );
        }
    }
}

impl INSObject for StreamOutput {}

impl PNSObject for StreamOutput {}

impl Deref for StreamOutput {
    type Target = Object;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

extern "C" fn stream_output(this: &mut Object, _cmd: Sel, _stream: id, sample: id, of_type: u8) {
    unsafe {
        let ptr = *this.get_ivar::<*mut c_void>("on_output_frame") as *mut OutputHandler;
        if ptr.is_null() {
            return;
        }

        let handler = &*ptr;
        #[allow(non_upper_case_globals)]
        match of_type as i64 {
            SCStreamOutputType_SCStreamOutputTypeScreen => {
                let sample_buffer_ref = sample as CMSampleBufferRef;
                let epoch = CMSampleBufferGetPresentationTimeStamp(sample_buffer_ref).epoch;
                let pixel_buffer = CMSampleBufferGetImageBuffer(sample_buffer_ref);

                CVPixelBufferLockBaseAddress(pixel_buffer, 0);

                let width = CVPixelBufferGetWidth(pixel_buffer);
                let height = CVPixelBufferGetHeight(pixel_buffer);
                if width == 0 || height == 0 {
                    return;
                }

                let luminance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 0);
                let luminance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 0);
                let luminance_bytes = slice::from_raw_parts(
                    luminance_bytes_address as *mut u8,
                    height * luminance_stride,
                )
                    .to_vec();

                let chrominance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 1);
                let chrominance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 1);
                let chrominance_bytes = slice::from_raw_parts(
                    chrominance_bytes_address as *mut u8,
                    height * chrominance_stride / 2,
                )
                    .to_vec();

                CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

                let frame = YUVFrame {
                    display_time: epoch as u64,
                    width: width as i32,
                    height: height as i32,
                    luminance_bytes,
                    luminance_stride: luminance_stride as i32,
                    chrominance_bytes,
                    chrominance_stride: chrominance_stride as i32,
                };

                handler.video_sender.try_send(frame).unwrap_or(());
            }
            SCStreamOutputType_SCStreamOutputTypeAudio => {
                let sample_buffer_ref = sample as CMSampleBufferRef;
                let a = CMSampleBufferGetNumSamples(sample_buffer_ref) as i32;

                handler.audio_sender.try_send(a).unwrap_or(());
            }
            _ => {
                error!("Unknown output type: {}", of_type);
            }
        }
    }
}

extern "C" fn stream_delegate(this: &mut Object, _cmd: Sel, _stream: id, error: id) {
    unsafe {
        let ptr = *this.get_ivar::<*mut c_void>("on_output_frame") as *mut ErrorHandler;
        if ptr.is_null() {
            return;
        }

        let handler = &*ptr;
        let error = from_nsstring!(NSError(error).localizedDescription());
        error!("Stream stopped due to error: {}", error);
        handler
            .error_sender
            .try_send(error.to_string())
            .unwrap_or_else(move |err| warn!("Failed to send error: {}", err.to_string()));
    }
}
