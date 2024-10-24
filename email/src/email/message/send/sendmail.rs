use async_trait::async_trait;
use mail_parser::MessageParser;
use tracing::{debug, info};

use super::SendMessage;
use crate::{email::error::Error, sendmail::SendmailContextSync, AnyResult};

#[derive(Clone)]
pub struct SendSendmailMessage {
    ctx: SendmailContextSync,
}

impl SendSendmailMessage {
    pub fn new(ctx: &SendmailContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &SendmailContextSync) -> Box<dyn SendMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &SendmailContextSync) -> Option<Box<dyn SendMessage>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl SendMessage for SendSendmailMessage {
    async fn send_message(&self, msg: &[u8]) -> AnyResult<()> {
        info!("sending sendmail message");

        let buffer: Vec<u8>;
        let mut msg = MessageParser::new().parse(msg).unwrap_or_else(|| {
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
                Err(_err) => {
                    debug!("cannot execute pre-send hook: {_err}");
                    debug!("{_err:?}");
                }
            }
        };

        self.ctx
            .sendmail_config
            .cmd()
            .run_with(msg.raw_message())
            .await
            .map_err(Error::RunSendmailCommandError)?;

        Ok(())
    }
}
