use std::path::Path;

use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use iced::{Application, Background, Color, Command, Element, executor, Theme};
use iced::theme::{Button, Palette};
use iced::widget::{button, row, text, text_input};
use iced::widget::button::Appearance;

use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::config;
use crate::gui::macros::column_iced;

#[derive(Clone, Debug)]
pub enum Message {
    Start,
    Stop,
    SetMaxFps(String),
    SetDisplay(String),
    CopyInviteLink,
}

pub struct App {
    capturer: Capturer,
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
                capturer: Capturer::new(args, config),
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
                self.capturer.run();
            }
            Message::Stop => {
                self.capturer.shutdown();
            }
            Message::SetMaxFps(value) => {
                if let Ok(value) = value.parse::<u32>() {
                    self.capturer.config.max_fps = value;
                }
            }
            Message::SetDisplay(value) => {
                if let Ok(value) = value.parse::<usize>() {
                    self.capturer.args.display = value;
                }
            }
            Message::CopyInviteLink => {
                if let Some(invite_link) = self.capturer.get_invite_link() {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                    ctx.set_contents(invite_link).unwrap();
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let is_sharing = self.capturer.is_running();

        let a: Element<Message> = row![
            // input("Display", &self.capturer.args.display.to_string(), Message::SetDisplay),
            // input("Max FPS", &self.capturer.config.max_fps.to_string(), Message::SetMaxFps),
            column_iced![
                button(
                    text(
                        if is_sharing {
                            "Stop Sharing"
                        } else {
                            "Start Sharing"
                        })
                    .size(16),
                ).style(
                    iced::theme::Button::Custom(Box::new(MaterialButton::new(
                        if is_sharing {
                            iced::theme::Button::Destructive
                        } else {
                            iced::theme::Button::Primary
                        }
                    )))
                )
                .padding([10, 24, 10, 24])
                .on_press(
                    if is_sharing {
                        Message::Stop
                    } else {
                        Message::Start
                    }
                ),
                if is_sharing {
                    Element::from(
                        button(
                            text(
                                "Copy invite link"
                            ).size(16),
                        ).style(
                            iced::theme::Button::Custom(Box::new(MaterialButton::new(
                                iced::theme::Button::Primary
                            )))
                        )
                        .padding([10, 24, 10, 24])
                        .on_press(Message::CopyInviteLink)
                    )
                } else {
                    Element::from(row![])
                }
            ]
            .align_items(iced::Alignment::Center)
            .width(iced::Length::Fill)
            .padding(10)
            .spacing(10),
        ]
            .align_items(iced::Alignment::Center)
            .height(iced::Length::Fill)
            .padding(10)
            .into();
        // a.explain(Color::WHITE)
        a
    }

    fn theme(&self) -> Self::Theme {
        Theme::Custom(
            Box::from(iced::theme::Custom::new(
                Palette {
                    background: Color::from_rgb(0.2, 0.2, 0.2),
                    primary: Color::from_rgb(0.0, 0.5, 0.5),
                    ..Palette::DARK
                }
            ))
        )
    }
}

fn input<'a>(
    name: &str,
    value: &str,
    message: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(format!("{}: ", name)),
        text_input(
            name,
            value,
        ).on_input(move |value| {
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

struct MaterialButton {
    style: Button,
}

impl MaterialButton {
    pub fn new(style: Button) -> Self {
        Self {
            style
        }
    }
}

impl button::StyleSheet for MaterialButton {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let appearance = Appearance {
            border_radius: 32.0,
            border_width: 0.0,
            ..Appearance::default()
        };

        let from_pair = |pair: iced::theme::palette::Pair| Appearance {
            background: Some(pair.color.into()),
            text_color: pair.text,
            ..appearance
        };

        let palette = style.extended_palette();

        match self.style {
            Button::Primary => from_pair(palette.primary.strong),
            Button::Secondary => from_pair(palette.secondary.base),
            Button::Positive => from_pair(palette.success.base),
            Button::Destructive => from_pair(palette.danger.base),
            Button::Text | Button::Custom(_) => Appearance {
                text_color: palette.background.base.text,
                ..appearance
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let active = self.active(style);
        Appearance {
            background: active.background.map(|background| match background {
                Background::Color(color) => Background::Color(Color {
                    r: color.r - 0.1,
                    g: color.g - 0.1,
                    b: color.b - 0.1,
                    a: color.a,
                }),
            }),
            ..active
        }
    }
}
