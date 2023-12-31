pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;
use log::debug;
use notify_rust::Notification;
use std::collections::HashMap;

use crate::{account::config::AccountConfig, envelope::Envelope, watch::config::WatchHook, Result};

#[async_trait]
pub trait WatchEnvelopes: Send + Sync {
    /// Watch the given folder for envelopes changes.
    async fn watch_envelopes(&self, folder: &str) -> Result<()>;

    async fn exec_hooks(
        &self,
        config: &AccountConfig,
        prev_envelopes: &HashMap<String, Envelope>,
        next_envelopes: &HashMap<String, Envelope>,
    ) {
        for (id, envelope) in next_envelopes {
            // a new envelope has been added
            if !prev_envelopes.contains_key(id) {
                debug!("processing received envelope event…");
                match config.find_received_envelope_hook() {
                    None => (),
                    Some(WatchHook::Cmd(cmd)) => {
                        debug!("running received envelope hook…");
                        debug!("{}", cmd.to_string());

                        if let Err(err) = cmd.run().await {
                            debug!("error while running received envelope hook: {err}");
                            debug!("{err:?}");
                        }
                    }
                    #[cfg(unix)]
                    Some(WatchHook::Notify(config)) => {
                        debug!("sending received envelope notification…");
                        debug!("{config:?}");

                        let notif = Notification::new()
                            .summary(&resolve_placeholders(&config.summary, envelope))
                            .body(&resolve_placeholders(&config.body, envelope))
                            .show_async()
                            .await;

                        if let Err(err) = notif {
                            debug!("error while sending received envelope notification: {err}");
                            debug!("{err:?}");
                        }
                    }
                    #[cfg(not(unix))]
                    Some(WatchHook::Notify(config)) => {
                        debug!("sending received envelope notification…");
                        debug!("{config:?}");

                        let notif = tokio::task::spawn_blocking(|| {
                            Notification::new()
                                .summary(&resolve_placeholders(&config.summary, envelope))
                                .body(&resolve_placeholders(&config.body, envelope))
                                .show()
                        })
                        .await;

                        if let Err(err) = notif {
                            debug!("error while sending received envelope notification: {err}");
                            debug!("{err:?}");
                            continue;
                        }

                        let notif = notif.unwrap();

                        if let Err(err) = notif {
                            debug!("error while sending received envelope notification: {err}");
                            debug!("{err:?}");
                        }
                    }
                }
            } else {
                match config.find_any_envelope_hook() {
                    None => (),
                    Some(WatchHook::Cmd(cmd)) => {
                        debug!("running any envelope hook…");
                        debug!("{}", cmd.to_string());

                        if let Err(err) = cmd.run().await {
                            debug!("error while running any envelope hook: {err}");
                            debug!("{err:?}");
                        }
                    }
                    #[cfg(unix)]
                    Some(WatchHook::Notify(config)) => {
                        debug!("sending any envelope notification…");
                        debug!("{config:?}");

                        let notif = Notification::new()
                            .summary(&resolve_placeholders(&config.summary, envelope))
                            .body(&resolve_placeholders(&config.body, envelope))
                            .show_async()
                            .await;

                        if let Err(err) = notif {
                            debug!("error while sending any envelope notification: {err}");
                            debug!("{err:?}");
                        }
                    }
                    #[cfg(not(unix))]
                    Some(WatchHook::Notify(config)) => {
                        debug!("sending any envelope notification…");
                        debug!("{config:?}");

                        let notif = tokio::task::spawn_blocking(|| {
                            Notification::new()
                                .summary(&resolve_placeholders(&config.summary, envelope))
                                .body(&resolve_placeholders(&config.body, envelope))
                                .show()
                        })
                        .await;

                        if let Err(err) = notif {
                            debug!("error while sending any envelope notification: {err}");
                            debug!("{err:?}");
                            continue;
                        }

                        let notif = notif.unwrap();

                        if let Err(err) = notif {
                            debug!("error while sending any envelope notification: {err}");
                            debug!("{err:?}");
                        }
                    }
                }
            }
        }

        // TODO: manager other cases
    }
}

fn resolve_placeholders(fmt: &str, envelope: &Envelope) -> String {
    let sender = envelope
        .from
        .name
        .as_ref()
        .map(String::as_str)
        .unwrap_or(&envelope.from.addr);
    let sender_name = envelope
        .from
        .name
        .as_ref()
        .map(String::as_str)
        .unwrap_or("unknown");
    let recipient = envelope
        .to
        .name
        .as_ref()
        .map(String::as_str)
        .unwrap_or(&envelope.to.addr);
    let recipient_name = envelope
        .to
        .name
        .as_ref()
        .map(String::as_str)
        .unwrap_or("unknown");

    fmt.replace("{id}", &envelope.id)
        .replace("{subject}", &envelope.subject)
        .replace("{sender}", sender)
        .replace("{sender.name}", sender_name)
        .replace("{sender.address}", &envelope.from.addr)
        .replace("{recipient}", recipient)
        .replace("{recipient.name}", recipient_name)
        .replace("{recipient.address}", &envelope.to.addr)
}
