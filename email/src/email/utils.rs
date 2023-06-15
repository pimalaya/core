use log::debug;
use std::{env, fs, path::PathBuf};

use crate::email::{Error, Result};

pub fn local_draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.eml");
    debug!("local draft path: {}", path.display());
    path
}

pub fn remove_local_draft() -> Result<()> {
    let path = local_draft_path();
    fs::remove_file(&path).map_err(|err| Error::DeleteLocalDraftError(err, path))?;
    Ok(())
}
