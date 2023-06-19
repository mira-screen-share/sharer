use std::path::Path;
use std::sync::{Arc, Mutex};

use clap::Parser;
use iced::{Application, Command, executor, Length};
use iced::Alignment::Center;
use iced::widget::row;

use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::component::{Component, sharing, start};
use crate::gui::component::sharing::SharingPage;
use crate::gui::component::start::StartPage;
use crate::gui::theme::Theme;
use crate::{column_iced, config};
use crate::gui::theme::widget::Element;

pub struct App {
    capturer: Arc<Mutex<Option<Capturer>>>, // late inited
    pub start_page: StartPage,
    pub sharing_page: SharingPage,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Message {
    Start(start::Message),
    Sharing(sharing::Message),
    Ignore,
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
            start_page: StartPage {},
            sharing_page: SharingPage::new(),
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
        return match message {
            Message::Start(message) => self.start_page.update(message, start::UpdateProps {
                capturer: self.capturer.as_ref(),
            }),
            Message::Sharing(message) => self.sharing_page.update(message, sharing::UpdateProps {
                capturer: self.capturer.as_ref(),
            }),
            Message::Ignore => Command::none(),
        };
    }

    fn view(&self) -> Element<Message> {
        let is_sharing = self.capturer.is_running();
        let element: Element<Message> = row![
            column_iced![
                if is_sharing {
                    self.sharing_page.view(
                        sharing::ViewProps {
                            room_id: self.capturer.get_room_id().unwrap_or_default(),
                            invite_link: self.capturer.get_invite_link().unwrap_or_default(),
                        }
                    )
                } else {
                    self.start_page.view(())
                }
            ].spacing(12),
        ].align_items(Center)
            .height(Length::Fill)
            .into();
        element
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}
