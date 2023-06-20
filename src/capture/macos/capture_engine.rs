use std::ffi::c_void;
use std::ops::Deref;
use std::ptr::null_mut;
use std::sync::Once;

use apple_sys::ScreenCaptureKit::{dispatch_queue_create, id, INSError, INSObject, ISCStream, NSError, NSObject, NSString_NSStringDeprecated, PNSObject, SCContentFilter, SCStream, SCStreamConfiguration, SCStreamOutputType_SCStreamOutputTypeAudio, SCStreamOutputType_SCStreamOutputTypeScreen};
use block::Block;
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::{from_nsstring, objc_closure};

pub struct CaptureEngine {
    stream: Option<SCStream>,
    output: Option<StreamOutput>,
}

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

    pub unsafe fn start_capture(&mut self, config: SCStreamConfiguration, filter: SCContentFilter) {
        let (video_tx, mut video_rx) = mpsc::channel::<()>(1);
        let (audio_tx, mut audio_rx) = mpsc::channel::<()>(1);

        let mut output = StreamOutput(StreamOutput::alloc().init());
        output.set_on_output_frame(OutputHandler {
            video_sender: video_tx,
            audio_sender: audio_tx,
        });

        tokio::spawn(async move {
            while let Some(()) = video_rx.recv().await {
                info!("received video buffer");
            }
        });
        tokio::spawn(async move {
            while let Some(()) = audio_rx.recv().await {
                info!("received audio buffer");
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
        self.stream.unwrap().startCaptureWithCompletionHandler_(objc_closure!(move |error: id| {
            if error.is_null() {
                println!("Started recording");
            } else {
                println!("Error starting recording: {:?}", error);
            }
        }));
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct StreamOutput(pub id);

pub struct OutputHandler {
    pub video_sender: Sender<()>,
    pub audio_sender: Sender<()>,
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

extern "C" fn stream_output(this: &mut Object, _cmd: Sel, _stream: id, _sample: id, of_type: u8) {
    unsafe {
        let ptr = *this.get_ivar::<*mut c_void>("on_output_frame") as *mut OutputHandler;
        if ptr.is_null() {
            return;
        }

        let handler = &*ptr;
        #[allow(non_upper_case_globals)]
        match of_type as i64 {
            SCStreamOutputType_SCStreamOutputTypeScreen => {
                handler
                    .video_sender
                    .try_send(())
                    .unwrap_or_else(move |err| {
                        warn!("Failed to send video frame: {}", err.to_string())
                    });
            }
            SCStreamOutputType_SCStreamOutputTypeAudio => {
                handler
                    .audio_sender
                    .try_send(())
                    .unwrap_or_else(move |err| {
                        warn!("Failed to send audio frame: {}", err.to_string())
                    });
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