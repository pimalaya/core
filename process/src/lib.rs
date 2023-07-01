use log::{debug, error, warn};
use std::{
    borrow::Cow,
    env, io,
    ops::{Deref, DerefMut},
    process::Stdio,
    result,
    string::FromUtf8Error,
};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run command: {1}")]
    SpawnProcessError(#[source] io::Error, String),
    #[error("cannot get standard input")]
    GetStdinError,
    #[error("cannot wait for exit status code of command: {1}")]
    WaitForExitStatusCodeError(#[source] io::Error, String),
    #[error("cannot get exit status code of command: {0}")]
    GetExitStatusCodeNotAvailableError(String),
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
    ParseCmdOutputStdoutError(#[source] FromUtf8Error),
    #[error("cannot parse command error output as string")]
    ParseCmdOutputStderrError(#[source] FromUtf8Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The main command structure.
///
/// A command can be either a single command or a command pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cmd {
    /// The single command variant.
    SingleCmd(SingleCmd),

    /// The command pipeline variant.
    Pipeline(Pipeline),
}

impl Cmd {
    /// Wrapper around `alloc::str::replace`.
    ///
    /// This function is particularly useful when you need to replace
    /// a placeholder on all inner commands.
    pub fn replace(mut self, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        match &mut self {
            Self::SingleCmd(SingleCmd(cmd)) => *cmd = cmd.replace(from.as_ref(), to.as_ref()),
            Self::Pipeline(Pipeline(cmds)) => {
                for SingleCmd(cmd) in cmds {
                    *cmd = cmd.replace(from.as_ref(), to.as_ref());
                }
            }
        }
        self
    }

    /// Runs the command with the given input.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        match self {
            Self::SingleCmd(cmd) => cmd.run(input).await,
            Self::Pipeline(cmds) => cmds.run(input).await,
        }
    }

    /// Runs the command without input.
    pub async fn run(&self) -> Result<CmdOutput> {
        self.run_with([]).await
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

impl From<&str> for Cmd {
    fn from(cmd: &str) -> Self {
        Self::SingleCmd(cmd.into())
    }
}

impl From<Vec<String>> for Cmd {
    fn from(cmds: Vec<String>) -> Self {
        Self::Pipeline(cmds.into())
    }
}

impl From<Vec<&str>> for Cmd {
    fn from(cmds: Vec<&str>) -> Self {
        Self::Pipeline(cmds.into())
    }
}

impl From<&[String]> for Cmd {
    fn from(cmds: &[String]) -> Self {
        Self::Pipeline(cmds.to_vec().into())
    }
}

impl From<&[&str]> for Cmd {
    fn from(cmds: &[&str]) -> Self {
        Self::Pipeline(cmds.to_vec().into())
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
/// Represents commands that are only composed of one single
/// command. No pipe is involved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleCmd(String);

impl SingleCmd {
    /// Runs the single command.
    ///
    /// If the given input is empty, the command waits straight for
    /// the output. Otherwise the commands pipes this input to the
    /// standard input channel then waits for the output.
    async fn run(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        debug!("running command: {}", self.to_string());

        let windows = cfg!(target_os = "windows")
            && !(env::var("MSYSTEM")
                .map(|env| env.starts_with("MINGW"))
                .unwrap_or_default());

        let (shell, arg) = if windows { ("cmd", "/C") } else { ("sh", "-c") };
        let mut cmd = Command::new(shell);
        let cmd = cmd.args(&[arg, &self.0]);

        if input.as_ref().is_empty() {
            let output = cmd.output().await.map_err(Error::GetOutputError)?;
            let code = output
                .status
                .code()
                .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;
            Ok(CmdOutput {
                out: output.stdout,
                err: output.stderr,
                code,
            })
        } else {
            let mut out = Vec::new();
            let mut err = Vec::new();

            let mut pipeline = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|err| Error::SpawnProcessError(err, self.to_string()))?;

            pipeline
                .stdin
                .as_mut()
                .ok_or(Error::GetStdinError)?
                .write_all(input.as_ref())
                .await
                .map_err(Error::WriteStdinError)?;

            let code = pipeline
                .wait()
                .await
                .map_err(|err| Error::WaitForExitStatusCodeError(err, self.to_string()))?
                .code()
                .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

            pipeline
                .stdout
                .as_mut()
                .ok_or(Error::GetStdoutError)?
                .read_to_end(&mut out)
                .await
                .map_err(Error::ReadStdoutError)?;

            pipeline
                .stderr
                .as_mut()
                .ok_or(Error::GetStderrError)?
                .read_to_end(&mut err)
                .await
                .map_err(Error::ReadStderrError)?;

            Ok(CmdOutput { out, err, code })
        }
    }
}

impl Deref for SingleCmd {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SingleCmd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<String> for SingleCmd {
    fn from(cmd: String) -> Self {
        Self(cmd)
    }
}

impl From<&str> for SingleCmd {
    fn from(cmd: &str) -> Self {
        Self(cmd.to_owned())
    }
}

impl ToString for SingleCmd {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

/// The command pipeline structure.
///
/// Represents commands that are composed of multiple single
/// commands. Commands are run in a pipeline, which means the output
/// of the previous command is piped to the input of the next one, and
/// so on.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Pipeline(Vec<SingleCmd>);

impl Pipeline {
    /// Runs the command pipeline.
    async fn run(&self, input: impl AsRef<[u8]>) -> Result<CmdOutput> {
        let mut output = CmdOutput {
            out: input.as_ref().to_owned(),
            err: Vec::new(),
            code: 0,
        };

        for cmd in &self.0 {
            output = cmd.run(&output.out).await?;
            let code = output.code;
            if code != 0 {
                let err = output.read_out_lossy();
                warn!("command returned non-zero status exit code {code}: {err}");
                break;
            }
        }

        Ok(output)
    }
}

impl Deref for Pipeline {
    type Target = Vec<SingleCmd>;

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

impl From<Vec<&str>> for Pipeline {
    fn from(cmd: Vec<&str>) -> Self {
        Self(cmd.into_iter().map(Into::into).collect())
    }
}

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

/// The command output structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmdOutput {
    /// The error code returned by the command.
    pub code: i32,

    /// The command output from the standard output channel.
    pub out: Vec<u8>,

    /// The command error from the standard error channel.
    pub err: Vec<u8>,
}

impl CmdOutput {
    /// Reads the output as string.
    ///
    /// If the code is 0, reads the command output, otherwise reads
    /// the command error output.
    pub fn read_out(&self) -> Result<String> {
        if self.code == 0 {
            String::from_utf8(self.out.clone()).map_err(Error::ParseCmdOutputStdoutError)
        } else {
            String::from_utf8(self.err.clone()).map_err(Error::ParseCmdOutputStderrError)
        }
    }

    /// Same as `read_out` but lossy.
    pub fn read_out_lossy(&self) -> Cow<str> {
        if self.code == 0 {
            String::from_utf8_lossy(&self.out)
        } else {
            String::from_utf8_lossy(&self.err)
        }
    }
}
