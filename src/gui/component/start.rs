use iced::Alignment::Center;
use iced::Length::Fill;

use crate::capture::capturer::Capturer;
use crate::column_iced;
use crate::gui::app;
use crate::gui::component::Component;
use crate::gui::theme::button;
use crate::gui::theme::button::FAB;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::widget::Element;

pub struct StartPage {}

#[derive(Clone, Debug)]
pub enum Message {
    Start,
}

impl From<Message> for app::Message {
    fn from(message: Message) -> Self {
        app::Message::Start(message)
    }
}

pub struct UpdateProps<'a> {
    pub capturer: &'a mut Capturer,
}

impl<'a> Component<'a> for StartPage {
    type Message = Message;
    type UpdateProps = UpdateProps<'a>;
    type ViewProps = ();

    fn update(&mut self, message: Self::Message, props: Self::UpdateProps) -> iced::Command<app::Message> {
        match message {
            Message::Start => {
                props.capturer.run();
            }
        }
        iced::Command::none()
    }

    fn view(&self, _params: Self::ViewProps) -> Element<'_, app::Message> {
        column_iced![
            FAB::new("Start Sharing", Icon::PlayCircle)
                .style(button::Style::Primary)
                .build()
                .on_press(Message::Start.into()),
        ].align_items(Center)
            .padding(16)
            .width(Fill)
            .into()
    }
}
