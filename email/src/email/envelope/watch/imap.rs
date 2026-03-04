use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;
use itertools::{Either, Itertools};

use super::WatchEnvelopes;
use crate::{
    envelope::{Envelope, Envelopes},
    imap::ImapContext, AnyResult
};

#[derive(Clone, Debug)]
pub struct WatchImapEnvelopes {
    ctx: ImapContext,
}

#[derive(Clone, Debug)]
struct SyncState {
    last_seen_uid: Option<u32>,
}

impl WatchImapEnvelopes {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn WatchEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn WatchEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }

    pub async fn watch_envelopes_loop(
        &self,
        folder: &str,
        wait_for_shutdown_request: &mut Receiver<()>,
    ) -> AnyResult<()> {
        info!("watching imap folder {folder} for envelope changes");

        let config = &self.ctx.account_config;
        let mut client = self.ctx.client().await;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let envelopes_count = client
            .examine_mailbox(folder_encoded)
            .await?
            .exists
            .unwrap() as usize;

        let mut sync_state = SyncState {
            last_seen_uid: None,
        };

        // Initial fetch of all envelopes
        debug!("Starting initial fetch...");

        let envelopes = if envelopes_count == 0 {
            Default::default()
        } else {
            client.fetch_all_envelopes().await?
            // Seems like we don't need to actually fetch envelopes for watch, just flags
            // client.fetch_flags("1:*".try_into().unwrap()).await?
        };

        // Track the highest UID we've seen
        sync_state.last_seen_uid = self.get_max_uid(&envelopes);
        debug!("Initial fetch: {} envelopes, last_seen_uid = {}", envelopes.len(), sync_state.last_seen_uid.unwrap());

        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        loop {
            info!("starting new IMAP IDLE loop…");
            client.idle(wait_for_shutdown_request).await?;
            info!("received IDLE change notification or timeout");

            let (new_envelopes, changed_envelopes, expunged_envelopes, last_seen_uid) = self.fetch_envelope_updates(
                &mut client, &mut envelopes, sync_state.last_seen_uid).await?;

            sync_state.last_seen_uid = last_seen_uid;

            let mut next_envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(new_envelopes.iter().by_ref().map(|e| (e.id.clone(), e.clone())));
            next_envelopes.extend(changed_envelopes.iter().by_ref().map(|e| (e.id.clone(), e.clone())));

            // TODO: exec_hooks() should probably take in separate parameters for new/changed/expunged
            self.exec_hooks(config, &envelopes, &next_envelopes).await;

            for env in new_envelopes.into_iter() {
                envelopes.insert(env.id.clone(), env);
            }
            for env in changed_envelopes.into_iter() {
                debug!("Replacing flags for {}: old={:?}, updated={:?}",
                    env.id.clone(),
                    envelopes.get(&env.id.clone()).unwrap().flags,
                    &env.flags);
                envelopes.insert(env.id.clone(), env);
            }
            for env in expunged_envelopes.into_iter() {
                envelopes.remove(&env.id);
            }
        }
    }

    fn get_max_uid(&self, envelopes: &[Envelope]) -> Option<u32> {
        envelopes.iter().filter_map(|e| e.id.parse::<u32>().ok()).max()
    }

    /// Fetch new envelopes and changes to existing envelopes since the last seen UID
    /// See RFC 4549, Section 3 and Section 4.3.1: https://www.rfc-editor.org/rfc/rfc4549
    /// 1) Discover new messages using UID FETCH <lastseenuid+1>:* (ENVELOPE)
    /// 2) Discover changes to existing messages using UID FETCH 1:<lastseenuid> (FLAGS)
    // This will return (new_envelopes, changed_envelopes, expunged_envelopes, new_last_seen_uid)
    async fn fetch_envelope_updates(
        &self,
        client: &mut crate::imap::ImapClient,
        envelope_map: &HashMap<String, Envelope>,
        last_seen_uid: Option<u32>,
    ) -> AnyResult<(Envelopes, Envelopes, Envelopes, Option<u32>)> {
        let (new_envelopes, _changed_envelopes_experimental) = self.fetch_new_envelopes(client, &envelope_map, last_seen_uid.unwrap()).await?;

        let (changed_envelopes, expunged_envelopes) = self.fetch_existing_envelope_changes(
            client, &envelope_map, last_seen_uid.unwrap()).await?;

        let new_last_seen_uid = self.get_max_uid(&new_envelopes).max(last_seen_uid);

        info!("Updated fetch: {} new envelopes, {} changed envelopes, {} expunged envelopes, new_last_seen_uid = {:?}",
            new_envelopes.len(), changed_envelopes.len(), expunged_envelopes.len(), new_last_seen_uid);

        debug!("New envelopes: {:?}", new_envelopes);
        debug!("Changed envelopes (experimental): {:?}", _changed_envelopes_experimental);
        debug!("Changed envelopes: {:?}", changed_envelopes);
        debug!("Expunged envelopes: {:?}", expunged_envelopes);

        Ok((new_envelopes, changed_envelopes, expunged_envelopes, new_last_seen_uid))
    }

    /// Fetch new envelopes since the last seen UID and return (new_envelopes, changed_envelopes)
    async fn fetch_new_envelopes(
        &self,
        client: &mut crate::imap::ImapClient,
        envelope_map: &HashMap<String, Envelope>,
        last_seen_uid: u32,
    ) -> AnyResult<(Envelopes, Envelopes)> {
        debug!("Fetching new envelopes since UID {}", last_seen_uid);

        // Discover new messages using UID FETCH <lastseenuid+1>:* (ENVELOPE)
        let s = format!("{}:*", last_seen_uid + 1);
        let fetched_envelopes = client
            .fetch_envelopes_map(s.as_str().try_into().unwrap())
            .await?;

        // UID FETCH <lastseenuid+1>:* seems to always include lastseenuid, so we have to filter it out
        // It seems some mail servers (e.g. Gmail) will actually return changed messages here as well (in addition to new messages).
        // Perhaps this is an intersection of multiple features like a persistent IMAP connection + IDLE + CONDSTORE?
        let (new_envelopes, changed_envelopes): (Vec<_>, Vec<_>) = fetched_envelopes.into_iter().map(|(_id, fetched)| fetched)
            .filter_map(move |fetched| {
                match envelope_map.get(&fetched.id) {
                    None => Some(Either::Left(fetched)),
                    Some(env) if fetched.flags != env.flags => {
                        Some(Either::Right(fetched))
                    },
                    Some(_) => return None,
                }
            })
            .partition_map(move |env| env);

        Ok((Envelopes::from_iter(new_envelopes), Envelopes::from_iter(changed_envelopes)))
    }

    /// Fetch flags to existing envelopes up to last seen UID, and return (changed_envelopes, expunged_envelopes)
    async fn fetch_existing_envelope_changes(
        &self,
        client: &mut crate::imap::ImapClient,
        envelope_map: &HashMap<String, Envelope>,
        last_seen_uid: u32,
    ) -> AnyResult<(Envelopes, Envelopes)> {
        debug!("Fetching changes to existing envelopes up to UID {}", last_seen_uid);

        // Discover changes to existing messages using UID FETCH 1:<lastseenuid> (FLAGS)
        let s = format!("1:{}", last_seen_uid);
        let fetched_envelopes = client
            .fetch_flags(s.as_str().try_into().unwrap())
            .await?;

        let fetched_envelope_map: HashMap<String, Envelope> =
            HashMap::from_iter(fetched_envelopes.into_iter().map(|env| (env.id.clone(), env)));

        let (changed_envelopes, expunged_envelopes): (Vec<_>, Vec<_>) = envelope_map
            .values()
            .filter_map(|env| {
                match fetched_envelope_map.get(&env.id) {
                    // Loop through previously fetched messages and compare with the updated fetch:
                    // Envelopes that have their flags changed in the updated fetch are updated (e.g. marked as seen)
                    // Envelopes that are not present in the updated fetch are expunged
                    Some(fetched) if fetched.flags != env.flags => {
                        // Clone existing env but replace with the updated fetched flags
                        let mut changed_env = env.clone();
                        changed_env.flags = fetched.flags.clone();
                        Some(Either::Left(changed_env))
                    },
                    Some(_) => return None,
                    None => Some(Either::Right(env.clone())),
                }
            })
            .partition_map(move |env| env);

        Ok((Envelopes::from_iter(changed_envelopes), Envelopes::from_iter(expunged_envelopes)))
    }

}

#[async_trait]
impl WatchEnvelopes for WatchImapEnvelopes {
    async fn watch_envelopes(
        &self,
        folder: &str,
        mut wait_for_shutdown_request: Receiver<()>,
        shutdown: Sender<()>,
    ) -> AnyResult<()> {
        let res = self
            .watch_envelopes_loop(folder, &mut wait_for_shutdown_request)
            .await;

        shutdown.send(()).unwrap();

        res
    }
}
