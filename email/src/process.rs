//! Process module.
//!
//! This module contains cross platform helpers around the
//! `std::process` crate.

use log::{debug, warn};

use std::{
    env,
    io::{self, prelude::*},
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

/// Runs the given command and returns the output as UTF8 string.
pub fn run(cmd: &str, input: &[u8]) -> Result<Vec<u8>> {
    let mut output = input.to_owned();
    let mut exit_code;

    for cmd in cmd.split('|') {
        debug!("running command: {}", cmd);
        (output, exit_code) = pipe(cmd.trim(), &output)?;

        if exit_code != 0 {
            warn!(
                "command returned non-zero status exit code {exit_code}:\n{output}",
                output = String::from_utf8_lossy(&output)
            );
        }
    }

    Ok(output)
}

/// Runs the given command in a pipeline and returns the raw output.
pub fn pipe(cmd: &str, input: &[u8]) -> Result<(Vec<u8>, i32)> {
    let mut output = Vec::new();

    let windows = cfg!(target_os = "windows")
        && !(env::var("MSYSTEM")
            .map(|env| env.starts_with("MINGW"))
            .unwrap_or_default());

    let mut pipeline = if windows {
        Command::new("cmd")
            .args(&["/C", cmd])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
    .map_err(|err| Error::SpawnProcessError(err, cmd.to_owned()))?;

    pipeline
        .stdin
        .as_mut()
        .ok_or(Error::GetStdinError)?
        .write_all(input)
        .map_err(Error::WriteStdinError)?;

    let exit_code = pipeline
        .wait()
        .map_err(|err| Error::WaitForExitStatusCodeError(err, cmd.to_owned()))?
        .code()
        .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(cmd.to_owned()))?;

    pipeline
        .stdout
        .ok_or(Error::GetStdoutError)?
        .read_to_end(&mut output)
        .map_err(Error::ReadStdoutError)?;

    Ok((output, exit_code))
}
