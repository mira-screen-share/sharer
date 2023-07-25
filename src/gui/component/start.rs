use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, pick_list, vertical_space};
use iced::Alignment;
use iced::Alignment::Center;
use iced::Length::Fill;

use crate::capture::capturer::Capturer;
use crate::capture::display::DisplaySelector;
use crate::capture::ScreenCaptureImpl;
use crate::column_iced;
use crate::gui::app;
use crate::gui::component::Component;
use crate::gui::theme::button;
use crate::gui::theme::button::FAB;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::text;
use crate::gui::theme::text::text;
use crate::gui::theme::widget::Element;

pub struct StartPage {}

#[derive(Clone, Debug)]
pub enum Message {
    Start,
    SelectDisplay(<ScreenCaptureImpl as DisplaySelector>::Display),
}

impl From<Message> for app::Message {
    fn from(message: Message) -> Self {
        app::Message::Start(message)
    }
}

pub struct UpdateProps<'a> {
    pub capturer: &'a mut Capturer,
}

pub struct ViewProps<'a> {
    pub capturer: &'a Capturer,
}

impl<'a> Component<'a> for StartPage {
    type Message = Message;
    type UpdateProps = UpdateProps<'a>;
    type ViewProps = ViewProps<'a>;

    fn update(
        &mut self,
        message: Self::Message,
        props: Self::UpdateProps,
    ) -> iced::Command<app::Message> {
        match message {
            Message::Start => {
                props.capturer.run();
            }
            Message::SelectDisplay(display) => {
                props.capturer.select_display(display);
            }
        }
        iced::Command::none()
    }

    fn view(&self, params: Self::ViewProps) -> Element<'_, app::Message> {
        container(
            column_iced![
                column_iced![
                    text("Display").size(16).style(text::Style::Label),
                    vertical_space(8),
                    pick_list(
                        params.capturer.available_displays(),
                        params.capturer.selected_display(),
                        move |message| app::Message::Start(Message::SelectDisplay(message))
                    )
                    .width(Fill),
                ]
                .align_items(Alignment::Start)
                .width(Fill),
                vertical_space(32),
                FAB::new("Start Sharing", Icon::PlayCircle)
                    .style(button::Style::Primary)
                    .build()
                    .on_press(Message::Start.into()),
            ]
            .align_items(Center)
            .padding(16)
            .width(Fill)
            .max_width(400),
        )
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
    }
}
