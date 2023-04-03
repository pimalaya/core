use std::{io, str::FromStr};

use super::timer::Timer;

#[derive(Debug)]
pub enum Response {
    Start,
    Get(Timer),
    Stop,
    Close,
}

impl ToString for Response {
    fn to_string(&self) -> String {
        match self {
            Self::Start => String::from("start"),
            Self::Get(timer) => format!("get {}", serde_json::to_string(timer).unwrap()),
            Self::Stop => String::from("stop"),
            Self::Close => String::from("close"),
        }
    }
}

impl FromStr for Response {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split_whitespace();
        match iter.next() {
            Some("start") => Ok(Self::Start),
            Some("stop") => Ok(Self::Stop),
            Some("close") => Ok(Self::Close),
            Some("get") => match iter.next().map(|s| serde_json::from_str(s)) {
                Some(Ok(timer)) => Ok(Self::Get(timer)),
                Some(Err(err)) => Err(Self::Err::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid request get: {err}"),
                )),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid request get: missing timer"),
                )),
            },
            Some(s) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid request {s}"),
            )),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("request missing"),
            )),
        }
    }
}
