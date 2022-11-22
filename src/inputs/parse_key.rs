use crate::Result;
use failure::format_err;

pub enum KeyOrSequence {
    Key(enigo::Key),
    Sequence(char),
}

impl KeyOrSequence {
    pub fn from_js_key(s: &str) -> Result<KeyOrSequence> {
        if s.len() == 1 {
            return Ok(KeyOrSequence::Sequence(s.chars().next().unwrap()));
        }

        match s {
            "Alt" => Ok(KeyOrSequence::Key(enigo::Key::Alt)),
            "Backspace" => Ok(KeyOrSequence::Key(enigo::Key::Backspace)),
            "Control" => Ok(KeyOrSequence::Key(enigo::Key::Control)),
            "Delete" => Ok(KeyOrSequence::Key(enigo::Key::Delete)),
            "End" => Ok(KeyOrSequence::Key(enigo::Key::End)),
            "Escape" => Ok(KeyOrSequence::Key(enigo::Key::Escape)),
            "F1" => Ok(KeyOrSequence::Key(enigo::Key::F1)),
            "F2" => Ok(KeyOrSequence::Key(enigo::Key::F2)),
            "F3" => Ok(KeyOrSequence::Key(enigo::Key::F3)),
            "F4" => Ok(KeyOrSequence::Key(enigo::Key::F4)),
            "F5" => Ok(KeyOrSequence::Key(enigo::Key::F5)),
            "F6" => Ok(KeyOrSequence::Key(enigo::Key::F6)),
            "F7" => Ok(KeyOrSequence::Key(enigo::Key::F7)),
            "F8" => Ok(KeyOrSequence::Key(enigo::Key::F8)),
            "F9" => Ok(KeyOrSequence::Key(enigo::Key::F9)),
            "F10" => Ok(KeyOrSequence::Key(enigo::Key::F10)),
            "F11" => Ok(KeyOrSequence::Key(enigo::Key::F11)),
            "F12" => Ok(KeyOrSequence::Key(enigo::Key::F12)),
            "Home" => Ok(KeyOrSequence::Key(enigo::Key::Home)),
            "ArrowLeft" => Ok(KeyOrSequence::Key(enigo::Key::LeftArrow)),
            "ArrowUp" => Ok(KeyOrSequence::Key(enigo::Key::UpArrow)),
            "ArrowRight" => Ok(KeyOrSequence::Key(enigo::Key::RightArrow)),
            "ArrowDown" => Ok(KeyOrSequence::Key(enigo::Key::DownArrow)),
            "Meta" => Ok(KeyOrSequence::Key(enigo::Key::Meta)),
            "Tab" => Ok(KeyOrSequence::Key(enigo::Key::Tab)),
            "Enter" => Ok(KeyOrSequence::Key(enigo::Key::Return)),
            "Shift" => Ok(KeyOrSequence::Key(enigo::Key::Shift)),
            "CapsLock" => Ok(KeyOrSequence::Key(enigo::Key::CapsLock)),
            "Space" => Ok(KeyOrSequence::Key(enigo::Key::Space)),
            "PageUp" => Ok(KeyOrSequence::Key(enigo::Key::PageUp)),
            "PageDown" => Ok(KeyOrSequence::Key(enigo::Key::PageDown)),
            _ => Err(format_err!("Unknown key: {}", s)),
        }
    }
}
