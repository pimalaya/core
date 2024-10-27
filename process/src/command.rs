//! # Command
//!
//! Module dedicated to commands. It only exposes the [`Command`]
//! struct, and various implementations of transformation.

use std::{
    ops::{Deref, DerefMut},
    process::Stdio,
};

#[cfg(feature = "async-std")]
use async_std::{io::WriteExt, process::Command as AsyncCommand};
#[cfg(feature = "tokio")]
use tokio::{io::AsyncWriteExt, process::Command as AsyncCommand};
use tracing::{debug, info};

use crate::{Error, Output, Result};

/// The command structure.
///
/// The structure is just a simple `String` wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "String", into = "String")
)]
pub struct Command {
    /// The inner command.
    inner: String,

    /// Whenever the output should be piped or not.
    ///
    /// Defaults to `true`.
    #[cfg_attr(feature = "derive", serde(skip))]
    piped: bool,
}

impl Command {
    /// Creates a new command from a string.
    ///
    /// By default, the output is piped. Use
    /// [`Command::with_output_piped`] to control this behaviour.
    pub fn new(cmd: impl ToString) -> Self {
        Self {
            inner: cmd.to_string(),
            piped: true,
        }
    }

    /// Defines whenever the output should be piped or not.
    ///
    /// See [`Command::with_output_piped`] for the builder pattern
    /// alternative.
    pub fn set_output_piped(&mut self, piped: bool) {
        self.piped = piped;
    }

    /// Defines whenever the output should be piped or not, using the
    /// builder pattern.
    ///
    /// See [`Command::set_output_piped`] for the setter alternative.
    pub fn with_output_piped(mut self, piped: bool) -> Self {
        self.set_output_piped(piped);
        self
    }

    /// Wrapper around [`alloc::str::replace`].
    ///
    /// This function is particularly useful when you need to replace
    /// placeholders.
    pub fn replace(mut self, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        self.inner = self.inner.replace(from.as_ref(), to.as_ref());
        self
    }

    /// Runs the current command without input.
    ///
    /// See [`Command::run_with`] to run command with output.
    pub async fn run(&self) -> Result<Output> {
        self.run_with([]).await
    }

    /// Run the command with the given input.
    ///
    /// If the given input is empty, the command returns straight the
    /// output. Otherwise the commands pipes this input to the
    /// standard input channel then waits for the output on the
    /// standard output channel.
    pub async fn run_with(&self, input: impl AsRef<[u8]>) -> Result<Output> {
        info!(cmd = self.inner, "run shell command");

        let input = input.as_ref();

        let stdin = if input.is_empty() {
            debug!("inherit stdin from parent");
            Stdio::inherit()
        } else {
            debug!("stdin piped");
            Stdio::piped()
        };

        let mut cmd = new_async_command()
            .arg(&self.inner)
            .stdin(stdin)
            .stdout(if self.piped {
                debug!("stdout piped");
                Stdio::piped()
            } else {
                debug!("inherit stdout from parent");
                Stdio::inherit()
            })
            .stderr(if self.piped {
                debug!("stderr piped");
                Stdio::piped()
            } else {
                debug!("inherit stderr from parent");
                Stdio::inherit()
            })
            .spawn()?;

        if !input.is_empty() {
            cmd.stdin
                .as_mut()
                .ok_or(Error::GetStdinError)?
                .write_all(input)
                .await?;
        }

        #[cfg(feature = "async-std")]
        let output = cmd.output().await?;
        #[cfg(feature = "tokio")]
        let output = cmd.wait_with_output().await?;

        let code = output
            .status
            .code()
            .ok_or_else(|| Error::GetExitStatusCodeNotAvailableError(self.to_string()))?;

        if code == 0 {
            debug!(code, "shell command gracefully exited");
        } else {
            let cmd = self.to_string();
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            debug!(code, err, "shell command ungracefully exited");
            return Err(Error::GetExitStatusCodeNonZeroError(cmd, code, err));
        }

        Ok(Output::from(output.stdout))
    }
}

impl Deref for Command {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Command {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<String> for Command {
    fn from(cmd: String) -> Self {
        Self::new(cmd)
    }
}

impl From<Command> for String {
    fn from(cmd: Command) -> Self {
        cmd.inner
    }
}

impl ToString for Command {
    fn to_string(&self) -> String {
        self.inner.clone()
    }
}

/// Prepares a new async command.
fn new_async_command() -> AsyncCommand {
    #[cfg(not(windows))]
    let windows = false;
    #[cfg(windows)]
    let windows = !std::env::var("MSYSTEM")
        .map(|env| env.starts_with("MINGW"))
        .unwrap_or_default();

    let (shell, arg) = if windows { ("cmd", "/C") } else { ("sh", "-c") };

    let mut cmd = AsyncCommand::new(shell);
    cmd.arg(arg);
    cmd
}
