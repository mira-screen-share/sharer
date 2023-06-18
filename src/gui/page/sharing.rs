use std::sync::Arc;

use iced::{clipboard, Command};
use iced::Alignment::Center;
use iced::Length::{Fill, Shrink};
use iced::widget::row;
use tokio::sync::Mutex;

use crate::capture::capturer::Capturer;
use crate::column_iced;
use crate::gui::app;
use crate::gui::component::invite_info_card;
use crate::gui::page::Page;
use crate::gui::theme::button;
use crate::gui::theme::button::FilledButton;
use crate::gui::theme::widget::Element;

#[derive(Default, Clone, Debug)]
pub enum Tab {
    #[default]
    Invite,
    Viewers,
}

pub struct SharingPage {
    capturer: Arc<Mutex<Capturer>>,
    current_tab: Tab,
}

impl SharingPage {
    pub fn new(capturer: Arc<Mutex<Capturer>>) -> Self {
        Self {
            capturer,
            current_tab: Default::default(),
        }
    }
}

pub struct Props {
    pub room_id: String,
    pub invite_link: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    Stop,
    CopyRoomID,
    CopyPasscode,
    CopyInviteLink,
    ChangeTab(Tab),
}

impl From<Message> for app::Message {
    fn from(message: Message) -> Self {
        app::Message::Sharing(message)
    }
}

impl Page for SharingPage {
    type Message = Message;
    type Props = Props;

    fn update(&mut self, message: Message) -> Command<app::Message> {
        match message {
            Message::CopyInviteLink => {
                if let Ok(capturer) = self.capturer.try_lock() {
                    if let Some(invite_link) = capturer.get_invite_link() {
                        return clipboard::write(invite_link);
                    }
                }
            }
            Message::CopyRoomID => {
                if let Ok(capturer) = self.capturer.try_lock() {
                    if let Some(room_id) = capturer.get_room_id() {
                        return clipboard::write(room_id);
                    }
                }
            }
            Message::CopyPasscode => {
                // TODO
            }
            Message::Stop => {
                if let Ok(mut capturer) = self.capturer.try_lock() {
                    capturer.shutdown();
                }
            }
            Message::ChangeTab(tab) => {
                self.current_tab = tab;
            }
        }
        Command::none()
    }

    fn view<'a>(&self, props: Props) -> Element<'a, app::Message> {
        column_iced![
            match self.current_tab {
                Tab::Invite => invite_page(props.room_id, props.invite_link),
                Tab::Viewers => viewers_page(),
            },
            row![
                FilledButton::new("End")
                    .icon("stop.svg")
                    .style(button::Style::Danger)
                    .build()
                    .on_press(Message::Stop.into()),
            ]
        ].align_items(Center)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn viewers_page<'a>() -> Element<'a, app::Message> {
    column_iced![].into()
}

fn invite_page<'a>(
    room_id: String,
    invite_link: String,
) -> Element<'a, app::Message> {
    column_iced![
        row![
            invite_info_card(
                "Room",
                room_id.as_str(),
                Message::CopyRoomID.into(),
                156.,
            ),
            invite_info_card(
                "Passcode",
                "TODO",
                Message::CopyPasscode.into(),
                156.,
            ),
        ].spacing(12),
        invite_info_card(
            "Invite Link",
            invite_link.as_str(),
            Message::CopyInviteLink.into(),
            324.,
        )
    ].width(Shrink)
        .height(Fill)
        .align_items(Center)
        .spacing(12)
        .into()
}
