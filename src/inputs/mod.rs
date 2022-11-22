use crate::Result;
use bytes::Bytes;
use enigo::{KeyboardControllable, MouseControllable};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

mod parse_key;
use crate::inputs::parse_key::KeyOrSequence;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<MouseButton> for enigo::MouseButton {
    fn from(btn: MouseButton) -> enigo::MouseButton {
        match btn {
            MouseButton::Left => enigo::MouseButton::Left,
            MouseButton::Right => enigo::MouseButton::Right,
            MouseButton::Middle => enigo::MouseButton::Middle,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InputMessage {
    KeyDown { key: String }, // Key from KeyboardEvent.key
    KeyUp { key: String },
    MouseMove { x: i32, y: i32 },
    MouseDown { x: i32, y: i32, button: MouseButton },
    MouseUp { x: i32, y: i32, button: MouseButton },
    MouseWheel { x: i32, y: i32, dx: i32, dy: i32 },
}

pub struct InputHandler {
    pub sender: mpsc::Sender<Bytes>,
}

impl InputHandler {
    fn handle_input_event(input_msg: Bytes) -> Result<()> {
        let mut enigo = enigo::Enigo::new();
        let input_msg = serde_json::from_slice::<InputMessage>(&input_msg)?;
        debug!("Deserialized input message: {:#?}", input_msg);
        match input_msg {
            InputMessage::KeyDown { key } => match KeyOrSequence::from_js_key(&key)? {
                KeyOrSequence::Key(key) => enigo.key_down(key),
                KeyOrSequence::Sequence(c) => enigo.key_sequence(&c.to_string()),
            },
            InputMessage::KeyUp { key } => match KeyOrSequence::from_js_key(&key)? {
                KeyOrSequence::Key(key) => enigo.key_up(key),
                KeyOrSequence::Sequence(_c) => {}
            },
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
        Ok(())
    }

    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Bytes>(32);
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                if let Err(err) = Self::handle_input_event(msg) {
                    warn!("Error handling input event: {}", err);
                }
            }
        });
        Self { sender }
    }
}
