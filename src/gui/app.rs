use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use futures_util::SinkExt;
use iced::widget::row;
use iced::Alignment::Center;
use iced::{executor, Application, Command, Length, Subscription};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::capture::capturer;
use crate::capture::capturer::Capturer;
use crate::gui::component::sharing::SharingPage;
use crate::gui::component::start::StartPage;
use crate::gui::component::{sharing, start, Component};
use crate::gui::theme::widget::Element;
use crate::gui::theme::Theme;
use crate::{column_iced, config};

pub struct App {
    capturer: Capturer,
    pub start_page: StartPage,
    pub sharing_page: SharingPage,
    intermediate_update_receiver: Option<Receiver<()>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Message {
    Start(start::Message),
    Sharing(sharing::Message),
    Ignore,
    UpdateChannel(Sender<()>),
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;

    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let args = capturer::Args::parse();
        let config = config::load(Path::new(&args.config)).unwrap();
        let (intermediate_update_sender, intermediate_update_receiver) = channel(10);
        let intermediate_update_sender = Box::leak(Box::new(intermediate_update_sender));
        (
            App {
                capturer: Capturer::new(
                    args,
                    config,
                    Arc::new(|| unsafe {
                        intermediate_update_sender.try_send(()).unwrap();
                    }),
                ),
                start_page: StartPage {},
                sharing_page: SharingPage::new(),
                intermediate_update_receiver: Some(intermediate_update_receiver),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Mira Sharer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        return match message {
            Message::Start(message) => self.start_page.update(
                message,
                start::UpdateProps {
                    capturer: &mut self.capturer,
                },
            ),
            Message::Sharing(message) => self.sharing_page.update(
                message,
                sharing::UpdateProps {
                    capturer: &mut self.capturer,
                },
            ),
            Message::Ignore => Command::none(),
            Message::UpdateChannel(channel) => {
                let mut receiver = self.intermediate_update_receiver.take().unwrap();
                tokio::spawn(async move {
                    while let Some(_) = receiver.recv().await {
                        channel.send(()).await.unwrap();
                    }
                });
                Command::none()
            }
        };
    }

    fn view(&self) -> Element<Message> {
        let is_sharing = self.capturer.is_running();
        let element: Element<Message> = row![column_iced![if is_sharing {
            self.sharing_page.view(sharing::ViewProps {
                room_id: self.capturer.get_room_id().unwrap_or_default(),
                invite_link: self.capturer.get_invite_link().unwrap_or_default(),
            })
        } else {
            self.start_page.view(())
        }]
        .spacing(12),]
        .align_items(Center)
        .height(Length::Fill)
        .into();
        element
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced::subscription::channel("updates", 10, |mut s| async move {
            let (sender, mut receiver) = channel(10);
            s.send(Message::UpdateChannel(sender)).await.unwrap();
            loop {
                let _ = receiver.recv().await;
                s.send(Message::Ignore).await.unwrap();
            }
        })
    }
}
