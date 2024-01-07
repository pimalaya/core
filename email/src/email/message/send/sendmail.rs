use async_trait::async_trait;
use log::{debug, info};
use mail_parser::MessageParser;
use thiserror::Error;

use crate::{sendmail::SendmailContext, Result};

use super::SendMessage;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run sendmail command")]
    RunSendmailCommandError(#[source] process::Error),
}

#[derive(Clone)]
pub struct SendMessageSendmail {
    ctx: SendmailContext,
}

impl SendMessageSendmail {
    pub fn new(ctx: &SendmailContext) -> Option<Box<dyn SendMessage>> {
        let ctx = ctx.clone();
        Some(Box::new(Self { ctx }))
    }
}

#[async_trait]
impl SendMessage for SendMessageSendmail {
    async fn send_message(&self, raw_msg: &[u8]) -> Result<()> {
        info!("sending raw sendmail message");

        let buffer: Vec<u8>;
        let mut msg = MessageParser::new().parse(raw_msg).unwrap_or_else(|| {
            debug!("cannot parse raw message");
            Default::default()
        });

        if let Some(cmd) = self.ctx.account_config.find_message_pre_send_hook() {
            match cmd.run_with(msg.raw_message()).await {
                Ok(res) => {
                    buffer = res.into();
                    msg = MessageParser::new().parse(&buffer).unwrap_or_else(|| {
                        debug!("cannot parse raw message after pre-send hook");
                        Default::default()
                    });
                }
                Err(err) => {
                    debug!("cannot execute pre-send hook: {err}");
                    debug!("{err:?}");
                }
            }
        };

        self.ctx
            .sendmail_config
            .cmd
            .run_with(msg.raw_message())
            .await
            .map_err(Error::RunSendmailCommandError)?;

        Ok(())
    }
}
