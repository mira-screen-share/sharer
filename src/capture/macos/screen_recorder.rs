extern crate libc;

use std::ffi::c_void;
use std::sync::{Arc, Barrier};

use apple_sys::ScreenCaptureKit::{
    id, CGSize, CMTime, INSError, INSObject, INSScreen, ISCContentFilter, ISCDisplay,
    ISCRunningApplication, ISCShareableContent, ISCStreamConfiguration, ISCWindow, NSError,
    NSScreen, NSString_NSStringDeprecated, PNSObject, SCContentFilter, SCDisplay,
    SCRunningApplication, SCShareableContent, SCStreamConfiguration, SCWindow,
};
use block::Block;
use itertools::Itertools;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::capture::macos::capture_engine::CaptureEngine;
use crate::capture::macos::ffi::{
    from_nsarray, from_nsstring, new_nsarray, objc_closure, FromNSArray, UnsafeSendable,
};
use crate::capture::macos::pcm_buffer::PCMBuffer;
use crate::capture::YUVFrame;

#[allow(dead_code)]
enum CaptureType {
    Display,
    Window,
}

#[allow(dead_code)]
pub struct ScreenRecorder {
    is_running: bool,
    capture_type: CaptureType,
    selected_display: Option<SCDisplay>,
    selected_window: Option<SCWindow>,
    is_app_excluded: bool,
    content_size: CGSize,
    scale_factor: usize,
    max_fps: u8,
    available_content: Option<SCShareableContent>,
    available_apps: Vec<SCRunningApplication>,
    available_displays: Vec<SCDisplay>,
    available_windows: Vec<SCWindow>,
    is_audio_capture_enabled: bool,
    is_app_audio_excluded: bool,
    capture_engine: CaptureEngine,
    is_setup: bool,
}

unsafe impl Send for ScreenRecorder {}

impl Drop for ScreenRecorder {
    fn drop(&mut self) {
        unsafe {
            if self.is_running {
                self.stop();
            }
            if let Some(content) = self.available_content.take() {
                content.release();
            }
        }
    }
}

#[allow(dead_code)]
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
            max_fps: 60,
            available_content: None,
            available_apps: Vec::new(),
            available_displays: Vec::new(),
            available_windows: Vec::new(),
            is_audio_capture_enabled: true,
            is_app_audio_excluded: false,
            capture_engine: CaptureEngine::new(),
            is_setup: false,
        }
    }

    pub fn set_max_fps(&mut self, fps: u8) {
        self.max_fps = fps;
    }

    pub async fn can_record() -> bool {
        let (tx, mut rx) = mpsc::channel::<bool>(1);
        unsafe {
            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                false,
                true,
                objc_closure!(move |_content: id, error: id| {
                    let result = error.is_null();
                    tx.blocking_send(result).unwrap();
                }),
            );
        }
        rx.recv().await.unwrap()
    }

    pub async fn monitor_available_content(&mut self) {
        if self.is_setup {
            return;
        }
        self.refresh_available_content().await;
    }

    /// Starts capturing screen content.
    pub async fn start(&mut self, video_tx: Sender<YUVFrame>, audio_tx: Sender<PCMBuffer>) {
        // Exit early if already running.
        if self.is_running {
            return;
        }

        if !self.is_setup {
            // Starting polling for available screen content.
            self.monitor_available_content().await;
            self.is_setup = true;
        }

        // If the user enables audio capture, start monitoring the audio stream.
        if self.is_audio_capture_enabled {
            // TODO
        }

        self.is_running = true;

        unsafe {
            self.capture_engine.start_capture(
                self.stream_configuration(),
                self.content_filter(),
                video_tx,
                audio_tx,
            );
        }
    }

    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }
        unsafe {
            self.capture_engine.stop_capture();
        }
        self.is_running = false;
    }

    fn content_filter(&self) -> SCContentFilter {
        unsafe {
            match self.capture_type {
                CaptureType::Display => {
                    if let Some(display) = self.selected_display {
                        // TODO ignore self
                        // If a user chooses to exclude the app from the stream,
                        // exclude it by matching its bundle identifier.
                        // let excluded_apps: NSArray = if self.is_app_excluded {
                        //     self.available_apps.clone().into_iter().filter(|app| {
                        //         let bundle = from_nsstring!(app.bundleIdentifier());
                        //         let this_bundle = from_nsstring!(NSBundle::mainBundle().bundleIdentifier());
                        //         bundle != this_bundle
                        //     }).collect::<Vec<SCRunningApplication>>().to_nsarray()
                        // } else {
                        //     new_nsarray::<SCRunningApplication>()
                        // };

                        let excluded_apps = new_nsarray::<SCRunningApplication>();
                        SCContentFilter(
                            SCContentFilter::alloc()
                                .initWithDisplay_excludingApplications_exceptingWindows_(
                                    display,
                                    excluded_apps,
                                    new_nsarray::<SCWindow>(),
                                ),
                        )
                    } else {
                        panic!("No display selected.")
                    }
                }
                CaptureType::Window => {
                    if let Some(_window) = self.selected_window {
                        todo!()
                    } else {
                        panic!("No window selected.")
                    }
                }
            }
        }
    }

    fn stream_configuration(&self) -> SCStreamConfiguration {
        unsafe {
            let config = SCStreamConfiguration(SCStreamConfiguration::alloc().init());

            // Configure audio capture.
            config.setCapturesAudio_(self.is_audio_capture_enabled);
            config.setExcludesCurrentProcessAudio_(self.is_app_audio_excluded);

            match self.capture_type {
                CaptureType::Display => {
                    if let Some(display) = self.selected_display {
                        // Configure the display content width and height.
                        config.setWidth_(display.width() as usize * self.scale_factor);
                        config.setHeight_(display.height() as usize * self.scale_factor);
                    }
                }
                CaptureType::Window => {
                    if let Some(window) = self.selected_window {
                        // Configure the display content width and height.
                        config.setWidth_(window.frame().size.width as usize * 2);
                        config.setHeight_(window.frame().size.height as usize * 2);
                    }
                }
            }

            config.setMinimumFrameInterval_(CMTime {
                value: 1,
                timescale: self.max_fps as i32,
                flags: 0,
                epoch: 0,
            });

            // Increase the depth of the frame queue to ensure high fps at the expense of increasing
            // the memory footprint of WindowServer.
            config.setQueueDepth_(1);

            config
        }
    }

    fn update_engine(&mut self) {
        if !self.is_running {
            return;
        }
        unsafe {
            self.capture_engine
                .update(self.stream_configuration(), self.content_filter());
        }
    }

    async fn refresh_available_content(&mut self) {
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = barrier.clone();
        let results = Arc::new(std::sync::Mutex::new(None));
        let results_clone = results.clone();
        unsafe {
            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                false,
                true,
                objc_closure!(move |content: id, error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        panic!("Error getting shareable content: {}", error);

                    } else {
                        let available_content = SCShareableContent(content);
                        available_content.retain();
                        results_clone.lock().unwrap().replace(UnsafeSendable(available_content));
                        barrier_clone.wait();
                    }
                }),
            );
            barrier.wait();
            let available_content = results
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| panic!("Failed to get shareable content."))
                .0;
            let available_displays = from_nsarray!(SCDisplay, available_content.displays());
            let available_windows = ScreenRecorder::filter_windows(from_nsarray!(
                SCWindow,
                available_content.windows()
            ));
            let available_apps =
                from_nsarray!(SCRunningApplication, available_content.applications());
            let old_content = self.available_content.replace(available_content);

            self.selected_display = self
                .selected_display
                .map(
                    // Make sure the selected display is still available.
                    |selected_display| {
                        available_displays
                            .iter()
                            .find(|display| display.0 == selected_display.0)
                            .cloned()
                    },
                )
                .flatten()
                .or(available_displays.first().cloned());

            self.selected_window = self
                .selected_window
                .map(
                    // Make sure the selected window is still available.
                    |selected_window| {
                        available_windows
                            .iter()
                            .find(|window| window.0 == selected_window.0)
                            .cloned()
                    },
                )
                .flatten()
                .or(available_windows.first().cloned());

            self.available_displays = available_displays;
            self.available_windows = available_windows;
            self.available_apps = available_apps;

            old_content.map(|content| content.release());
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
            // TODO ignore self
            // Remove this app's window from the list.
            // .filter(|window| unsafe {
            //     let window_bundle = from_nsstring!(window.owningApplication().bundleIdentifier());
            //     let this_bundle = from_nsstring!(NSBundle::mainBundle().bundleIdentifier());
            //     window_bundle != this_bundle
            // })
            .collect()
    }
}
