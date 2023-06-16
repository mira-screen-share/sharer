use std::path::Path;

use clap::Parser;
use iced::{Application, clipboard, Command, executor, Length};
use iced::Alignment::Center;
use iced::widget::{row, vertical_space};

use crate::{column_iced, config};
use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::component::invite_info_card;
use crate::gui::message::Message;
use crate::gui::theme::button;
use crate::gui::theme::button::{Buildable, FAB, FilledButton};
use crate::gui::theme::Theme;
use crate::gui::theme::widget::Element;

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
                    return clipboard::write(invite_link);
                }
            }
            Message::CopyRoomID => {
                if let Some(room_id) = self.capturer.get_room_id() {
                    return clipboard::write(room_id);
                }
            }
            Message::CopyPasscode => {
                // TODO
            }
            Message::Ignore => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let is_sharing = self.capturer.is_running();
        let element: Element<Message> = row![
            if is_sharing {
                column_iced![
                    column_iced![
                        row![
                            invite_info_card(
                                "Room",
                                self.capturer.get_room_id().unwrap_or_default().as_str(),
                                Message::CopyRoomID,
                                156.,
                            ),
                            invite_info_card(
                                "Passcode",
                                "TODO",
                                Message::CopyPasscode,
                                156.,
                            ),
                        ].spacing(12),
                        invite_info_card(
                            "Invite Link",
                            self.capturer.get_invite_link().unwrap_or_default().as_str(),
                            Message::CopyInviteLink,
                            324.,
                        )
                    ].width(Length::Shrink)
                        .height(Length::Shrink)
                        .spacing(12),
                    vertical_space(Length::Fill),
                    FilledButton::new("End")
                        .icon("stop.svg")
                        .style(button::Style::Danger)
                        .build()
                        .on_press(Message::Stop),
                ]
                .height(Length::Fill)
            } else {
                column_iced![
                    FAB::new("Start Sharing", "play.svg")
                        .style(button::Style::Primary)
                        .build()
                        .on_press(Message::Start),
                ]
            }.align_items(Center)
                .width(Length::Fill)
                .padding(10)
                .spacing(12),
        ].align_items(Center)
            .height(Length::Fill)
            .padding(10)
            .into();

        // element.explain(Color::WHITE)
        element
    }

    fn theme(&self) -> Self::Theme { Theme::Dark }
}
