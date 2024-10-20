use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use tracing::{debug, info};

use crate::{Command, Output, Result};

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
pub struct Pipeline(Vec<Command>);

impl Pipeline {
    pub fn new(cmds: impl IntoIterator<Item = impl Into<Command>>) -> Self {
        Self(cmds.into_iter().map(Into::into).collect())
    }

    /// Wrapper around [`alloc::str::replace`].
    ///
    /// This function is particularly useful when you need to replace
    /// placeholders on all inner commands.
    pub fn replace(mut self, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        for cmd in self.iter_mut() {
            *cmd = cmd.clone().replace(from.as_ref(), to.as_ref())
        }

        self
    }

    pub async fn run(&self) -> Result<Output> {
        self.run_with([]).await
    }

    /// Run the command pipeline with the given input.
    pub async fn run_with(&self, input: impl IntoIterator<Item = u8>) -> Result<Output> {
        info!("run pipeline of {} commands", self.len());

        let mut output: Vec<u8> = input.into_iter().collect();

        for (i, cmd) in self.iter().enumerate() {
            debug!("run command {} from pipeline", i + 1);
            output = cmd.run_with(&output).await?.into();
        }

        Ok(Output::from(output))
    }
}

impl Deref for Pipeline {
    type Target = Vec<Command>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Pipeline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Command>> for Pipeline {
    fn from(cmds: Vec<Command>) -> Self {
        Self(cmds)
    }
}

impl From<Vec<String>> for Pipeline {
    fn from(cmds: Vec<String>) -> Self {
        Self(cmds.into_iter().map(Command::from).collect())
    }
}

impl From<Pipeline> for Vec<String> {
    fn from(pipeline: Pipeline) -> Self {
        pipeline.iter().map(ToString::to_string).collect()
    }
}

impl fmt::Display for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut glue = "";

        for cmd in self.iter() {
            write!(f, "{glue}{}", cmd.to_string())?;
            glue = " | ";
        }

        Ok(())
    }
}
