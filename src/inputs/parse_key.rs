use crate::Result;
use failure::format_err;

pub trait FromJsKey {
    fn from_js_key(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl FromJsKey for enigo::Key {
    fn from_js_key(s: &str) -> Result<Self> {
        if s.len() == 1 {
            return Ok(enigo::Key::Layout(s.chars().next().unwrap()));
        }

        match s {
            "Alt" => Ok(enigo::Key::Alt),
            "Backspace" => Ok(enigo::Key::Backspace),
            "Control" => Ok(enigo::Key::Control),
            "Delete" => Ok(enigo::Key::Delete),
            "End" => Ok(enigo::Key::End),
            "Escape" => Ok(enigo::Key::Escape),
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
            "Home" => Ok(enigo::Key::Home),
            "ArrowLeft" => Ok(enigo::Key::LeftArrow),
            "ArrowUp" => Ok(enigo::Key::UpArrow),
            "ArrowRight" => Ok(enigo::Key::RightArrow),
            "ArrowDown" => Ok(enigo::Key::DownArrow),
            "Meta" => Ok(enigo::Key::Meta),
            "Tab" => Ok(enigo::Key::Tab),
            "Enter" => Ok(enigo::Key::Return),
            "Shift" => Ok(enigo::Key::Shift),
            "CapsLock" => Ok(enigo::Key::CapsLock),
            "Space" => Ok(enigo::Key::Space),
            "PageUp" => Ok(enigo::Key::PageUp),
            "PageDown" => Ok(enigo::Key::PageDown),
            _ => Err(format_err!("Unknown key: {}", s)),
        }
    }
}
