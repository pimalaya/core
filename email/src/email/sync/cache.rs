//! Module dedicated to email synchronization cache.
//!
//! This module contains everything needed to manipule email
//! synchronization cache entities using SQLite.

use chrono::DateTime;
use log::debug;
use rusqlite::{types::Value, Connection, Transaction};

use crate::{
    envelope::{Address, Envelope, Envelopes},
    Result,
};

const CREATE_ENVELOPES_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS envelopes (
        id          TEXT     NOT NULL,
        internal_id TEXT     NOT NULL,
        message_id  TEXT     NOT NULL,
        account     TEXT     NOT NULL,
        folder      TEXT     NOT NULL,
        flag        TEXT     DEFAULT NULL,
        sender      TEXT     NOT NULL,
        subject     TEXT     NOT NULL,
        date        DATETIME NOT NULL,
        UNIQUE(internal_id, message_id, account, folder, flag)
    )
";

const INSERT_ENVELOPE: &str = "
    INSERT INTO envelopes
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
";

const DELETE_ENVELOPE: &str = "
    DELETE FROM envelopes
    WHERE account = ?
    AND folder = ?
    AND internal_id = ?
";

const SELECT_ENVELOPES: &str = "
    SELECT id, internal_id, message_id, account, folder, GROUP_CONCAT(flag, ' ') AS flags, sender, subject, date
    FROM envelopes
    WHERE account = ?
    AND folder = ?
    GROUP BY message_id
    ORDER BY date DESC
";

/// The email synchronization cache.
///
/// This structure contains all functions needed to manipule SQLite
/// cache.
pub struct EmailSyncCache;

impl EmailSyncCache {
    const LOCAL_SUFFIX: &'static str = ":cache";

    pub fn init(conn: &mut Connection) -> Result<()> {
        conn.execute(CREATE_ENVELOPES_TABLE, ())?;
        Ok(())
    }

    fn list_envelopes(
        conn: &mut Connection,
        account: impl AsRef<str>,
        folder: impl AsRef<str>,
    ) -> Result<Envelopes> {
        let mut stmt = conn.prepare(SELECT_ENVELOPES)?;
        let envelopes: Vec<Envelope> = stmt
            .query_map([account.as_ref(), folder.as_ref()], |row| {
                Ok(Envelope {
                    id: row.get(1)?,
                    message_id: row.get(2)?,
                    flags: row
                        .get::<usize, Option<String>>(5)?
                        .unwrap_or_default()
                        .as_str()
                        .into(),
                    from: Address::new_nameless(row.get::<usize, String>(6)?),
                    // TODO: add recipient to the database schema
                    to: Address::default(),
                    subject: row.get(7)?,
                    date: {
                        let date: String = row.get(8)?;
                        match DateTime::parse_from_rfc3339(&date) {
                            Ok(date) => date,
                            Err(err) => {
                                debug!("invalid date {date}, skipping it");
                                debug!("{err:?}");
                                DateTime::default()
                            }
                        }
                    },
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        Ok(Envelopes::from_iter(envelopes))
    }

    pub fn list_local_envelopes(
        conn: &mut Connection,
        name: impl ToString,
        folder: impl AsRef<str>,
    ) -> Result<Envelopes> {
        Self::list_envelopes(conn, name.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn list_remote_envelopes(
        conn: &mut Connection,
        name: impl AsRef<str>,
        folder: impl AsRef<str>,
    ) -> Result<Envelopes> {
        Self::list_envelopes(conn, name, folder)
    }

    fn insert_envelope(
        transaction: &Transaction,
        account: impl AsRef<str>,
        folder: impl AsRef<str>,
        envelope: Envelope,
    ) -> Result<()> {
        if envelope.flags.is_empty() {
            transaction.execute(
                INSERT_ENVELOPE,
                (
                    &envelope.id,
                    &envelope.id,
                    &envelope.message_id,
                    account.as_ref(),
                    folder.as_ref(),
                    Value::Null,
                    &envelope.from.addr,
                    &envelope.subject,
                    envelope.date.to_rfc3339(),
                ),
            )?;
        } else {
            for flag in envelope.flags.iter() {
                transaction.execute(
                    INSERT_ENVELOPE,
                    (
                        &envelope.id,
                        &envelope.id,
                        &envelope.message_id,
                        account.as_ref(),
                        folder.as_ref(),
                        flag.to_string(),
                        &envelope.from.addr,
                        &envelope.subject,
                        envelope.date.to_rfc3339(),
                    ),
                )?;
            }
        }

        Ok(())
    }

    pub fn insert_local_envelope(
        tx: &Transaction,
        name: impl ToString,
        folder: impl AsRef<str>,
        envelope: Envelope,
    ) -> Result<()> {
        Self::insert_envelope(tx, name.to_string() + Self::LOCAL_SUFFIX, folder, envelope)
    }

    pub fn insert_remote_envelope(
        tx: &Transaction,
        name: impl AsRef<str>,
        folder: impl AsRef<str>,
        envelope: Envelope,
    ) -> Result<()> {
        Self::insert_envelope(tx, name, folder, envelope)
    }

    fn delete_envelope(
        tx: &Transaction,
        account: impl AsRef<str>,
        folder: impl AsRef<str>,
        internal_id: impl AsRef<str>,
    ) -> Result<()> {
        tx.execute(
            DELETE_ENVELOPE,
            [account.as_ref(), folder.as_ref(), internal_id.as_ref()],
        )?;

        Ok(())
    }

    pub fn delete_local_envelope(
        tx: &Transaction,
        name: impl ToString,
        folder: impl AsRef<str>,
        internal_id: impl AsRef<str>,
    ) -> Result<()> {
        Self::delete_envelope(
            tx,
            name.to_string() + Self::LOCAL_SUFFIX,
            folder,
            internal_id,
        )
    }

    pub fn delete_remote_envelope(
        tx: &Transaction,
        name: impl AsRef<str>,
        folder: impl AsRef<str>,
        internal_id: impl AsRef<str>,
    ) -> Result<()> {
        Self::delete_envelope(tx, name, folder, internal_id)
    }
}
