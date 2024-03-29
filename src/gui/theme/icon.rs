/// Material Design Icons
/// https://fonts.google.com/icons
#[derive(Debug, Clone, Copy)]
pub enum Icon {
    ContentCopy,
    PlayCircle,
    StopCircle,
    Link,
    Group,
    Close,
    Done,
    PersonRemove,
}

impl From<&Icon> for char {
    fn from(icon: &Icon) -> Self {
        match icon {
            Icon::ContentCopy => '\u{e14d}',
            Icon::PlayCircle => '\u{e1c4}',
            Icon::StopCircle => '\u{ef71}',
            Icon::Link => '\u{e157}',
            Icon::Group => '\u{e7ef}',
            Icon::Close => '\u{e5cd}',
            Icon::Done => '\u{e876}',
            Icon::PersonRemove => '\u{ef66}',
        }
    }
}

impl From<Icon> for char {
    fn from(icon: Icon) -> Self {
        char::from(&icon)
    }
}

impl ToString for Icon {
    fn to_string(&self) -> String {
        String::from(char::from(self))
    }
}
