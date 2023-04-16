use chrono::{DateTime, Local};
use log::warn;
use rusqlite::types::Value;

use crate::{envelope::Mailbox, Envelope, Envelopes};

use super::Result;

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

pub struct Cache;

impl Cache {
    const LOCAL_SUFFIX: &str = ":cache";

    pub fn init(conn: &mut rusqlite::Connection) -> Result<()> {
        conn.execute(CREATE_ENVELOPES_TABLE, ())?;
        Ok(())
    }

    fn list_envelopes<A, F>(
        conn: &mut rusqlite::Connection,
        account: A,
        folder: F,
    ) -> Result<Envelopes>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        let mut stmt = conn.prepare(SELECT_ENVELOPES)?;
        let envelopes: Vec<Envelope> = stmt
            .query_map([account.as_ref(), folder.as_ref()], |row| {
                Ok(Envelope {
                    id: row.get(0)?,
                    internal_id: row.get(1)?,
                    message_id: row.get(2)?,
                    flags: row
                        .get::<usize, Option<String>>(5)?
                        .unwrap_or_default()
                        .as_str()
                        .into(),
                    from: Mailbox::new_nameless(row.get::<usize, String>(6)?),
                    subject: row.get(7)?,
                    date: {
                        let date: String = row.get(8)?;
                        match DateTime::parse_from_rfc3339(&date) {
                            Ok(date) => date.with_timezone(&Local),
                            Err(err) => {
                                warn!("invalid date {}, skipping it: {}", date, err);
                                DateTime::default()
                            }
                        }
                    },
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        Ok(Envelopes::from_iter(envelopes))
    }

    pub fn list_local_envelopes<N, F>(
        conn: &mut rusqlite::Connection,
        name: N,
        folder: F,
    ) -> Result<Envelopes>
    where
        N: ToString,
        F: AsRef<str>,
    {
        Self::list_envelopes(conn, name.to_string() + Self::LOCAL_SUFFIX, folder)
    }

    pub fn list_remote_envelopes<N, F>(
        conn: &mut rusqlite::Connection,
        name: N,
        folder: F,
    ) -> Result<Envelopes>
    where
        N: AsRef<str>,
        F: AsRef<str>,
    {
        Self::list_envelopes(conn, name, folder)
    }

    fn insert_envelope<A, F>(
        transaction: &rusqlite::Transaction,
        account: A,
        folder: F,
        envelope: Envelope,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
    {
        if envelope.flags.is_empty() {
            transaction.execute(
                INSERT_ENVELOPE,
                (
                    &envelope.id,
                    &envelope.internal_id,
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
                        &envelope.internal_id,
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

    pub fn insert_local_envelope<N, F>(
        tx: &rusqlite::Transaction,
        name: N,
        folder: F,
        envelope: Envelope,
    ) -> Result<()>
    where
        N: ToString,
        F: AsRef<str>,
    {
        Self::insert_envelope(tx, name.to_string() + Self::LOCAL_SUFFIX, folder, envelope)
    }

    pub fn insert_remote_envelope<N, F>(
        tx: &rusqlite::Transaction,
        name: N,
        folder: F,
        envelope: Envelope,
    ) -> Result<()>
    where
        N: AsRef<str>,
        F: AsRef<str>,
    {
        Self::insert_envelope(tx, name, folder, envelope)
    }

    fn delete_envelope<A, F, I>(
        tx: &rusqlite::Transaction,
        account: A,
        folder: F,
        internal_id: I,
    ) -> Result<()>
    where
        A: AsRef<str>,
        F: AsRef<str>,
        I: AsRef<str>,
    {
        tx.execute(
            DELETE_ENVELOPE,
            [account.as_ref(), folder.as_ref(), internal_id.as_ref()],
        )?;
        Ok(())
    }

    pub fn delete_local_envelope<N, F, I>(
        tx: &rusqlite::Transaction,
        name: N,
        folder: F,
        internal_id: I,
    ) -> Result<()>
    where
        N: ToString,
        F: AsRef<str>,
        I: AsRef<str>,
    {
        Self::delete_envelope(
            tx,
            name.to_string() + Self::LOCAL_SUFFIX,
            folder,
            internal_id,
        )
    }

    pub fn delete_remote_envelope<N, F, I>(
        tx: &rusqlite::Transaction,
        name: N,
        folder: F,
        internal_id: I,
    ) -> Result<()>
    where
        N: AsRef<str>,
        F: AsRef<str>,
        I: AsRef<str>,
    {
        Self::delete_envelope(tx, name, folder, internal_id)
    }
}
