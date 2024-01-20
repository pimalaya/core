use async_trait::async_trait;
use log::info;

use crate::{smtp::SmtpContextSync, Result};

use super::SendMessage;

#[derive(Clone)]
pub struct SendSmtpMessage {
    ctx: SmtpContextSync,
}

impl SendSmtpMessage {
    pub fn new(ctx: impl Into<SmtpContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<SmtpContextSync>) -> Box<dyn SendMessage> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl SendMessage for SendSmtpMessage {
    async fn send_message(&self, msg: &[u8]) -> Result<()> {
        info!("sending smtp message");

        let mut ctx = self.ctx.lock().await;
        ctx.send(msg).await?;

        Ok(())
    }
}
