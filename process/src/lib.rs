use log::{error, trace, warn};
use std::{
    env,
    io::{self, prelude::*},
    process::{Command, Stdio},
    result,
};
use thiserror::Error;

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
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmdOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
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
        trace!("running command: {}", self.to_string());

        let windows = cfg!(target_os = "windows")
            && !(env::var("MSYSTEM")
                .map(|env| env.starts_with("MINGW"))
                .unwrap_or_default());

        let (shell, arg) = if windows { ("cmd", "/C") } else { ("sh", "-c") };
        let mut cmd = Command::new(shell);
        let cmd = cmd.args(&[arg, &self.0]);

        if input.as_ref().is_empty() {
            let output = cmd.output().map_err(Error::GetOutputError)?;
            let code = output
                .status
                .code()
                .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;
            Ok(CmdOutput {
                stdout: output.stdout,
                stderr: output.stderr,
                code,
            })
        } else {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();

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
                .map_err(Error::WriteStdinError)?;

            let code = pipeline
                .wait()
                .map_err(|err| Error::WaitForExitStatusCodeError(err, self.to_string()))?
                .code()
                .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

            pipeline
                .stdout
                .as_mut()
                .ok_or(Error::GetStdoutError)?
                .read_to_end(&mut stdout)
                .map_err(Error::ReadStdoutError)?;

            pipeline
                .stdout
                .as_mut()
                .ok_or(Error::GetStderrError)?
                .read_to_end(&mut stderr)
                .map_err(Error::ReadStderrError)?;

            Ok(CmdOutput {
                stdout,
                stderr,
                code,
            })
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
        let mut output = CmdOutput {
            stdout: input.as_ref().to_owned(),
            stderr: Vec::new(),
            code: 0,
        };

        for cmd in &self.0 {
            output = cmd.run(&output.stdout)?;

            if output.code != 0 {
                warn!("command returned non-zero status exit code {}", output.code);
                warn!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                warn!("stderr: {}", String::from_utf8_lossy(&output.stderr));
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

impl Default for Cmd {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
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

impl Cmd {
    pub fn replace<P, T>(mut self, from: P, to: T) -> Self
    where
        P: AsRef<str>,
        T: AsRef<str>,
    {
        match &mut self {
            Self::SingleCmd(SingleCmd(ref mut cmd)) => {
                *cmd = cmd.replace(from.as_ref(), to.as_ref())
            }
            Self::Pipeline(Pipeline(ref mut cmds)) => {
                for SingleCmd(ref mut cmd) in cmds {
                    *cmd = cmd.replace(from.as_ref(), to.as_ref());
                }
            }
        }
        self
    }

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

impl From<&str> for Cmd {
    fn from(cmd: &str) -> Self {
        Self::SingleCmd(SingleCmd(cmd.to_owned()))
    }
}

impl From<Vec<String>> for Cmd {
    fn from(cmds: Vec<String>) -> Self {
        Self::Pipeline(Pipeline(cmds.into_iter().map(SingleCmd).collect()))
    }
}
