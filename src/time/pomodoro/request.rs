#[derive(Debug)]
pub enum Request {
    Start,
    Get,
    Pause,
    Resume,
    Stop,
}

impl ToString for Request {
    fn to_string(&self) -> String {
        match self {
            Self::Start => String::from("start"),
            Self::Get => String::from("get"),
            Self::Pause => String::from("pause"),
            Self::Resume => String::from("resume"),
            Self::Stop => String::from("stop"),
        }
    }
}
