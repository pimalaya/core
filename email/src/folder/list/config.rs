use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FolderListConfig {
    /// Define the size of a page when listing folders.
    ///
    /// A page size of 0 disables the pagination and displays all
    /// available folders.
    pub page_size: Option<usize>,
}
