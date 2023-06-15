use log::debug;
use std::{env, fs, io, path::PathBuf};

pub fn local_draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.eml");
    debug!("local draft path: {}", path.display());
    path
}

pub fn remove_local_draft() -> io::Result<()> {
    let path = local_draft_path();
    fs::remove_file(&path)?;
    Ok(())
}
