#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Message {
    Start,
    Stop,
    SetMaxFps(String),
    SetDisplay(String),
    CopyInviteLink,
    CopyRoomID,
    CopyPasscode,
    Ignore,
}
