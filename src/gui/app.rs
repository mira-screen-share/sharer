use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use iced::{Application, Command, executor, Length};
use iced::Alignment::Center;
use iced::widget::row;
use tokio::sync::Mutex;

use crate::{column_iced, config};
use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::page::{Page, sharing, start};
use crate::gui::page::sharing::SharingPage;
use crate::gui::page::start::StartPage;
use crate::gui::theme::Theme;
use crate::gui::theme::widget::Element;

pub struct App {
    pub capturer: Arc<Mutex<Capturer>>,
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
        let capturer = Arc::new(Mutex::new(Capturer::new(args, config)));
        (
            App {
                capturer: capturer.clone(),
                start_page: StartPage::new(capturer.clone()),
                sharing_page: SharingPage::new(capturer.clone()),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Mira Sharer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        return match message {
            Message::Start(message) => self.start_page.update(message),
            Message::Sharing(message) => self.sharing_page.update(message),
            Message::Ignore => Command::none(),
        };
    }

    fn view(&self) -> Element<Message> {
        if let Ok(capturer) = self.capturer.try_lock() {
            let is_sharing = capturer.is_running();
            let element: Element<Message> = row![
                column_iced![
                    if is_sharing {
                        self.sharing_page.view(
                            sharing::Props {
                                room_id: capturer.get_room_id().unwrap_or_default(),
                                invite_link: capturer.get_invite_link().unwrap_or_default(),
                            }
                        )
                    } else {
                        self.start_page.view(())
                    }
                ].padding(10)
                    .spacing(12),
            ].align_items(Center)
                .height(Length::Fill)
                .padding(10)
                .into();

            element.explain(iced::Color::WHITE)
            // element
        } else {
            row![].into()
        }
    }

    fn theme(&self) -> Self::Theme { Theme::Dark }
}
