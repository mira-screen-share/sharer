extern crate libc;

use std::ffi::c_void;

use anyhow::anyhow;
use apple_sys::ScreenCaptureKit::{
    id, CGSize, CMTime, INSBundle, INSError, INSObject, INSProcessInfo, ISCContentFilter,
    ISCDisplay, ISCRunningApplication, ISCShareableContent, ISCStreamConfiguration, ISCWindow,
    NSArray, NSBundle, NSError, NSProcessInfo, NSString_NSStringDeprecated, PNSObject,
    SCContentFilter, SCDisplay, SCRunningApplication, SCShareableContent, SCStreamConfiguration,
    SCWindow,
};
use block::Block;
use itertools::Itertools;

use crate::capture::display::DisplaySelector;
use crate::capture::macos::capture_engine::CaptureEngine;
use crate::capture::macos::display::Display;
use crate::capture::macos::ffi::{
    from_nsarray, from_nsstring, new_nsarray, objc_closure, FromNSArray, ToNSArray, UnsafeSendable,
};
use crate::capture::macos::pcm_buffer::PCMBuffer;
use crate::capture::{DisplayInfo, YUVFrame};
use crate::result::Result;

#[allow(dead_code)]
enum CaptureType {
    Display,
    Window,
}

#[allow(dead_code)]
pub struct ScreenRecorder {
    is_running: bool,
    capture_type: CaptureType,
    selected_display: Option<Display>,
    selected_window: Option<SCWindow>,
    is_app_excluded: bool,
    content_size: CGSize,
    max_fps: u8,
    available_content: Option<SCShareableContent>,
    available_apps: Vec<SCRunningApplication>,
    available_displays: Vec<Display>,
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

    pub fn can_record() -> bool {
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        unsafe {
            #[cfg(target_arch = "x86_64")]
            let exclude_desktop_windows = 0;
            #[cfg(target_arch = "x86_64")]
            let on_screen_windows_only = 1;

            #[cfg(target_arch = "aarch64")]
            let exclude_desktop_windows = false;
            #[cfg(target_arch = "aarch64")]
            let on_screen_windows_only = true;

            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                exclude_desktop_windows,
                on_screen_windows_only,
                objc_closure!(move |_content: id, error: id| {
                    let result = error.is_null();
                    tx.send(result).unwrap();
                }),
            );
        }
        rx.recv().unwrap()
    }

    pub fn monitor_available_content(&mut self) {
        if self.is_setup {
            return;
        }
        self.refresh_available_content();
    }

    /// Starts capturing screen content.
    pub fn start(
        &mut self,
        video_tx: tokio::sync::mpsc::Sender<YUVFrame>,
        audio_tx: tokio::sync::mpsc::Sender<PCMBuffer>,
    ) {
        // Exit early if already running.
        if self.is_running {
            return;
        }

        if !self.is_setup {
            // Starting polling for available screen content.
            self.monitor_available_content();
            self.is_setup = true;
        }

        self.is_running = true;

        unsafe {
            self.capture_engine.start_capture(
                self.stream_configuration(),
                self.content_filter(),
                video_tx,
                if self.is_audio_capture_enabled {
                    Some(audio_tx)
                } else {
                    None
                },
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
                    if let Some(display) = &self.selected_display {
                        info!("Capturing display: {}", display.sc_display.displayID());

                        // Exclude the Sharer app itself.
                        let excluded_apps: NSArray = if self.is_app_excluded {
                            self.available_apps
                                .clone()
                                .into_iter()
                                .filter(|app| match NSBundle::mainBundle().bundleIdentifier() {
                                    this_bundle if this_bundle.0.is_null() => {
                                        let app_name = from_nsstring!(app.applicationName());
                                        let this_name = {
                                            from_nsstring!(
                                                NSProcessInfo::processInfo().processName()
                                            )
                                        };
                                        app_name == this_name
                                    }
                                    bundle => {
                                        let app_bundle = from_nsstring!(app.bundleIdentifier());
                                        let this_bundle = from_nsstring!(bundle);
                                        app_bundle == this_bundle
                                    }
                                })
                                .collect::<Vec<SCRunningApplication>>()
                                .to_nsarray()
                        } else {
                            new_nsarray::<SCRunningApplication>()
                        };

                        SCContentFilter(
                            SCContentFilter::alloc()
                                .initWithDisplay_excludingApplications_exceptingWindows_(
                                    display.sc_display,
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
            #[cfg(target_arch = "x86_64")]
            let is_audio_capture_enabled = if self.is_audio_capture_enabled { 1 } else { 0 };
            #[cfg(target_arch = "x86_64")]
            let is_app_audio_excluded = if self.is_app_audio_excluded { 1 } else { 0 };

            #[cfg(target_arch = "aarch64")]
            let is_audio_capture_enabled = self.is_audio_capture_enabled;
            #[cfg(target_arch = "aarch64")]
            let is_app_audio_excluded = self.is_app_audio_excluded;

            config.setCapturesAudio_(is_audio_capture_enabled);
            config.setExcludesCurrentProcessAudio_(is_app_audio_excluded);

            let (width, height) = self.resolution();
            config.setWidth_(width as _);
            config.setHeight_(height as _);

            config.setMinimumFrameInterval_(CMTime {
                value: 1,
                timescale: self.max_fps as i32,
                flags: 0,
                epoch: 0,
            });

            // Increase the depth of the frame queue to ensure high fps at the expense of increasing
            // the memory footprint of WindowServer.
            config.setQueueDepth_(3);

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

    fn refresh_available_content(&mut self) {
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        unsafe {
            #[cfg(target_arch = "x86_64")]
            let exclude_desktop_windows = 0;
            #[cfg(target_arch = "x86_64")]
            let on_screen_windows_only = 1;

            #[cfg(target_arch = "aarch64")]
            let exclude_desktop_windows = false;
            #[cfg(target_arch = "aarch64")]
            let on_screen_windows_only = true;

            SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler_(
                exclude_desktop_windows,
                on_screen_windows_only,
                objc_closure!(move |content: id, error: id| {
                    if !error.is_null() {
                        let error = from_nsstring!(NSError(error).localizedDescription());
                        panic!("Error getting shareable content: {}", error);
                    } else {
                        let available_content = SCShareableContent(content);
                        available_content.retain();
                        result_tx.send(UnsafeSendable(available_content)).unwrap();
                    }
                }),
            );
            let available_content = result_rx.recv().unwrap().0;
            let available_displays = from_nsarray!(SCDisplay, available_content.displays());
            let available_windows = ScreenRecorder::filter_windows(from_nsarray!(
                SCWindow,
                available_content.windows()
            ));
            let available_apps =
                from_nsarray!(SCRunningApplication, available_content.applications());

            // Release later.
            let old_content = self.available_content.replace(available_content);

            self.available_displays = available_displays
                .iter()
                .map(|display| Display::new(display.clone()))
                .collect();
            self.available_windows = available_windows;
            self.available_apps = available_apps;

            self.selected_display = self
                .selected_display
                .as_ref()
                .map(
                    // Use the currently selected display if it is still available.
                    |selected_display| {
                        self.available_displays
                            .iter()
                            .find(|display| display == &selected_display)
                            .cloned()
                    },
                )
                .flatten()
                .or(self.available_displays.first().cloned());

            self.selected_window = self
                .selected_window
                .map(
                    // Use the currently selected window if it is still available.
                    |selected_window| {
                        self.available_windows
                            .iter()
                            .find(|window| window.windowID() == selected_window.windowID())
                            .cloned()
                    },
                )
                .flatten()
                .or(self.available_windows.first().cloned());

            if let Some(old_content) = old_content {
                old_content.release();
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
                match NSBundle::mainBundle().bundleIdentifier() {
                    this_bundle if this_bundle.0.is_null() => {
                        let window_name =
                            from_nsstring!(window.owningApplication().applicationName());
                        let this_name = from_nsstring!(NSProcessInfo::processInfo().processName());
                        window_name != this_name
                    }
                    bundle => {
                        let this_bundle = from_nsstring!(bundle);
                        let window_bundle =
                            from_nsstring!(window.owningApplication().bundleIdentifier());
                        window_bundle != this_bundle
                    }
                }
            })
            .collect()
    }
}

impl DisplayInfo for ScreenRecorder {
    fn resolution(&self) -> (u32, u32) {
        match self.capture_type {
            CaptureType::Display => self
                .selected_display
                .as_ref()
                .unwrap_or_else(|| panic!("No display is selected."))
                .resolution(),
            CaptureType::Window => {
                if let Some(window) = self.selected_window {
                    unsafe {
                        (
                            window.frame().size.width as u32 * self.dpi_conversion_factor() as u32,
                            window.frame().size.height as u32 * self.dpi_conversion_factor() as u32,
                        )
                    }
                } else {
                    panic!("No window is selected.")
                }
            }
        }
    }

    fn dpi_conversion_factor(&self) -> f64 {
        match self.capture_type {
            CaptureType::Display => self
                .selected_display
                .as_ref()
                .unwrap_or_else(|| panic!("No display is selected."))
                .dpi_conversion_factor(),
            CaptureType::Window => 2.0,
        }
    }
}

impl DisplaySelector for ScreenRecorder {
    type Display = Display;

    fn available_displays(&mut self) -> Result<Vec<Self::Display>> {
        self.refresh_available_content();
        Ok(self.available_displays.clone())
    }

    fn select_display(&mut self, display: &Self::Display) -> Result<()> {
        match self.available_displays.iter().find(|d| d == &display) {
            Some(display) => {
                self.selected_display = Some(display.clone());
                Ok(())
            }
            None => Err(anyhow!("Display is not available.")),
        }
    }

    fn selected_display(&self) -> Result<Option<Self::Display>> {
        Ok(self.selected_display.clone())
    }
}
