use async_trait::async_trait;
use log::info;

use crate::{smtp::SmtpClientSync, Result};

use super::SendRawMessage;

#[derive(Clone)]
pub struct SendRawMessageSmtp {
    client: SmtpClientSync,
}

impl SendRawMessageSmtp {
    pub fn new(client: &SmtpClientSync) -> Option<Box<dyn SendRawMessage>> {
        let client = client.clone();
        Some(Box::new(Self { client }))
    }
}

#[async_trait]
impl SendRawMessage for SendRawMessageSmtp {
    async fn send_raw_message(&self, raw_msg: &[u8]) -> Result<()> {
        info!("smtp: sending raw email message");

        let mut client = self.client.lock().await;
        client.send(raw_msg).await?;

        Ok(())
    }
}
