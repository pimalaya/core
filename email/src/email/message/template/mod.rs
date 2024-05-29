//! # Template
//!
//! A template is a simplified version of an email MIME message, based
//! on [MML](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html).

pub mod config;
pub mod forward;
pub mod new;
pub mod reply;

use std::{
    borrow::Cow,
    fmt,
    ops::{Deref, DerefMut},
};

pub use mml::{
    message::{FilterHeaders, FilterParts},
    MimeInterpreter,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct Template {
    pub content: String,
    pub cursor: TemplateCursor,
}

impl Template {
    pub fn new(content: impl ToString) -> Self {
        Self::new_with_cursor(content, TemplateCursor::default())
    }

    pub fn new_with_cursor(content: impl ToString, cursor: impl Into<TemplateCursor>) -> Self {
        let content = content.to_string();
        let cursor = cursor.into();
        Self { content, cursor }
    }

    pub fn append(&mut self, section: impl AsRef<str>) {
        if !self.content.is_empty() {
            self.content.push_str(section.as_ref())
        }
    }
}

impl Deref for Template {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl DerefMut for Template {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl From<String> for Template {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[derive(Clone, Debug, Eq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct TemplateCursor {
    pub row: usize,
    pub col: usize,

    #[cfg_attr(feature = "derive", serde(skip))]
    locked: bool,
}

impl TemplateCursor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            locked: false,
        }
    }

    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Default for TemplateCursor {
    fn default() -> Self {
        Self {
            row: 1,
            col: 0,
            locked: false,
        }
    }
}

impl PartialEq for TemplateCursor {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl From<(usize, usize)> for TemplateCursor {
    fn from((row, col): (usize, usize)) -> Self {
        TemplateCursor::new(row, col)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct TemplateBody {
    content: String,
    buffer: String,
    cursor: TemplateCursor,
    push_new_lines: bool,
}

impl TemplateBody {
    pub fn new(mut cursor: TemplateCursor) -> Self {
        cursor.row += 1;

        Self {
            content: Default::default(),
            buffer: Default::default(),
            cursor,
            push_new_lines: false,
        }
    }

    pub fn flush(&mut self) {
        let mut buffer = String::new();

        if self.push_new_lines {
            buffer.push_str("\n\n");
        } else {
            self.push_new_lines = true;
        };

        buffer.push_str(self.buffer.drain(..).as_str());

        if !self.cursor.is_locked() {
            match buffer.rsplit_once('\n') {
                Some((left, right)) => {
                    // NOTE: left.lines().count() does not distinguish
                    // "hello" from "hello\n" (returns 1)
                    let left_lines_count = left
                        .chars()
                        .fold(1, |count, c| count + if c == '\n' { 1 } else { 0 });

                    self.cursor.row += left_lines_count;
                    self.cursor.col = right.len();
                }
                None => {
                    self.cursor.col += buffer.len();
                }
            }
        }

        self.content.push_str(&buffer)
    }
}

impl Deref for TemplateBody {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for TemplateBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

impl From<TemplateBody> for Cow<'_, str> {
    fn from(value: TemplateBody) -> Self {
        value.content.into()
    }
}
