extern crate core;
#[macro_use]
extern crate log;

use iced::{Application, Settings};

use crate::capture::ScreenCapture;
use crate::gui::app::App;
use crate::output::OutputSink;
use crate::result::Result;

mod auth;
mod capture;
mod config;
mod encoder;
mod gui;
mod inputs;
mod output;
mod performance_profiler;
mod result;
mod signaller;

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .level_for("wgpu_core", log::LevelFilter::Warn)
        .level_for("wgpu_hal", log::LevelFilter::Warn)
        .level_for("iced_wgpu", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|_| {
            eprintln!("Failed to initialize logger");
        });
    App::run(Settings {
        window: iced::window::Settings {
            size: (640, 373),
            min_size: Some((400, 300)),
            icon: Some(
                iced::window::icon::from_file_data(
                    include_bytes!("../resources/icons/256x256.png"),
                    None,
                )
                .unwrap(),
            ),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
}
