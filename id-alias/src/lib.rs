use log::{debug, trace};
use std::{path::PathBuf, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open id alias database at {1}")]
    OpenDbError(#[source] rusqlite::Error, PathBuf),
    #[error("cannot create id alias database table {1}")]
    CreateTableError(#[source] rusqlite::Error, String),
    #[error("cannot create alias for id {1}")]
    CreateAliasError(#[source] rusqlite::Error, String),
    #[error("cannot get alias for id {1}")]
    GetAliasByIdError(#[source] rusqlite::Error, String),
    #[error("cannot get id for alias {1}")]
    GetIdByAliasError(#[source] rusqlite::Error, i64),
    #[error("cannot find id for alias {0}")]
    GetIdByAliasNotFoundError(i64),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct IdAlias {
    table: String,
    conn: rusqlite::Connection,
}

impl IdAlias {
    pub fn new<P, K>(path: P, key: K) -> Result<Self>
    where
        K: AsRef<str>,
        P: Into<PathBuf>,
    {
        let path = &path.into();
        let digest = md5::compute(key.as_ref());
        let table = format!("id_alias_{digest:x}");
        debug!("creating id alias table {table} at {path:?}…");

        let conn = rusqlite::Connection::open(path)
            .map_err(|err| Error::OpenDbError(err, path.clone()))?;

        let query = format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                internal_id TEXT UNIQUE
            )",
        );
        trace!("create table query: {query:#?}");

        conn.execute(&query, [])
            .map_err(|err| Error::CreateTableError(err, table.clone()))?;

        Ok(Self { table, conn })
    }

    pub fn create<I>(&self, id: I) -> Result<i64>
    where
        I: AsRef<str>,
    {
        let id = id.as_ref();
        debug!("creating alias for {id}…");

        let query = format!(
            "INSERT OR IGNORE INTO {} (internal_id) VALUES (?)",
            self.table
        );
        trace!("insert query: {query:#?}");

        self.conn
            .execute(&query, [id])
            .map_err(|err| Error::CreateAliasError(err, id.to_owned()))?;

        let alias = self.conn.last_insert_rowid();
        debug!("created alias {alias} for id {id}");

        Ok(alias)
    }

    pub fn get_or_create_alias<I>(&self, id: I) -> Result<i64>
    where
        I: AsRef<str>,
    {
        let id = id.as_ref();
        debug!("getting alias for id {id}…");

        let query = format!("SELECT id FROM {} WHERE internal_id = ?", self.table);
        trace!("select query: {query:#?}");

        let mut stmt = self
            .conn
            .prepare(&query)
            .map_err(|err| Error::GetAliasByIdError(err, id.to_owned()))?;
        let aliases: Vec<i64> = stmt
            .query_map([id], |row| row.get(0))
            .map_err(|err| Error::GetAliasByIdError(err, id.to_owned()))?
            .collect::<rusqlite::Result<_>>()
            .map_err(|err| Error::GetAliasByIdError(err, id.to_owned()))?;
        let alias = match aliases.first() {
            Some(alias) => {
                debug!("found alias {alias} for id {id}");
                *alias
            }
            None => {
                debug!("alias not found, creating it…");
                self.create(id)?
            }
        };

        Ok(alias)
    }

    pub fn get_id<A>(&self, alias: A) -> Result<String>
    where
        A: Into<i64>,
    {
        let alias = alias.into();
        debug!("getting id for alias {alias}…");

        let query = format!("SELECT internal_id FROM {} WHERE id = ?", self.table);
        trace!("select query: {query:#?}");

        let mut stmt = self
            .conn
            .prepare(&query)
            .map_err(|err| Error::GetIdByAliasError(err, alias))?;
        let ids: Vec<String> = stmt
            .query_map([alias], |row| row.get(0))
            .map_err(|err| Error::GetIdByAliasError(err, alias))?
            .collect::<rusqlite::Result<_>>()
            .map_err(|err| Error::GetIdByAliasError(err, alias))?;
        let id = ids
            .first()
            .ok_or_else(|| Error::GetIdByAliasNotFoundError(alias))?
            .to_owned();
        debug!("found id {id} for alias {alias}");

        Ok(id)
    }
}
