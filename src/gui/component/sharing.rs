use std::sync::Arc;

use iced::alignment::Horizontal;
use iced::widget::{container, horizontal_space, row, scrollable, text_input, vertical_space};
use iced::Alignment::Center;
use iced::Length::{Fill, Shrink};
use iced::{clipboard, Command};
use iced_aw::TabLabel;

use crate::auth::{ViewerIdentifier, ViewerManager};
use crate::capture::capturer::Capturer;
use crate::column_iced;
use crate::gui::component::avatar::text_avatar;
use crate::gui::component::Component;
use crate::gui::theme::button::{FilledButton, IconButton};
use crate::gui::theme::icon::Icon;
use crate::gui::theme::tab::Tab;
use crate::gui::theme::text;
use crate::gui::theme::text::text;
use crate::gui::theme::widget::Element;
use crate::gui::theme::widget::Tabs;
use crate::gui::theme::{button, PaletteColor};
use crate::gui::{app, resource};

pub struct SharingPage {
    current_tab: usize,
    invite_tab: InviteTab,
    viewers_tab: ViewersTab,
}

impl SharingPage {
    pub fn new() -> Self {
        Self {
            current_tab: Default::default(),
            invite_tab: InviteTab {},
            viewers_tab: ViewersTab {},
        }
    }
}

pub struct InviteTab {}

pub struct ViewersTab {}

pub struct UpdateProps<'a> {
    pub capturer: &'a mut Capturer,
    pub viewer_manager: Arc<ViewerManager>,
}

#[derive(Clone, Debug)]
pub struct ViewProps {
    pub room_id: String,
    pub room_password: String,
    pub invite_link: String,
    pub viewing_viewers: Vec<ViewerIdentifier>,
    pub pending_viewers: Vec<ViewerIdentifier>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Stop,
    CopyRoomID,
    CopyPasscode,
    CopyInviteLink,
    ChangeTab(usize),
    DeclineJoin(ViewerIdentifier),
    AcceptJoin(ViewerIdentifier),
    KickViewer(ViewerIdentifier),
}

impl From<Message> for app::Message {
    fn from(message: Message) -> Self {
        app::Message::Sharing(message)
    }
}

impl<'a> Component<'a> for SharingPage {
    type Message = Message;
    type UpdateProps = UpdateProps<'a>;
    type ViewProps = ViewProps;

    fn update(
        &mut self,
        message: Self::Message,
        props: Self::UpdateProps,
    ) -> Command<app::Message> {
        match message {
            Message::CopyInviteLink => {
                if let Some(invite_link) = props.capturer.get_invite_link() {
                    return clipboard::write(invite_link);
                }
            }
            Message::CopyRoomID => {
                if let Some(room_id) = props.capturer.get_room_id() {
                    return clipboard::write(room_id);
                }
            }
            Message::CopyPasscode => {
                if let Some(room_password) = props.capturer.get_room_password() {
                    return clipboard::write(room_password);
                }
            }
            Message::Stop => {
                props.capturer.shutdown();
            }
            Message::ChangeTab(tab) => {
                self.current_tab = tab;
            }
            Message::DeclineJoin(viewer_id) => {
                let handle = tokio::runtime::Handle::current();
                let viewer_manager = props.viewer_manager.clone();
                tokio::task::block_in_place(move || {
                    handle.block_on(async move {
                        viewer_manager.decline_viewer(viewer_id).await;
                    })
                });
            }
            Message::AcceptJoin(viewer_id) => {
                let handle = tokio::runtime::Handle::current();
                let viewer_manager = props.viewer_manager.clone();
                tokio::task::block_in_place(move || {
                    handle.block_on(async move {
                        viewer_manager.permit_viewer(viewer_id).await;
                    })
                });
            }
            Message::KickViewer(viewer_id) => {
                let handle = tokio::runtime::Handle::current();
                let viewer_manager = props.viewer_manager.clone();
                tokio::task::block_in_place(move || {
                    handle.block_on(async move {
                        viewer_manager.kick_viewer(viewer_id).await;
                    })
                });
            }
        }
        Command::none()
    }

    fn view(&self, props: Self::ViewProps) -> Element<'_, app::Message> {
        column_iced![
            container(
                Tabs::new(self.current_tab, move |message| app::Message::Sharing(
                    Message::ChangeTab(message)
                ))
                .push(
                    self.invite_tab.tab_label(),
                    self.invite_tab.view(props.clone())
                )
                .push(self.viewers_tab.tab_label(), self.viewers_tab.view(props))
                .tab_bar_style(Default::default())
                .icon_font(resource::font::ICON)
                .text_font(resource::font::BARLOW)
                .tab_bar_position(iced_aw::TabBarPosition::Top)
            )
            .height(Fill)
            .width(Fill),
            action_bar(),
        ]
        .align_items(Center)
        .width(Fill)
        .height(Fill)
        .into()
    }
}

fn action_bar<'a>() -> Element<'a, app::Message> {
    row![FilledButton::new("End")
        .icon(Icon::StopCircle)
        .style(button::Style::Danger)
        .build()
        .on_press(Message::Stop.into()),]
    .padding([0, 16, 16, 16])
    .into()
}

impl Tab for InviteTab {
    type Message = app::Message;
    type Props = ViewProps;

    fn title(&self) -> String {
        String::from("Invite")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::IconText(Icon::Link.into(), self.title())
    }

    fn content(&self, props: Self::Props) -> Element<'_, app::Message> {
        scrollable(
            column_iced![
                row![
                    invite_info_card(
                        "Room",
                        props.room_id.as_str(),
                        Message::CopyRoomID.into(),
                        156.,
                    ),
                    invite_info_card(
                        "Passcode",
                        props.room_password.as_str(),
                        Message::CopyPasscode.into(),
                        156.,
                    ),
                ]
                .spacing(12),
                invite_info_card(
                    "Invite Link",
                    props.invite_link.as_str(),
                    Message::CopyInviteLink.into(),
                    324.,
                )
            ]
            .width(Shrink)
            .align_items(Center)
            .spacing(12)
            .padding([0, 16]),
        )
        .height(Fill)
        .into()
    }
}

fn invite_info_card<'a>(
    head: &str,
    body: &str,
    on_copy: app::Message,
    width: f32,
) -> Element<'a, app::Message> {
    container(
        row![
            column_iced![
                text(head).size(14).width(iced::Length::Shrink),
                vertical_space(6),
                text_input("", body)
                    .style(crate::gui::theme::text_input::Style::Selectable)
                    .size(18)
                    .font(resource::font::BARLOW)
                    .on_input(move |_| { app::Message::Ignore })
                    .width(iced::Length::Fill)
                    .padding(0)
            ]
            .width(iced::Length::Fixed(width - 72.)),
            IconButton::new(Icon::ContentCopy).build().on_press(on_copy)
        ]
        .align_items(Center)
        .spacing(8)
        .padding([16, 8, 16, 16]),
    )
    .style(crate::gui::theme::container::Style::OutlinedCard)
    .width(width)
    .into()
}

impl Tab for ViewersTab {
    type Message = app::Message;
    type Props = ViewProps;

    fn title(&self) -> String {
        String::from("Viewers")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::IconText(Icon::Group.into(), self.title())
    }

    fn content(&self, props: Self::Props) -> Element<'_, app::Message> {
        let mut column = vec![];
        if !props.pending_viewers.is_empty() {
            column.push(text("Pending").size(16).style(text::Style::Label).into());
            for pending in props.pending_viewers.iter() {
                column.push(pending_viewer_cell(pending));
            }
        }
        if !props.viewing_viewers.is_empty() {
            if !column.is_empty() {
                column.push(vertical_space(2).into());
            }
            column.push(text("Viewing").size(16).style(text::Style::Label).into());
            for viewing in props.viewing_viewers.iter() {
                column.push(viewing_viewer_cell(viewing));
            }
        }

        scrollable(
            container(
                iced::widget::Column::with_children(column)
                    .width(Fill)
                    .max_width(400)
                    .spacing(16)
                    .padding([0, 24]),
            )
            .width(Fill)
            .align_x(Horizontal::Center),
        )
        .height(Fill)
        .into()
    }
}

fn viewing_viewer_cell<'a>(viewer: &ViewerIdentifier) -> Element<'a, app::Message> {
    row![
        text_avatar(PaletteColor::Primary, viewer.name.chars().next().unwrap()),
        horizontal_space(16),
        text(viewer.name.clone()).width(Fill),
        horizontal_space(16),
        IconButton::new(Icon::PersonRemove)
            .style(button::Style::Danger)
            .build()
            .on_press(Message::KickViewer(viewer.clone()).into()),
    ]
    .align_items(Center)
    .into()
}

fn pending_viewer_cell<'a>(viewer: &ViewerIdentifier) -> Element<'a, app::Message> {
    row![
        text_avatar(PaletteColor::Primary, viewer.name.chars().next().unwrap()),
        horizontal_space(16),
        text(viewer.name.clone()).width(Fill),
        horizontal_space(16),
        IconButton::new(Icon::Done)
            .style(button::Style::Success)
            .filled(true)
            .build()
            .on_press(Message::AcceptJoin(viewer.clone()).into()),
        horizontal_space(8),
        IconButton::new(Icon::Close)
            .style(button::Style::Danger)
            .filled(true)
            .build()
            .on_press(Message::DeclineJoin(viewer.clone()).into()),
    ]
    .align_items(Center)
    .into()
}
