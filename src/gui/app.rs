use std::path::Path;
use std::sync::{Arc, Mutex};

use clap::Parser;
use iced::widget::{row, vertical_space};
use iced::Alignment::Center;
use iced::{clipboard, executor, Application, Command, Length};

use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::component::invite_info_card;
use crate::gui::message::Message;
use crate::gui::theme::button;
use crate::gui::theme::button::{Buildable, FilledButton, FAB};
use crate::gui::theme::widget::Element;
use crate::gui::theme::Theme;
use crate::{column_iced, config};

pub struct App {
    capturer: Arc<Mutex<Option<Capturer>>>, // late inited
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;

    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let args = capturer::Args::parse();
        let config = config::load(Path::new(&args.config)).unwrap();
        let app = App {
            capturer: Arc::new(Mutex::new(None)),
        };
        let capturer_clone = app.capturer.clone();
        tokio::spawn(async move {
            let capturer = Capturer::new(args, config).await;
            capturer_clone.lock().unwrap().replace(capturer);
        });
        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("Mira Sharer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        let mut capturer = self.capturer.lock().unwrap();

        if capturer.is_none() {
            return Command::none();
        }

        let capturer = capturer.as_mut().unwrap();
        match message {
            Message::Start => {
                capturer.run();
            }
            Message::Stop => {
                capturer.shutdown();
            }
            Message::SetMaxFps(value) => {
                if let Ok(value) = value.parse::<u32>() {
                    capturer.config.max_fps = value;
                }
            }
            Message::SetDisplay(value) => {
                if let Ok(value) = value.parse::<usize>() {
                    capturer.args.display = value;
                }
            }
            Message::CopyInviteLink => {
                if let Some(invite_link) = capturer.get_invite_link() {
                    return clipboard::write(invite_link);
                }
            }
            Message::CopyRoomID => {
                if let Some(room_id) = capturer.get_room_id() {
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
        let is_sharing = self
            .capturer
            .lock()
            .unwrap()
            .as_ref()
            .map_or(false, |c| c.is_running());

        let element: Element<Message> = row![if is_sharing {
            let (room_id, invite_link) = {
                let capturer = self.capturer.lock().unwrap();
                let capturer_ref = capturer.as_ref().unwrap();
                (
                    capturer_ref.get_room_id().unwrap_or_default(),
                    capturer_ref.get_invite_link().unwrap_or_default(),
                )
            };
            column_iced![
                column_iced![
                    row![
                        invite_info_card("Room", room_id.as_str(), Message::CopyRoomID, 156.,),
                        invite_info_card("Passcode", "TODO", Message::CopyPasscode, 156.,),
                    ]
                    .spacing(12),
                    invite_info_card(
                        "Invite Link",
                        invite_link.as_str(),
                        Message::CopyInviteLink,
                        324.,
                    )
                ]
                .width(Length::Shrink)
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
            column_iced![FAB::new("Start Sharing", "play.svg")
                .style(button::Style::Primary)
                .build()
                .on_press(Message::Start),]
        }
        .align_items(Center)
        .width(Length::Fill)
        .padding(10)
        .spacing(12),]
        .align_items(Center)
        .height(Length::Fill)
        .padding(10)
        .into();

        // element.explain(Color::WHITE)
        element
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}
