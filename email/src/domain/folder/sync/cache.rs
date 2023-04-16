pub use rusqlite::Error;
use std::collections::HashSet;

use super::{sync, FoldersName, Result};

const CREATE_FOLDERS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS folders (
        account TEXT NOT NULL,
        name    TEXT NOT NULL,
        UNIQUE(name, account)
    )
";

const INSERT_FOLDER: &str = "
    INSERT INTO folders
    VALUES (?, ?)
";

const DELETE_FOLDER: &str = "
    DELETE FROM folders
    WHERE account = ?
    AND name = ?
";

const SELECT_ALL_FOLDERS: &str = "
    SELECT name
    FROM folders
    WHERE account = ?
";

const SELECT_FOLDERS_IN: &str = "
    SELECT name
    FROM folders
    WHERE account = ?
    AND name IN (!)
";

const SELECT_FOLDERS_NOT_IN: &str = "
    SELECT name
    FROM folders
    WHERE account = ?
    AND name NOT IN (!)
";

pub struct Cache;

impl Cache {
    const LOCAL_SUFFIX: &str = ":cache";

    pub fn init(conn: &mut rusqlite::Connection) -> Result<()> {
        conn.execute(CREATE_FOLDERS_TABLE, ())?;
        Ok(())
    }

    fn list_all_folders<A>(conn: &mut rusqlite::Connection, account: A) -> Result<FoldersName>
    where
        A: AsRef<str>,
    {
        let mut stmt = conn.prepare(SELECT_ALL_FOLDERS)?;
        let folders: Vec<String> = stmt
            .query_map([account.as_ref()], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        Ok(FoldersName::from_iter(folders))
    }

    fn list_folders_with<A>(
        conn: &mut rusqlite::Connection,
        account: A,
        folders: &HashSet<String>,
        query: &str,
    ) -> Result<FoldersName>
    where
        A: AsRef<str>,
    {
        let folders = folders
            .iter()
            .map(|f| format!("{f:#?}"))
            .collect::<Vec<_>>()
            .join(", ");

        let mut stmt = conn.prepare(&query.replace("!", &folders))?;

        let folders: Vec<String> = stmt
            .query_map([account.as_ref()], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        Ok(FoldersName::from_iter(folders))
    }

    pub fn list_local_folders<A>(
        conn: &mut rusqlite::Connection,
        account: A,
        strategy: &sync::Strategy,
    ) -> Result<FoldersName>
    where
        A: ToString,
    {
        match strategy {
            sync::Strategy::All => {
                Self::list_all_folders(conn, account.to_string() + Self::LOCAL_SUFFIX)
            }
            sync::Strategy::Include(folders) => Self::list_folders_with(
                conn,
                account.to_string() + Self::LOCAL_SUFFIX,
                folders,
                SELECT_FOLDERS_IN,
            ),
            sync::Strategy::Exclude(folders) => Self::list_folders_with(
                conn,
                account.to_string() + Self::LOCAL_SUFFIX,
                folders,
                SELECT_FOLDERS_NOT_IN,
            ),
        }
    }

    pub fn list_remote_folders<A>(
        conn: &mut rusqlite::Connection,
        account: A,
        strategy: &sync::Strategy,
    ) -> Result<FoldersName>
    where
        A: AsRef<str>,
    {
        match strategy {
            sync::Strategy::All => Self::list_all_folders(conn, account),
            sync::Strategy::Include(folders) => {
                Self::list_folders_with(conn, account, folders, SELECT_FOLDERS_IN)
            }
            sync::Strategy::Exclude(folders) => {
                Self::list_folders_with(conn, account, folders, SELECT_FOLDERS_NOT_IN)
            }
        }
    }

    fn insert_folder<A, F>(tx: &rusqlite::Transaction, account: A, folder: F) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        tx.execute(INSERT_FOLDER, [account.as_ref(), folder.as_ref()])?;
        Ok(())
    }

    pub fn insert_local_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: ToString,
        F: AsRef<str>,
    {
        Self::insert_folder(tx, account.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn insert_remote_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        Self::insert_folder(tx, account, folder)
    }

    fn delete_folder<A, F>(tx: &rusqlite::Transaction, account: A, folder: F) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        tx.execute(DELETE_FOLDER, [account.as_ref(), folder.as_ref()])?;
        Ok(())
    }

    pub fn delete_local_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: ToString,
        F: AsRef<str>,
    {
        Self::delete_folder(tx, account.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn delete_remote_folder<A, F>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        Self::delete_folder(tx, account, folder)
    }
}
