//! Process module.
//!
//! This module contains cross-platform helpers around the
//! `std::process` crate.

use log::{debug, warn};
use std::{
    env,
    io::{self, prelude::*},
    ops::Deref,
    process::{Command, Stdio},
    result, string,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run command {1:?}")]
    RunCmdError(#[source] io::Error, String),
    #[error("cannot parse command output")]
    ParseCmdOutputError(#[source] string::FromUtf8Error),
    #[error("cannot spawn process for command: {1}")]
    SpawnProcessError(#[source] io::Error, String),
    #[error("cannot get standard input")]
    GetStdinError,
    #[error("cannot wait for exit status code for command: {1}")]
    WaitForExitStatusCodeError(#[source] io::Error, String),
    #[error("cannot get unavailable exit status code for command: {0}")]
    GetExitStatusCodeNotAvailableError(String),
    #[error("cannot write data to standard input")]
    WriteStdinError(#[source] io::Error),
    #[error("cannot get standard output")]
    GetStdoutError,
    #[error("cannot read data from standard output")]
    ReadStdoutError(#[source] io::Error),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmdOutput {
    output: Vec<u8>,
    pub exit_code: i32,
}

impl Deref for CmdOutput {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl CmdOutput {
    pub fn new<O: Into<Vec<u8>>>(output: O) -> Self {
        Self {
            output: output.into(),
            exit_code: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleCmd(String);

impl ToString for SingleCmd {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl SingleCmd {
    fn run<I: AsRef<[u8]>>(&self, input: I) -> Result<CmdOutput> {
        debug!("running command: {}", self.to_string());

        let mut output = Vec::new();

        let windows = cfg!(target_os = "windows")
            && !(env::var("MSYSTEM")
                .map(|env| env.starts_with("MINGW"))
                .unwrap_or_default());

        let mut pipeline = if windows {
            Command::new("cmd")
                .args(&["/C", &self.0])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else {
            Command::new("sh")
                .args(&["-c", &self.0])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
        .map_err(|err| Error::SpawnProcessError(err, self.to_string()))?;

        pipeline
            .stdin
            .as_mut()
            .ok_or(Error::GetStdinError)?
            .write_all(input.as_ref())
            .map_err(Error::WriteStdinError)?;

        let exit_code = pipeline
            .wait()
            .map_err(|err| Error::WaitForExitStatusCodeError(err, self.to_string()))?
            .code()
            .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

        pipeline
            .stdout
            .ok_or(Error::GetStdoutError)?
            .read_to_end(&mut output)
            .map_err(Error::ReadStdoutError)?;

        Ok(CmdOutput { output, exit_code })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pipeline(Vec<SingleCmd>);

impl ToString for Pipeline {
    fn to_string(&self) -> String {
        self.0.iter().fold(String::new(), |s, cmd| {
            if s.is_empty() {
                cmd.to_string()
            } else {
                s + "|" + &cmd.to_string()
            }
        })
    }
}

impl Pipeline {
    fn run<I: AsRef<[u8]>>(&self, input: I) -> Result<CmdOutput> {
        let mut output = CmdOutput::new(input.as_ref());

        for cmd in &self.0 {
            output = cmd.run(&*output)?;

            if output.exit_code != 0 {
                warn!(
                    "command returned non-zero status exit code {}",
                    output.exit_code,
                );
                warn!("{}", String::from_utf8_lossy(&output))
            }
        }

        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cmd {
    SingleCmd(SingleCmd),
    Pipeline(Pipeline),
}

impl Cmd {
    pub fn run(&self) -> Result<CmdOutput> {
        self.run_with([])
    }

    pub fn run_with<I: AsRef<[u8]>>(&self, input: I) -> Result<CmdOutput> {
        match self {
            Self::SingleCmd(cmd) => cmd.run(input),
            Self::Pipeline(cmds) => cmds.run(input),
        }
    }
}

impl From<String> for Cmd {
    fn from(cmd: String) -> Self {
        Self::SingleCmd(SingleCmd(cmd))
    }
}

impl From<Vec<String>> for Cmd {
    fn from(cmds: Vec<String>) -> Self {
        Self::Pipeline(Pipeline(cmds.into_iter().map(SingleCmd).collect()))
    }
}
