use std::{io, str::FromStr};

#[derive(Debug)]
pub enum Request {
    Start,
    Get,
    Pause,
    Resume,
    Stop,
    Kill,
}

impl ToString for Request {
    fn to_string(&self) -> String {
        match self {
            Self::Start => String::from("start"),
            Self::Get => String::from("get"),
            Self::Pause => String::from("pause"),
            Self::Resume => String::from("resume"),
            Self::Stop => String::from("stop"),
            Self::Kill => String::from("kill"),
        }
    }
}

impl FromStr for Request {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "start" => Ok(Self::Start),
            "get" => Ok(Self::Get),
            "pause" => Ok(Self::Pause),
            "resume" => Ok(Self::Resume),
            "stop" => Ok(Self::Stop),
            "kill" => Ok(Self::Kill),
            unknown => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid request: {unknown}"),
            )),
        }
    }
}
