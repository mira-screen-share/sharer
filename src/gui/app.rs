use std::path::Path;

use clap::Parser;
use iced::{
    Application, Command, Element, executor, Theme,
};
use iced::widget::{button, row, text, text_input};

use crate::capture::capturer;
use crate::config;
use crate::config::Config;
use crate::gui::macros::column_iced;

#[derive(Clone, Debug)]
pub enum Message {
    Start,
    Stop,
    SetMaxFps(String),
    SetDisplay(String),
}

pub struct App {
    args: capturer::Args,
    config: Config,
    capturer_thread: Option<tokio::task::JoinHandle<()>>,
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;

    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let args = capturer::Args::parse();
        let config = config::load(Path::new(&args.config)).unwrap();
        (
            App {
                args,
                config,
                capturer_thread: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Mira Sharer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Start => {
                let args = self.args.clone();
                let config = self.config.clone();
                self.capturer_thread = Some(tokio::spawn(async move {
                    capturer::start_capture(args, config).await.unwrap();
                }));
            }
            Message::Stop => {
                if let Some(thread) = self.capturer_thread.take() {
                    thread.abort();
                }
            }
            Message::SetMaxFps(value) => {
                if let Ok(value) = value.parse::<u32>() {
                    self.config.max_fps = value;
                }
            }
            Message::SetDisplay(value) => {
                if let Ok(value) = value.parse::<usize>() {
                    self.args.display = value;
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let is_sharing = self.capturer_thread.is_some();

        column_iced![
            input("Display", &self.args.display.to_string(), &Message::SetDisplay),
            input("Max FPS", &self.config.max_fps.to_string(), &Message::SetMaxFps),
            row![
                button(
                    if is_sharing {
                        "Stop"
                    } else {
                        "Start"
                    },
                ).on_press(
                    if is_sharing {
                        Message::Stop
                    } else {
                        Message::Start
                    }
                ),
            ]
            .padding(10)
            .spacing(10),
        ]
            .padding(10)
            .into()
    }
}

fn input<'a>(
    name: &str,
    value: &str,
    message: &'a dyn Fn(String) -> Message,
) -> Element<'a, Message> {
    row![
        text(format!("{}: ", name)),
        text_input(
            name,
            value,
        ).on_input(|value| {
            message(value)
        })
        .width(iced::Length::Fixed(100.)),
    ]
        .width(iced::Length::Shrink)
        .align_items(iced::Alignment::Center)
        .padding(10)
        .spacing(10)
        .into()
}
