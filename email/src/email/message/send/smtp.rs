use async_trait::async_trait;
use log::info;

use crate::{smtp::SmtpClientSync, Result};

use super::SendMessage;

#[derive(Clone)]
pub struct SendMessageSmtp {
    client: SmtpClientSync,
}

impl SendMessageSmtp {
    pub fn new(client: &SmtpClientSync) -> Option<Box<dyn SendMessage>> {
        let client = client.clone();
        Some(Box::new(Self { client }))
    }
}

#[async_trait]
impl SendMessage for SendMessageSmtp {
    async fn send_message(&self, raw_msg: &[u8]) -> Result<()> {
        info!("sending raw smtp message");

        let mut client = self.client.lock().await;
        client.send(raw_msg).await?;

        Ok(())
    }
}
