pub enum Message {
    Start,
    Get,
    Stop,
    Close,
}

impl ToString for Message {
    fn to_string(&self) -> String {
        String::from(match self {
            Self::Start => "start",
            Self::Get => "get",
            Self::Stop => "stop",
            Self::Close => "close",
        })
    }
}
