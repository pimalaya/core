//! Cross-platform, asynchronous library to run commands in pipelines.
//!
//! The core concept of this library is to simplify the execution of
//! commands, following these rules:
//!
//! 1. Commands are executed asynchronously, using the [tokio] async
//! runtime.
//!
//! 2. Commands work on all major platforms (windows, macos and
//! linux).
//!
//! 3. Commands can be executed in a pipeline, which means the output
//! of the previous command is send as input of the next one.

use log::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    env, io,
    ops::{Deref, DerefMut},
    process::Stdio,
    result,
    string::FromUtf8Error,
};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

const TOKIO_CMD: Lazy<tokio::process::Command> = Lazy::new(|| {
    let windows = cfg!(target_os = "windows")
        && !(env::var("MSYSTEM")
            .map(|env| env.starts_with("MINGW"))
            .unwrap_or_default());

    let (shell, arg) = if windows { ("cmd", "/C") } else { ("sh", "-c") };

    let mut cmd = tokio::process::Command::new(shell);
    cmd.arg(arg);
    cmd
});

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get standard input")]
    GetStdinError,
    #[error("cannot wait for exit status code of command: {1}")]
    WaitForExitStatusCodeError(#[source] io::Error, String),
    #[error("cannot get exit status code of command: {0}")]
    GetExitStatusCodeNotAvailableError(String),
    #[error("command {0} returned non-zero exit status code {1}: {2}")]
    InvalidExitStatusCodeNonZeroError(String, i32, String),
    #[error("cannot write data to standard input")]
    WriteStdinError(#[source] io::Error),
    #[error("cannot get standard output")]
    GetStdoutError,
    #[error("cannot read data from standard output")]
    ReadStdoutError(#[source] io::Error),
    #[error("cannot get standard error")]
    GetStderrError,
    #[error("cannot read data from standard error")]
    ReadStderrError(#[source] io::Error),
    #[error("cannot get command output")]
    GetOutputError(#[source] io::Error),
    #[error("cannot parse command output as string")]
    ParseOutputAsUtf8StringError(#[source] FromUtf8Error),

    #[error(transparent)]
    IoError(#[from] io::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The main command structure.
///
/// A command can be either a single command or a pipeline.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cmd {
    /// The single command variant.
    SingleCmd(SingleCmd),

    /// The pipeline variant.
    Pipeline(Pipeline),
}

impl Cmd {
    /// Wrapper around [`alloc::str::replace`].
    ///
    /// This function is particularly useful when you need to replace
    /// placeholders on all inner commands.
    pub fn replace(mut self, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        match &mut self {
            Self::SingleCmd(SingleCmd { cmd, .. }) => {
                *cmd = cmd.replace(from.as_ref(), to.as_ref())
            }
            Self::Pipeline(Pipeline { cmds }) => {
                for SingleCmd { cmd, .. } in cmds {
                    *cmd = cmd.replace(from.as_ref(), to.as_ref());
                }
            }
        }
        self
    }

    /// Runs the command without piped input.
    pub async fn run(&self) -> Result<CmdOutput> {
        self.run_with([]).await
    }

    /// Runs the command with the given piped input.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        match self {
            Self::SingleCmd(cmd) => cmd.run_with(input).await,
            Self::Pipeline(cmds) => cmds.run_with(input).await,
        }
    }
}

impl Default for Cmd {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
    }
}

impl From<String> for Cmd {
    fn from(cmd: String) -> Self {
        Self::SingleCmd(cmd.into())
    }
}

impl From<&String> for Cmd {
    fn from(cmd: &String) -> Self {
        cmd.clone().into()
    }
}

impl From<&str> for Cmd {
    fn from(cmd: &str) -> Self {
        cmd.to_owned().into()
    }
}

impl From<Vec<String>> for Cmd {
    fn from(cmd: Vec<String>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<Vec<&String>> for Cmd {
    fn from(cmd: Vec<&String>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<Vec<&str>> for Cmd {
    fn from(cmd: Vec<&str>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[String]> for Cmd {
    fn from(cmd: &[String]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[&String]> for Cmd {
    fn from(cmd: &[&String]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[&str]> for Cmd {
    fn from(cmd: &[&str]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl ToString for Cmd {
    fn to_string(&self) -> String {
        match self {
            Self::SingleCmd(cmd) => cmd.to_string(),
            Self::Pipeline(pipeline) => pipeline.to_string(),
        }
    }
}

/// The single command structure.
///
/// Represents commands that are only composed of one single command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct SingleCmd {
    cmd: String,
    #[serde(skip_deserializing)]
    output_piped: bool,
}

impl SingleCmd {
    pub fn with_output_piped(mut self, piped: bool) -> Self {
        self.output_piped = piped;
        self
    }

    pub async fn run(&self) -> Result<CmdOutput> {
        self.run_with([]).await
    }

    /// Runs the single command with the given input.
    ///
    /// If the given input is empty, the command gets straight the
    /// output. Otherwise the commands pipes this input to the
    /// standard input channel then waits for the output on the
    /// standard output channel.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        debug!("running single command: {}", self.to_string());

        let input = input.as_ref();

        let stdin = if input.is_empty() {
            Stdio::inherit()
        } else {
            Stdio::piped()
        };

        let stdout = || {
            if self.output_piped {
                Stdio::piped()
            } else {
                Stdio::inherit()
            }
        };

        let mut cmd = TOKIO_CMD;
        let mut cmd = cmd
            .arg(&self.cmd)
            .stdin(stdin)
            .stdout(stdout())
            .stderr(stdout())
            .spawn()?;

        if !input.is_empty() {
            cmd.stdin
                .as_mut()
                .ok_or(Error::GetStdinError)?
                .write_all(input)
                .await
                .map_err(Error::WriteStdinError)?;
        }

        let output = cmd
            .wait_with_output()
            .await
            .map_err(Error::GetOutputError)?;

        let code = output
            .status
            .code()
            .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

        if code != 0 {
            let cmd = self.to_string();
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::InvalidExitStatusCodeNonZeroError(cmd, code, err));
        }

        Ok(output.stdout.into())
    }
}

impl Deref for SingleCmd {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl DerefMut for SingleCmd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmd
    }
}

impl From<String> for SingleCmd {
    fn from(cmd: String) -> Self {
        Self {
            cmd,
            output_piped: true,
        }
    }
}

impl From<&String> for SingleCmd {
    fn from(cmd: &String) -> Self {
        cmd.as_str().into()
    }
}

impl From<&str> for SingleCmd {
    fn from(cmd: &str) -> Self {
        cmd.to_owned().into()
    }
}

impl Into<String> for SingleCmd {
    fn into(self) -> String {
        self.cmd
    }
}

impl ToString for SingleCmd {
    fn to_string(&self) -> String {
        self.clone().into()
    }
}

/// The command pipeline structure.
///
/// Represents commands that are composed of multiple single
/// commands. Commands are run in a pipeline, which means the output
/// of the previous command is piped to the input of the next one, and
/// so on.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "Vec<String>", into = "Vec<String>")]
pub struct Pipeline {
    #[serde(flatten)]
    cmds: Vec<SingleCmd>,
}

impl Pipeline {
    /// Runs the command pipeline with the given input.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        debug!("running pipeline: {}", self.to_string());

        let mut output = input.as_ref().to_owned();

        for cmd in &self.cmds {
            output = cmd.run_with(&output).await?.0;
        }

        Ok(output.into())
    }
}

impl Deref for Pipeline {
    type Target = Vec<SingleCmd>;

    fn deref(&self) -> &Self::Target {
        &self.cmds
    }
}

impl DerefMut for Pipeline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmds
    }
}

impl From<Vec<String>> for Pipeline {
    fn from(cmd: Vec<String>) -> Self {
        Self {
            cmds: cmd.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Vec<&String>> for Pipeline {
    fn from(cmd: Vec<&String>) -> Self {
        Self {
            cmds: cmd.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Vec<&str>> for Pipeline {
    fn from(cmd: Vec<&str>) -> Self {
        Self {
            cmds: cmd.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&[String]> for Pipeline {
    fn from(cmd: &[String]) -> Self {
        Self {
            cmds: cmd.iter().map(Into::into).collect(),
        }
    }
}

impl From<&[&String]> for Pipeline {
    fn from(cmd: &[&String]) -> Self {
        Self {
            cmds: cmd.iter().map(|cmd| (*cmd).into()).collect(),
        }
    }
}

impl From<&[&str]> for Pipeline {
    fn from(cmd: &[&str]) -> Self {
        Self {
            cmds: cmd.iter().map(|cmd| (*cmd).into()).collect(),
        }
    }
}

impl Into<Vec<String>> for Pipeline {
    fn into(self) -> Vec<String> {
        self.iter().map(ToString::to_string).collect()
    }
}

impl ToString for Pipeline {
    fn to_string(&self) -> String {
        self.iter().fold(String::new(), |s, cmd| {
            if s.is_empty() {
                cmd.to_string()
            } else {
                s + " | " + &cmd.to_string()
            }
        })
    }
}

/// Wrapper around command output.
///
/// The only role of this struct is to provide convenient functions to
/// export command output as string.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CmdOutput(Vec<u8>);

impl CmdOutput {
    /// Reads the command output as string lossy.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self).to_string()
    }
}

impl Deref for CmdOutput {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CmdOutput {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for CmdOutput {
    fn from(output: Vec<u8>) -> Self {
        Self(output)
    }
}

impl Into<Vec<u8>> for CmdOutput {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl TryInto<String> for CmdOutput {
    type Error = Error;

    fn try_into(self) -> Result<String> {
        String::from_utf8(self.0).map_err(Error::ParseOutputAsUtf8StringError)
    }
}
