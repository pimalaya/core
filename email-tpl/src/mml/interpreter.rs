use log::warn;
use mail_parser::{Message, MessagePart, PartType};
use pimalaya_process::Cmd;
use std::{collections::HashSet, env, io, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // TODO: return the original chumsky::Error
    #[error("cannot parse template: {0}")]
    ParseTplError(String),
    #[error("cannot interpret template: recipient is missing")]
    InterpretTplMissingRecipientError,
    #[error("cannot interpret template")]
    WriteInterpretdPartToVecError(#[source] io::Error),
    #[error("cannot find missing property filename")]
    GetFilenamePropMissingError,
    #[error("cannot expand filename {1}")]
    ExpandFilenameError(#[source] shellexpand::LookupError<env::VarError>, String),
    #[error("cannot read attachment at {1}")]
    ReadAttachmentError(#[source] io::Error, String),
    #[error("cannot encrypt multi part")]
    EncryptPartError(#[from] pimalaya_process::Error),
    #[error("cannot sign multi part")]
    SignPartError(#[source] pimalaya_process::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the show text parts strategy [`TplBuilder`] build
/// option.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ShowTextPartsStrategy {
    /// Shows plain text parts first. If none of them found, fallback
    /// to HTML.
    #[default]
    PlainOtherwiseHtml,
    /// Shows plain text parts only.
    PlainOnly,
    /// Shows HTML parts first. If none of them found, fallback to
    /// plain text.
    HtmlOtherwisePlain,
    /// Shows HTML parts only.
    HtmlOnly,
}

/// Represents the show headers [`TplBuilder`] build option.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowHeadersStrategy {
    /// Shows all available headers in [`TplBuilder::headers`].
    #[default]
    All,
    /// Shows only specific headers from [`TplBuilder::headers`] and
    /// overrides the order [`TplBuilder::headers_order`].
    Only(HashSet<String>),
}

impl ShowHeadersStrategy {
    pub fn all() -> Self {
        Self::All
    }

    pub fn only<I, H>(headers: H) -> Self
    where
        I: ToString,
        H: IntoIterator<Item = I>,
    {
        Self::Only(HashSet::from_iter(
            headers.into_iter().map(|h| h.to_string()),
        ))
    }

    pub fn none() -> Self {
        Self::Only(HashSet::default())
    }
}

/// Represents the interpreter builder. It allows you to customize the
/// template compilation using the [Builder pattern].
///
/// [Builder pattern]: https://en.wikipedia.org/wiki/Builder_pattern
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InterpreterBuilder {
    pub pgp_decrypt_cmd: Option<Cmd>,
    pub pgp_verify_cmd: Option<Cmd>,
    pub pgp_verify_recipient: Option<String>,
    pub show_text_parts: ShowTextPartsStrategy,
    pub show_headers: ShowHeadersStrategy,
    pub show_text_parts_only: bool,
    /// Represents the build option that sanitizes text/plain parts.
    pub sanitize_text_plain_parts: bool,
    /// Represents the build option that sanitizes text/html parts.
    pub sanitize_text_html_parts: bool,
    /// Represents the build option that removes signature from
    /// text/plain parts.
    pub remove_text_plain_parts_signature: bool,
}

impl<'a> InterpreterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_decrypt_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_decrypt_cmd = cmd.map(|c| c.into());
        self
    }

    pub fn pgp_verify_recipient<R: AsRef<str>>(mut self, recipient: R) -> Self {
        match recipient.as_ref().parse() {
            Ok(mbox) => {
                self.pgp_verify_recipient = Some(mbox);
            }
            Err(err) => {
                warn!(
                    "skipping invalid pgp verify recipient {}: {}",
                    recipient.as_ref(),
                    err
                );
            }
        }
        self
    }

    pub fn pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_verify_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_verify_cmd = cmd.map(|c| c.into());
        self
    }

    pub fn show_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::All;
        self
    }

    /// Appends headers filters to the template builder. See
    /// [TplBuilder::show_headers] for more information about the
    /// `show_headers` build option.
    pub fn show_headers<S: ToString, B: IntoIterator<Item = S>>(mut self, headers: B) -> Self {
        let headers = headers
            .into_iter()
            .map(|header| header.to_string())
            .collect();

        match self.show_headers {
            ShowHeadersStrategy::All => {
                self.show_headers = ShowHeadersStrategy::Only(headers);
            }
            ShowHeadersStrategy::Only(prev_headers) => {
                let mut prev_headers = prev_headers.clone();
                prev_headers.extend(headers);
                self.show_headers = ShowHeadersStrategy::Only(prev_headers);
            }
        };

        self
    }

    pub fn hide_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::Only(HashSet::new());
        self
    }

    pub fn build(self) -> Interpreter {
        Interpreter {
            pgp_decrypt_cmd: self
                .pgp_decrypt_cmd
                .unwrap_or_else(|| "gpg --decrypt --quiet".into()),
            pgp_verify_recipient: self.pgp_verify_recipient,
            pgp_verify_cmd: self
                .pgp_verify_cmd
                .unwrap_or_else(|| "gpg --verify --quiet --recipient <recipient>".into()),
            show_text_parts: self.show_text_parts,
            show_headers: self.show_headers,
            show_text_parts_only: self.show_text_parts_only,
            sanitize_text_plain_parts: self.sanitize_text_plain_parts,
            sanitize_text_html_parts: self.sanitize_text_html_parts,
            remove_text_plain_parts_signature: self.remove_text_plain_parts_signature,
        }
    }
}

/// Represents the interpreter options. It is the final struct passed
/// down to the [Tpl::interpret] function.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Interpreter {
    pub pgp_decrypt_cmd: Cmd,
    pub pgp_verify_recipient: Option<String>,
    pub pgp_verify_cmd: Cmd,
    pub show_text_parts: ShowTextPartsStrategy,
    pub show_headers: ShowHeadersStrategy,
    pub show_text_parts_only: bool,
    pub sanitize_text_plain_parts: bool,
    pub sanitize_text_html_parts: bool,
    pub remove_text_plain_parts_signature: bool,
}

impl<'a> Interpreter {
    /// Interprets the given string template into a raw MIME Message
    /// using [InterpreterOpts] from the builder.
    pub fn interpret(&self, msg: &Message) -> Result<String> {
        self.interpret_part(msg.root_part())
    }

    /// Builds the final PGP encrypt system command by replacing
    /// `<recipient>` occurrences with the actual recipient. Fails in
    /// case no recipient is found.
    fn pgp_verify_cmd(&self) -> Result<Cmd> {
        let recipient = self
            .pgp_verify_recipient
            .as_ref()
            .ok_or(Error::InterpretTplMissingRecipientError)?;

        let cmd = self
            .pgp_verify_cmd
            .clone()
            .replace("<recipient>", &recipient.to_string());

        Ok(cmd)
    }

    fn interpret_part(&self, part: &MessagePart) -> Result<String> {
        let mut tpl = String::new();

        match &part.body {
            PartType::Text(text) => tpl.push_str(&text),
            PartType::Html(_) => (),
            PartType::Binary(_) => (),
            PartType::InlineBinary(_) => (),
            PartType::Message(msg) => tpl.push_str(&self.interpret(msg)?),
            PartType::Multipart(_) => (),
        }

        Ok(tpl)
    }
}
