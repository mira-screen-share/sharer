use bytes::Bytes;
use enigo::{KeyboardControllable, MouseControllable};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MouseButton {
    Left,
    Right,
    Middle,
}

impl Into<enigo::MouseButton> for MouseButton {
    fn into(self) -> enigo::MouseButton {
        match self {
            MouseButton::Left => enigo::MouseButton::Left,
            MouseButton::Right => enigo::MouseButton::Right,
            MouseButton::Middle => enigo::MouseButton::Middle,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InputMessage {
    KeyDown { key: u16 },
    KeyUp { key: u16 },
    MouseMove { x: i32, y: i32 },
    MouseDown { x: i32, y: i32, button: MouseButton },
    MouseUp { x: i32, y: i32, button: MouseButton },
    MouseWheel { x: i32, y: i32, dx: i32, dy: i32 },
}

pub struct InputHandler {
    pub sender: mpsc::Sender<Bytes>,
}

impl InputHandler {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Bytes>(32);
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                let mut enigo = enigo::Enigo::new(); // todo:fixme
                let input_msg = serde_json::from_slice::<InputMessage>(&msg).unwrap();
                debug!("Deserialized input message: {:#?}", input_msg);
                match input_msg {
                    InputMessage::KeyDown { key } => enigo.key_down(enigo::Key::Raw(key)),
                    InputMessage::KeyUp { key } => enigo.key_up(enigo::Key::Raw(key)),
                    InputMessage::MouseMove { x, y } => enigo.mouse_move_to(x, y),
                    InputMessage::MouseDown { x, y, button } => {
                        enigo.mouse_move_to(x, y);
                        enigo.mouse_down(button.into())
                    }
                    InputMessage::MouseUp { x, y, button } => {
                        enigo.mouse_move_to(x, y);
                        enigo.mouse_up(button.into())
                    }
                    InputMessage::MouseWheel { x, y, dx, dy } => {
                        enigo.mouse_move_to(x, y);
                        enigo.mouse_scroll_y(dy);
                        enigo.mouse_scroll_x(dx);
                    }
                };
            }
        });
        Self { sender }
    }
}
