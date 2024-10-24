use async_trait::async_trait;
use tracing::info;

use super::SendMessage;
use crate::{smtp::SmtpContextSync, AnyResult};

#[derive(Clone)]
pub struct SendSmtpMessage {
    ctx: SmtpContextSync,
}

impl SendSmtpMessage {
    pub fn new(ctx: &SmtpContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &SmtpContextSync) -> Box<dyn SendMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &SmtpContextSync) -> Option<Box<dyn SendMessage>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl SendMessage for SendSmtpMessage {
    async fn send_message(&self, msg: &[u8]) -> AnyResult<()> {
        info!("sending smtp message");

        let mut ctx = self.ctx.lock().await;
        ctx.send(msg).await?;

        Ok(())
    }
}
