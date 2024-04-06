//! # Process
//!
//! Cross-platform, asynchronous library to run commands in pipelines.
//!
//! The core concept of this library is to simplify the execution of
//! shell commands, following these rules:
//!
//! 1. Commands are executed asynchronously, using the [tokio] async
//! runtime.
//!
//! 2. Commands work on all major platforms (windows, macos and
//! linux).
//!
//! 3. Commands can be executed in a pipeline, which means the output
//! of the previous command is send as input of the next one.

pub mod error;
pub use error::*;

use log::debug;
use std::{
    env,
    ops::{Deref, DerefMut},
    process::Stdio,
};
use tokio::{io::AsyncWriteExt, process::Command as TokioCommand};

fn new_tokio_cmd() -> TokioCommand {
    let windows = cfg!(target_os = "windows")
        && !(env::var("MSYSTEM")
            .map(|env| env.starts_with("MINGW"))
            .unwrap_or_default());

    let (shell, arg) = if windows { ("cmd", "/C") } else { ("sh", "-c") };

    let mut cmd = TokioCommand::new(shell);
    cmd.arg(arg);
    cmd
}

/// The main command structure.
///
/// A command can be either a single command or a pipeline of single
/// commands.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
pub enum Command {
    /// The single command variant.
    SingleCommand(SingleCommand),

    /// The pipeline variant.
    Pipeline(Pipeline),
}

impl Command {
    /// Wrapper around [`alloc::str::replace`].
    ///
    /// This function is particularly useful when you need to replace
    /// placeholders on all inner commands.
    pub fn replace(mut self, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        match &mut self {
            Self::SingleCommand(SingleCommand(cmd, ..)) => {
                *cmd = cmd.replace(from.as_ref(), to.as_ref())
            }
            Self::Pipeline(Pipeline(cmds)) => {
                for SingleCommand(cmd, ..) in cmds {
                    *cmd = cmd.replace(from.as_ref(), to.as_ref());
                }
            }
        }
        self
    }

    /// Run the command without piped input.
    pub async fn run(&self) -> Result<CommandOutput> {
        self.run_with([]).await
    }

    /// Run the command with the given piped input.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CommandOutput> {
        match self {
            Self::SingleCommand(cmd) => cmd.run_with(input).await,
            Self::Pipeline(cmds) => cmds.run_with(input).await,
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
    }
}

impl From<String> for Command {
    fn from(cmd: String) -> Self {
        Self::SingleCommand(cmd.into())
    }
}

impl From<&String> for Command {
    fn from(cmd: &String) -> Self {
        Self::SingleCommand(cmd.into())
    }
}

impl From<&str> for Command {
    fn from(cmd: &str) -> Self {
        Self::SingleCommand(cmd.into())
    }
}

impl From<Vec<String>> for Command {
    fn from(cmd: Vec<String>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<Vec<&String>> for Command {
    fn from(cmd: Vec<&String>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<Vec<&str>> for Command {
    fn from(cmd: Vec<&str>) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[String]> for Command {
    fn from(cmd: &[String]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[&String]> for Command {
    fn from(cmd: &[&String]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl From<&[&str]> for Command {
    fn from(cmd: &[&str]) -> Self {
        Self::Pipeline(cmd.into())
    }
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Self::SingleCommand(cmd) => cmd.to_string(),
            Self::Pipeline(pipeline) => pipeline.to_string(),
        }
    }
}

/// The single command structure.
///
/// Represents commands that are composed of one single command.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "String", into = "String")
)]
pub struct SingleCommand(String, bool);

impl SingleCommand {
    pub fn with_output_piped(mut self, piped: bool) -> Self {
        self.1 = piped;
        self
    }

    pub async fn run(&self) -> Result<CommandOutput> {
        self.run_with([]).await
    }

    /// Run the single command with the given input.
    ///
    /// If the given input is empty, the command gets straight the
    /// output. Otherwise the commands pipes this input to the
    /// standard input channel then waits for the output on the
    /// standard output channel.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CommandOutput> {
        debug!("running single command: {}", self.to_string());

        let input = input.as_ref();

        let stdin = if input.is_empty() {
            Stdio::inherit()
        } else {
            Stdio::piped()
        };

        let stdout = || {
            if self.1 {
                Stdio::piped()
            } else {
                Stdio::inherit()
            }
        };

        let mut cmd = new_tokio_cmd()
            .arg(&self.0)
            .stdin(stdin)
            .stdout(stdout())
            .stderr(stdout())
            .spawn()?;

        if !input.is_empty() {
            cmd.stdin
                .as_mut()
                .ok_or(Error::GetStdinError)?
                .write_all(input)
                .await?
        }

        let output = cmd.wait_with_output().await?;

        let code = output
            .status
            .code()
            .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

        if code != 0 {
            let cmd = self.to_string();
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::GetExitStatusCodeNonZeroError(cmd, code, err));
        }

        Ok(output.stdout.into())
    }
}

impl Deref for SingleCommand {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SingleCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<String> for SingleCommand {
    fn from(cmd: String) -> Self {
        Self(cmd, true)
    }
}

impl From<&String> for SingleCommand {
    fn from(cmd: &String) -> Self {
        cmd.as_str().into()
    }
}

impl From<&str> for SingleCommand {
    fn from(cmd: &str) -> Self {
        cmd.to_owned().into()
    }
}

impl From<SingleCommand> for String {
    fn from(cmd: SingleCommand) -> Self {
        cmd.0
    }
}

impl ToString for SingleCommand {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

/// The command pipeline structure.
///
/// Represents commands that are composed of multiple single
/// commands. Commands are run in a pipeline, which means the output
/// of the previous command is piped to the input of the next one, and
/// so on.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "Vec<String>", into = "Vec<String>")
)]
pub struct Pipeline(Vec<SingleCommand>);

impl Pipeline {
    /// Run the command pipeline with the given input.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CommandOutput> {
        debug!("running pipeline: {}", self.to_string());

        let mut output = input.as_ref().to_owned();

        for cmd in &self.0 {
            output = cmd.run_with(&output).await?.0;
        }

        Ok(output.into())
    }
}

impl Deref for Pipeline {
    type Target = Vec<SingleCommand>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Pipeline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<String>> for Pipeline {
    fn from(cmd: Vec<String>) -> Self {
        Self(cmd.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<&String>> for Pipeline {
    fn from(cmd: Vec<&String>) -> Self {
        Self(cmd.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<&str>> for Pipeline {
    fn from(cmd: Vec<&str>) -> Self {
        Self(cmd.into_iter().map(Into::into).collect())
    }
}

impl From<&[String]> for Pipeline {
    fn from(cmd: &[String]) -> Self {
        Self(cmd.iter().map(Into::into).collect())
    }
}

impl From<&[&String]> for Pipeline {
    fn from(cmd: &[&String]) -> Self {
        Self(cmd.iter().map(|cmd| (*cmd).into()).collect())
    }
}

impl From<&[&str]> for Pipeline {
    fn from(cmd: &[&str]) -> Self {
        Self(cmd.iter().map(|cmd| (*cmd).into()).collect())
    }
}

impl From<Pipeline> for Vec<String> {
    fn from(val: Pipeline) -> Self {
        val.iter().map(ToString::to_string).collect()
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
pub struct CommandOutput(Vec<u8>);

impl CommandOutput {
    /// Reads the command output as string lossy.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self).to_string()
    }
}

impl Deref for CommandOutput {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CommandOutput {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for CommandOutput {
    fn from(output: Vec<u8>) -> Self {
        Self(output)
    }
}

impl From<CommandOutput> for Vec<u8> {
    fn from(val: CommandOutput) -> Self {
        val.0
    }
}

impl TryFrom<CommandOutput> for String {
    type Error = Error;

    fn try_from(cmd: CommandOutput) -> Result<Self> {
        String::from_utf8(cmd.into()).map_err(Error::ParseOutputAsUtf8StringError)
    }
}
