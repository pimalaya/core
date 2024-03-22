/// Configuration dedicated to message deletion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct DeleteMessageConfig {
    /// The message deletion style.
    ///
    /// Message deletion can be performed either by moving messages to
    /// the Trash folder or by adding the Deleted flag to their
    /// respective envelopes.
    pub style: Option<DeleteMessageStyle>,
}

/// The message deletion style.
///
/// Message deletion can be performed either by moving messages to the
/// Trash folder or by adding the Deleted flag to their respective
/// envelopes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum DeleteMessageStyle {
    /// The folder-based message deletion style.
    ///
    /// This style uses the Trash folder as primary source of
    /// deletion. Deleted messages are move to this folder. When a
    /// message is deleted from the Trash folder itself, the flag
    /// deletion style is applied.
    #[default]
    Folder,

    /// The flag-based message deletion style.
    ///
    /// This style uses the Deleted flag as primary source of
    /// deletion. Delete messages' respective envelopes receive the
    /// Deleted flag. The only way to definitely delete those messages
    /// is to expunge the folder they belong to.
    Flag,
}

impl DeleteMessageStyle {
    /// Return `true` if the current message deletion style matches
    /// the folder-based message deletion style.
    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder)
    }

    /// Return `true` if the current message deletion style matches
    /// the flag-based message deletion style.
    pub fn is_flag(&self) -> bool {
        matches!(self, Self::Flag)
    }
}
