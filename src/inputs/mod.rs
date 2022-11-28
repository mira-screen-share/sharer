use crate::Result;
use bytes::Bytes;
use enigo::{KeyboardControllable, MouseControllable};
use parse_key::FromJsKey;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

mod parse_key;

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
    KeyDown { key: String }, // Key from KeyboardEvent.code
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
    fn handle_input_event(input_msg: Bytes, dpi_factor: f64) -> Result<()> {
        let scroll_reverse_factor = if cfg!(target_os = "windows") { -1. } else { 1. };
        let mut enigo = enigo::Enigo::new();
        let input_msg = serde_json::from_slice::<InputMessage>(&input_msg)?;
        debug!("Deserialized input message: {:#?}", input_msg);
        match input_msg {
            InputMessage::KeyDown { key } => enigo.key_down(enigo::Key::from_js_key(&key)?),
            InputMessage::KeyUp { key } => enigo.key_up(enigo::Key::from_js_key(&key)?),
            InputMessage::MouseMove { x, y } => enigo.mouse_move_to(
                (x as f64 * dpi_factor) as i32, (y as f64 * dpi_factor) as i32
            ),
            InputMessage::MouseDown { x, y, button } => {
                enigo.mouse_move_to((x as f64 * dpi_factor) as i32, (y as f64 * dpi_factor) as i32);
                enigo.mouse_down(button.into())
            }
            InputMessage::MouseUp { x, y, button } => {
                enigo.mouse_move_to((x as f64 * dpi_factor) as i32, (y as f64 * dpi_factor) as i32);
                enigo.mouse_up(button.into())
            }
            InputMessage::MouseWheel { x, y, dx, dy } => {
                enigo.mouse_move_to((x as f64 * dpi_factor) as i32, (y as f64 * dpi_factor) as i32);
                enigo.mouse_scroll_y((dy as f64 / 120. * scroll_reverse_factor) as i32);
                enigo.mouse_scroll_x((dx as f64 / 120.) as i32 );
            }
        };
        Ok(())
    }

    pub fn new(disabled_control: bool, dpi_factor: f64) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Bytes>(32);
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                if disabled_control {
                    continue; // Skip the message if user disabled remote control
                }
                if let Err(err) = Self::handle_input_event(msg, dpi_factor) {
                    warn!("Error handling input event: {}", err);
                }
            }
        });
        Self { sender }
    }
}
