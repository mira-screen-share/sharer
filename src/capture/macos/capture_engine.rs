use std::ffi::c_void;
use std::ops::Deref;
use std::ptr::null_mut;
use std::sync::{Arc, Barrier, Once};
use std::{mem, slice};

use apple_sys::AVFAudio::{
    kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment, AVAudioFormat, AVAudioPCMBuffer,
    CFRelease, CMAudioFormatDescriptionGetStreamBasicDescription, CMAudioFormatDescriptionRef,
    CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer, CMSampleBufferGetDataBuffer,
    IAVAudioFormat, IAVAudioPCMBuffer,
};
use apple_sys::CoreMedia::{
    CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionaryGetValue, CFDictionaryRef,
    CFNumberGetValue, CFNumberType_kCFNumberSInt64Type, CMSampleBufferGetFormatDescription,
    CMSampleBufferGetImageBuffer, CMSampleBufferGetPresentationTimeStamp,
    CMSampleBufferGetSampleAttachmentsArray, CMSampleBufferIsValid, CMSampleBufferRef,
    CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeight,
    CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
};
use apple_sys::ScreenCaptureKit::{
    dispatch_queue_create, id, CFTypeRef, INSError, INSObject, ISCStream, NSError, NSObject,
    NSString_NSStringDeprecated, PNSObject, SCContentFilter, SCFrameStatus_SCFrameStatusComplete,
    SCFrameStatus_SCFrameStatusIdle, SCStream, SCStreamConfiguration, SCStreamFrameInfoStatus,
    SCStreamOutputType_SCStreamOutputTypeAudio, SCStreamOutputType_SCStreamOutputTypeScreen,
};
use block::Block;
use objc::declare::ClassDecl;
use objc::runtime::{Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use tokio::sync::mpsc::Sender;

use crate::capture::macos::ffi::UnsafeSendable;
use crate::capture::macos::pcm_buffer::PCMBuffer;
use crate::capture::YUVFrame;
use crate::{from_nsstring, objc_closure};

pub struct CaptureEngine {
    stream: Option<SCStream>,
    output: Option<StreamOutput>,
}

unsafe impl Send for CaptureEngine {}

impl Drop for CaptureEngine {
    fn drop(&mut self) {
        unsafe {
            if let Some(stream) = self.stream.take() {
                stream.release()
            }
            if let Some(output) = self.output.take() {
                output.release()
            }
        }
    }
}

#[allow(dead_code)]
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
        audio_tx: Sender<PCMBuffer>,
    ) {
        let mut output = StreamOutput(StreamOutput::alloc().init());
        output.set_on_output_frame(OutputHandler {
            video_sender: video_tx,
            audio_sender: audio_tx,
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
    pub audio_sender: Sender<PCMBuffer>,
}

pub struct ErrorHandler {
    pub error_sender: Sender<String>,
}

#[allow(dead_code)]
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
        let handler = {
            let ptr = *this.get_ivar::<*mut c_void>("on_output_frame") as *mut OutputHandler;
            if ptr.is_null() {
                return;
            }
            &*ptr
        };

        let sample_buffer_ref = {
            let ret = sample as CMSampleBufferRef;
            if CMSampleBufferIsValid(ret) == 0 {
                return;
            }
            ret
        };

        #[allow(non_upper_case_globals)]
        match of_type as i64 {
            SCStreamOutputType_SCStreamOutputTypeScreen => {
                if let Some(frame) = create_yuv_frame(sample_buffer_ref) {
                    handler.video_sender.try_send(frame).unwrap_or(());
                }
            }
            SCStreamOutputType_SCStreamOutputTypeAudio => {
                if let Some(frame) = create_pcm_buffer(sample_buffer_ref) {
                    handler.audio_sender.try_send(frame).unwrap_or(());
                }
            }
            _ => {
                error!("Unknown output type: {}", of_type);
            }
        }
    }
}

unsafe fn create_yuv_frame(sample_buffer_ref: CMSampleBufferRef) -> Option<YUVFrame> {
    {
        // Check that the frame status is complete
        let attachments = CMSampleBufferGetSampleAttachmentsArray(sample_buffer_ref, 0);
        if attachments.is_null() || CFArrayGetCount(attachments) == 0 {
            return None;
        }
        let attachment = CFArrayGetValueAtIndex(attachments, 0) as CFDictionaryRef;
        let frame_status_ref =
            CFDictionaryGetValue(attachment, SCStreamFrameInfoStatus.0 as _) as CFTypeRef;
        if frame_status_ref.is_null() {
            return None;
        }
        let mut frame_status: i64 = 0;
        let result = CFNumberGetValue(
            frame_status_ref as _,
            CFNumberType_kCFNumberSInt64Type,
            mem::transmute(&mut frame_status),
        );
        if result == 0 {
            return None;
        }
        if frame_status != SCFrameStatus_SCFrameStatusComplete
            && frame_status != SCFrameStatus_SCFrameStatusIdle
        {
            return None;
        }
    }

    let epoch = CMSampleBufferGetPresentationTimeStamp(sample_buffer_ref).epoch;
    let pixel_buffer = CMSampleBufferGetImageBuffer(sample_buffer_ref);

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let width = CVPixelBufferGetWidth(pixel_buffer);
    let height = CVPixelBufferGetHeight(pixel_buffer);
    if width == 0 || height == 0 {
        return None;
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

    YUVFrame {
        display_time: epoch as u64,
        width: width as i32,
        height: height as i32,
        luminance_bytes,
        luminance_stride: luminance_stride as i32,
        chrominance_bytes,
        chrominance_stride: chrominance_stride as i32,
    }
    .into()
}

unsafe fn create_pcm_buffer(sample_buffer_ref: CMSampleBufferRef) -> Option<PCMBuffer> {
    let mut buffer_size = 0;
    CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sample_buffer_ref as _,
        &mut buffer_size,
        null_mut(),
        0,
        null_mut(),
        null_mut(),
        0,
        null_mut(),
    );

    let mut block_buffer_ref = CMSampleBufferGetDataBuffer(sample_buffer_ref as _);
    let audio_buffer_list_ptr =
        std::alloc::alloc(std::alloc::Layout::from_size_align(buffer_size as usize, 16).unwrap());
    let result = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sample_buffer_ref as _,
        null_mut(),
        audio_buffer_list_ptr as _,
        buffer_size,
        null_mut(),
        null_mut(),
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
        &mut block_buffer_ref,
    );
    CFRelease(block_buffer_ref as _);
    if result != 0 {
        return None;
    }
    let audio_format_description_ref =
        CMSampleBufferGetFormatDescription(sample_buffer_ref) as CMAudioFormatDescriptionRef;
    let audio_stream_basic_description =
        &*CMAudioFormatDescriptionGetStreamBasicDescription(audio_format_description_ref);
    let sample_rate = audio_stream_basic_description.mSampleRate;
    let channels = audio_stream_basic_description.mChannelsPerFrame;
    let format = AVAudioFormat(
        AVAudioFormat::alloc().initStandardFormatWithSampleRate_channels_(sample_rate, channels),
    );
    let pcm_buffer = AVAudioPCMBuffer(
        AVAudioPCMBuffer::alloc().initWithPCMFormat_bufferListNoCopy_deallocator_(
            format,
            audio_buffer_list_ptr as *const _ as _,
            objc_closure!(move |_: id| {
                std::alloc::dealloc(
                    audio_buffer_list_ptr,
                    std::alloc::Layout::from_size_align(buffer_size as usize, 16).unwrap(),
                );
            }),
        ),
    );

    use apple_sys::AVFAudio::PNSObject;
    format.release();

    PCMBuffer::new(pcm_buffer).into()
}

extern "C" fn stream_delegate(this: &mut Object, _cmd: Sel, _stream: id, error: id) {
    unsafe {
        let error = from_nsstring!(NSError(error).localizedDescription());
        error!("Stream stopped due to error: {}", error);

        let handler = {
            let ptr = *this.get_ivar::<*mut c_void>("on_stopped_with_error") as *mut ErrorHandler;
            if ptr.is_null() {
                return;
            }
            &*ptr
        };
        handler
            .error_sender
            .try_send(error.to_string())
            .unwrap_or_else(move |err| warn!("Failed to send error: {}", err.to_string()));
    }
}
