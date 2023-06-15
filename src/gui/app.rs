use std::path::Path;

use clap::Parser;
use iced::{Application, clipboard, Command, executor};
use iced::Alignment::Center;

use crate::{column_iced, config};
use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::message::Message;
use crate::gui::message::Message::Ignore;
use crate::gui::theme::button;
use crate::gui::theme::Theme;
use crate::gui::widget::Element;

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
        // let is_sharing = self.capturer.is_running();
        //
        // let element: Element<Message> = row![
        //     if is_sharing {
        //         column_iced![
        //             column_iced![
        //                 row![
        //                     material_card(
        //                         "Room",
        //                         self.capturer.get_room_id().unwrap_or_default().as_str(),
        //                         Fixed(156.),
        //                         Some(material_icon_button("copy.png", Message::CopyRoomID))
        //                     ),
        //                     material_card(
        //                         "Passcode",
        //                         "TODO",
        //                         Fixed(156.),
        //                         Some(material_icon_button("copy.png", Message::CopyPasscode))
        //                     ),
        //                 ].spacing(12),
        //                 material_card(
        //                     "Invite Link",
        //                     self.capturer.get_invite_link().unwrap_or_default().as_str(),
        //                     iced::Length::Fixed(324.),
        //                     Some(material_icon_button("copy.png", Message::CopyInviteLink))
        //                 )
        //             ].width(Shrink)
        //                 .height(Shrink)
        //                 .spacing(12),
        //             vertical_space(Fill),
        //             fab("End", Message::Stop, Button::Destructive, Some("stop.png"), [12, 20, 12, 20]),
        //         ]
        //         .height(Fill)
        //     } else {
        //         column_iced![
        //             fab("Start Sharing", Message::Start, Button::Primary, Some("play.png"), [18, 20, 18, 20])
        //         ]
        //     }.align_items(Center)
        //         .width(Fill)
        //         .padding(10)
        //         .spacing(12),
        // ].align_items(Center)
        //     .height(Fill)
        //     .padding(10)
        //     .into();

        // let element = filled_button::primary(text("123")).on_press(Ignore).into();

        let element: Element<Message> =
            column_iced![
                button::Filled::new("Filled Button")
                    .style(button::Style::Primary)
                    .into()
                    .on_press(Ignore),
                button::Filled::new("Filled Button")
                    .style(button::Style::Primary)
                    .icon("play.png")
                    .into()
                    .on_press(Ignore),
            ].spacing(16).align_items(Center).into();

        // element.explain(Color::WHITE)
        element
    }

    fn theme(&self) -> Self::Theme { Theme::Dark }
}
