use log::{debug, info, trace};
use std::result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get internal id from id {0}")]
    GetInternalIdFromId(String),
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub struct IdMapper {
    account: String,
    folder: String,
    db: rusqlite::Connection,
}

impl IdMapper {
    fn build_table_name<A, F>(account: A, folder: F) -> String
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        let hash = md5::compute(account.as_ref().to_owned() + folder.as_ref());
        format!("id_mapper_{hash:x}")
    }

    pub fn new<A, F>(db: rusqlite::Connection, account: A, folder: F) -> Result<Self>
    where
        A: AsRef<str> + ToString,
        F: AsRef<str> + ToString,
    {
        info!(
            "creating a new id mapper for account {} and folder {}",
            account.as_ref(),
            folder.as_ref(),
        );

        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                internal_id TEXT UNIQUE
            )",
            Self::build_table_name(account.as_ref(), folder.as_ref())
        );

        db.execute(&create_table, [])?;

        Ok(Self {
            account: account.to_string(),
            folder: folder.to_string(),
            db,
        })
    }

    fn table_name(&self) -> String {
        Self::build_table_name(&self.account, &self.folder)
    }

    pub fn insert<I>(&self, internal_id: I) -> Result<String>
    where
        I: AsRef<str>,
    {
        info!(
            "inserting internal id {} to id mapper",
            internal_id.as_ref()
        );

        self.db.execute(
            &format!(
                "INSERT OR IGNORE INTO {} (internal_id) VALUES (?)",
                self.table_name(),
            ),
            [internal_id.as_ref()],
        )?;

        let id = self.db.last_insert_rowid().to_string();
        debug!("last inserted id: {id}");

        Ok(id)
    }

    pub fn get_id<I>(&self, internal_id: I) -> Result<String>
    where
        I: AsRef<str> + ToString,
    {
        info!("getting id from internal id {}", internal_id.as_ref());

        let mut stmt = self.db.prepare(&format!(
            "SELECT id FROM {} WHERE internal_id = ?",
            self.table_name()
        ))?;

        let ids: Vec<usize> = stmt
            .query_map([internal_id.as_ref()], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let id = match ids.first() {
            Some(id) => id.to_string(),
            None => self.insert(internal_id)?,
        };
        debug!("id: {id}");

        Ok(id)
    }

    pub fn get_internal_id<I>(&self, id: I) -> Result<String>
    where
        I: AsRef<str> + ToString,
    {
        info!("getting internal id from id {}", id.as_ref());

        let mut stmt = self.db.prepare(&format!(
            "SELECT internal_id FROM {} WHERE id = ?",
            self.table_name()
        ))?;

        let internal_ids: Vec<String> = stmt
            // TODO: id should be a usize instead of a string
            .query_map([id.as_ref().parse::<usize>().unwrap()], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let internal_id = internal_ids
            .first()
            .ok_or_else(|| Error::GetInternalIdFromId(id.to_string()))?
            .to_string();
        trace!("interal id: {internal_id}");

        Ok(internal_id)
    }
}
