extern crate libc;

use std::ffi::c_void;
use std::ffi::CStr;

use apple_sys::ScreenCaptureKit::{CGSize, CMTime, id, INSArray, INSBundle, INSError, INSObject, INSScreen, ISCContentFilter, ISCDisplay, ISCRunningApplication, ISCShareableContent, ISCStreamConfiguration, ISCWindow, NSArray, NSArray_NSExtendedArray, NSBundle, NSError, NSScreen, NSString_NSStringDeprecated, SCContentFilter, SCDisplay, SCRunningApplication, SCShareableContent, SCStreamConfiguration, SCWindow};
use block::{Block, ConcreteBlock};
use itertools::Itertools;
use tokio::sync::mpsc;

enum CaptureType {
    Display,
    Window,
}

pub struct ScreenRecorder {
    is_running: bool,
    capture_type: CaptureType,
    selected_display: Option<SCDisplay>,
    selected_window: Option<SCWindow>,
    is_app_excluded: bool,
    content_size: CGSize,
    scale_factor: usize,
    available_apps: Vec<SCRunningApplication>,
    available_displays: Vec<SCDisplay>,
    available_windows: Vec<SCWindow>,
    is_audio_capture_enabled: bool,
    is_app_audio_excluded: bool,
    is_setup: bool,
}

macro_rules! objc_handler {
    ($a:expr) => {
        &*ConcreteBlock::new($a).copy() as *const Block<_, _> as *mut c_void
    };
}

trait FromNSArray<T> {
    fn from_nsarray(array: NSArray) -> Vec<T>;
}

trait ToNSArray<T> {
    fn to_nsarray(&self) -> NSArray;
}

fn new_nsarray<T: 'static>() -> NSArray {
    unsafe {
        NSArray(<NSArray as INSArray<T>>::init(&NSArray::alloc()))
    }
}

macro_rules! impl_from_to_nsarray_for {
    ($T:ident) => {
        impl FromNSArray<$T> for Vec<$T> {
            fn from_nsarray(array: NSArray) -> Vec<$T> {
                let mut vec = Vec::new();
                let count = unsafe {
                    <NSArray as INSArray<$T>>::count(&array)
                };
                for i in 0..count {
                    vec.push(unsafe {$T(<NSArray as INSArray<$T>>::objectAtIndex_(&array, i))});
                }
                vec
            }
        }

        impl ToNSArray<$T> for Vec<$T> {
            fn to_nsarray(&self) -> NSArray {
                unsafe {
                    let mut array = new_nsarray::<$T>();
                    for x in self {
                        array = <NSArray as NSArray_NSExtendedArray<$T>>::arrayByAddingObject_(&array, x.0);
                    }
                    array
                }
            }
        }
    };
}

macro_rules! from_nsarray {
    ($T:ident, $e:expr) => {
        <Vec<$T>>::from_nsarray($e)
    };
}

macro_rules! from_nsstring {
    ($s:expr) => {
        CStr::from_ptr($s.cString()).to_str().unwrap()
    };
}

macro_rules! aaa {
    ($T:ident, $e:expr, $then:expr, $or:expr) => {
        let res = unsafe { $e };
        if res.0.is_null() {
            $or
        } else {
            unsafe { $then }
        }
    };
}

impl_from_to_nsarray_for!(SCRunningApplication);
impl_from_to_nsarray_for!(SCDisplay);
impl_from_to_nsarray_for!(SCWindow);

#[derive(Debug)]
struct SendableContent(pub SCShareableContent);

unsafe impl Send for SendableContent {}

impl ScreenRecorder {
    pub fn new() -> Self {
        ScreenRecorder {
            is_running: false,
            capture_type: CaptureType::Display,
            selected_display: None,
            selected_window: None,
            is_app_excluded: true,
            content_size: CGSize {
                width: 1.,
                height: 1.,
            },
            scale_factor: {
                let screen = unsafe { NSScreen::mainScreen() };
                if screen.0.is_null() {
                    2
                } else {
                    (unsafe { screen.backingScaleFactor() }) as usize
                }
            },
            available_apps: Vec::new(),
            available_displays: Vec::new(),
            available_windows: Vec::new(),
            is_audio_capture_enabled: true,
            is_app_audio_excluded: false,
            is_setup: false,
        }
    }

    pub async fn can_record() -> bool {
        let (tx, mut rx) = mpsc::channel::<bool>(1);
        unsafe {
            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                false,
                true,
                objc_handler!(move |_content: id, error: id| {
                    let result = error.is_null();
                    tx.blocking_send(result).unwrap();
                }),
            );
        }
        rx.recv().await.unwrap()
    }

    pub async fn monitor_available_content(&mut self) {
        if self.is_setup { return; }
        self.refresh_available_content().await;
    }

    /// Starts capturing screen content.
    pub async fn start(&mut self) {
        // Exit early if already running.
        if self.is_running { return; }

        if !self.is_setup {
            // Starting polling for available screen content.
            self.monitor_available_content().await;
            self.is_setup = true;
        }

        // If the user enables audio capture, start monitoring the audio stream.
        if self.is_audio_capture_enabled {
            // TODO
        }

        let config = self.stream_configuration();
        let filter = self.content_filter();

        self.is_running = true;

        // TODO
        // // Start the stream and await new video frames.
        //             for try await frame in captureEngine.startCapture(configuration: config, filter: filter) {
        //                 capturePreview.updateFrame(frame)
        //                 if contentSize != frame.size {
        //                     // Update the content size if it changed.
        //                     contentSize = frame.size
        //                 }
        //             }

        unsafe {
            config.finalize();
            config.dealloc();
            filter.finalize();
            filter.dealloc();
        }
    }

    fn content_filter(&self) -> SCContentFilter {
        unsafe {
            match self.capture_type {
                CaptureType::Display => if let Some(display) = self.selected_display {
                    // If a user chooses to exclude the app from the stream,
                    // exclude it by matching its bundle identifier.
                    let excluded_apps: NSArray = if self.is_app_excluded {
                        self.available_apps.clone().into_iter().filter(|app| {
                            let bundle = from_nsstring!(app.bundleIdentifier());
                            let this_bundle = from_nsstring!(NSBundle::mainBundle().bundleIdentifier());
                            bundle != this_bundle
                        }).collect::<Vec<SCRunningApplication>>().to_nsarray()
                    } else {
                        new_nsarray::<SCRunningApplication>()
                    };
                    SCContentFilter(SCContentFilter::alloc().initWithDisplay_excludingApplications_exceptingWindows_(
                        display,
                        excluded_apps,
                        new_nsarray::<SCWindow>(),
                    ))
                } else {
                    panic!("No display selected.")
                }
                CaptureType::Window => if let Some(window) = self.selected_window {
                    todo!()
                } else {
                    panic!("No window selected.")
                }
            }
        }
    }

    fn stream_configuration(&self) -> SCStreamConfiguration {
        unsafe {
            let mut config = SCStreamConfiguration(SCStreamConfiguration::alloc().init());

            // Configure audio capture.
            config.setCapturesAudio_(self.is_audio_capture_enabled);
            config.setExcludesCurrentProcessAudio_(self.is_app_audio_excluded);

            match self.capture_type {
                CaptureType::Display => if let Some(display) = self.selected_display {
                    // Configure the display content width and height.
                    config.setWidth_(display.width() as usize * self.scale_factor);
                    config.setHeight_(display.height() as usize * self.scale_factor);
                }
                CaptureType::Window => if let Some(window) = self.selected_window {
                    // Configure the display content width and height.
                    config.setWidth_(window.frame().size.width as usize * 2);
                    config.setHeight_(window.frame().size.height as usize * 2);
                }
            }

            // Set the capture interval at 60 fps.
            // TODO pull from config
            config.setMinimumFrameInterval_(CMTime {
                value: 1,
                timescale: 60,
                flags: 0,
                epoch: 0,
            });

            // Increase the depth of the frame queue to ensure high fps at the expense of increasing
            // the memory footprint of WindowServer.
            config.setQueueDepth_(5);

            config
        }
    }

    async fn refresh_available_content(&mut self) {
        let (tx, mut rx) = mpsc::channel::<Option<SendableContent>>(1);
        unsafe {
            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                false,
                true,
                objc_handler!(move |content: id, error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        error!("Error getting shareable content: {}", error);
                        tx.blocking_send(None).unwrap();
                    } else {
                        tx.blocking_send(Some(SendableContent(SCShareableContent(content)))).unwrap();
                    }
                }),
            );
            if let Some(SendableContent(available_content)) = rx.recv().await.unwrap() {
                let available_displays = from_nsarray!(SCDisplay, available_content.displays());
                let available_windows = ScreenRecorder::filter_windows(
                    from_nsarray!(SCWindow, available_content.windows())
                );
                self.available_apps = from_nsarray!(SCRunningApplication, available_content.applications());
                if self.selected_display.is_none() {
                    self.selected_display = available_displays.first().cloned();
                }
                if self.selected_window.is_none() {
                    self.selected_window = available_windows.first().cloned();
                }
            }
        }
    }

    fn filter_windows(windows: Vec<SCWindow>) -> Vec<SCWindow> {
        windows
            .into_iter()
            // Sort the windows by app name.
            .sorted_by(|a, b| unsafe {
                let a = match { a.owningApplication() } {
                    app if app.0.is_null() => "",
                    app => from_nsstring!(app.applicationName()),
                };
                let b = match { b.owningApplication() } {
                    app if app.0.is_null() => "",
                    app => from_nsstring!(app.applicationName()),
                };
                if a < b {
                    std::cmp::Ordering::Less
                } else if a > b {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            // Remove windows that don't have an associated .app bundle.
            .filter(|window| unsafe {
                match { window.owningApplication() } {
                    app if app.0.is_null() => false,
                    app => !from_nsstring!(app.applicationName()).is_empty(),
                }
            })
            // Remove this app's window from the list.
            .filter(|window| unsafe {
                let window_bundle = from_nsstring!(window.owningApplication().bundleIdentifier());
                let this_bundle = from_nsstring!(NSBundle::mainBundle().bundleIdentifier());
                window_bundle != this_bundle
            })
            .collect()
    }
}

unsafe impl Send for ScreenRecorder {}

