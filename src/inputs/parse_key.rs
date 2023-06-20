use crate::Result;
use anyhow::format_err;

pub trait FromJsKey {
    /// convert from KeyboardEvent.code
    fn from_js_key(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl FromJsKey for enigo::Key {
    fn from_js_key(s: &str) -> Result<Self> {
        // Handle Digit0-9
        if s.starts_with("Digit") || (s.len() == 7 && s.starts_with("Numpad")) {
            let digit = s.chars().last().unwrap();
            return Ok(enigo::Key::Layout(digit));
        }

        // Handle KeyA-Z
        if s.starts_with("Key") {
            let letter = s.chars().last().unwrap().to_ascii_lowercase();
            return Ok(enigo::Key::Layout(letter));
        }

        // Unsupported Keys:
        // Pause, NumLock, Context Menu, Numpad*
        // Currently does not distinguish between left and right keys
        match s {
            "Backspace" => Ok(enigo::Key::Backspace),
            "Tab" => Ok(enigo::Key::Tab),
            "ShiftLeft" => Ok(enigo::Key::Shift),
            "ShiftRight" => Ok(enigo::Key::Shift),
            "ControlLeft" => Ok(enigo::Key::Control),
            "ControlRight" => Ok(enigo::Key::Control),
            "AltLeft" => Ok(enigo::Key::Alt),
            "AltRight" => Ok(enigo::Key::Alt),
            "CapsLock" => Ok(enigo::Key::CapsLock),
            "Escape" => Ok(enigo::Key::Escape),
            "Space" => Ok(enigo::Key::Space),
            "ArrowLeft" => Ok(enigo::Key::LeftArrow),
            "ArrowUp" => Ok(enigo::Key::UpArrow),
            "ArrowRight" => Ok(enigo::Key::RightArrow),
            "ArrowDown" => Ok(enigo::Key::DownArrow),
            "MetaLeft" => Ok(enigo::Key::Meta),
            "MetaRight" => Ok(enigo::Key::Meta),
            "End" => Ok(enigo::Key::End),
            "F1" => Ok(enigo::Key::F1),
            "F2" => Ok(enigo::Key::F2),
            "F3" => Ok(enigo::Key::F3),
            "F4" => Ok(enigo::Key::F4),
            "F5" => Ok(enigo::Key::F5),
            "F6" => Ok(enigo::Key::F6),
            "F7" => Ok(enigo::Key::F7),
            "F8" => Ok(enigo::Key::F8),
            "F9" => Ok(enigo::Key::F9),
            "F10" => Ok(enigo::Key::F10),
            "F11" => Ok(enigo::Key::F11),
            "F12" => Ok(enigo::Key::F12),
            "BracketLeft" => Ok(enigo::Key::Layout('[')),
            "BracketRight" => Ok(enigo::Key::Layout(']')),
            "Backslash" => Ok(enigo::Key::Layout('\\')),
            "Semicolon" => Ok(enigo::Key::Layout(';')),
            "Backquote" => Ok(enigo::Key::Layout('`')),
            "Quote" => Ok(enigo::Key::Layout('\'')),
            "Equal" => Ok(enigo::Key::Layout('=')),
            "Minus" => Ok(enigo::Key::Layout('-')),
            "Comma" => Ok(enigo::Key::Layout(',')),
            "Period" => Ok(enigo::Key::Layout('.')),
            "Slash" => Ok(enigo::Key::Layout('/')),
            "Delete" => Ok(enigo::Key::Delete),
            "Home" => Ok(enigo::Key::Home),
            "Enter" => Ok(enigo::Key::Return),
            "PageUp" => Ok(enigo::Key::PageUp),
            "PageDown" => Ok(enigo::Key::PageDown),
            _ => Err(format_err!("Unknown key: {}", s)),
        }
    }
}
