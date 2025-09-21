pub mod telegram;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub platform: String,
    pub chat_id: String,
    pub user_id: String,
    pub text: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait Messenger {
    async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn platform(&self) -> &str;
}

pub struct MessengerManager {
    messengers: Vec<Box<dyn Messenger + Send + Sync>>,
}

impl MessengerManager {
    pub fn new() -> Self {
        Self {
            messengers: Vec::new(),
        }
    }

    pub fn add_messenger(&mut self, messenger: Box<dyn Messenger + Send + Sync>) {
        self.messengers.push(messenger);
    }

    pub async fn start_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for messenger in &self.messengers {
            messenger.start().await?;
        }
        Ok(())
    }

    pub async fn send_message(
        &self,
        platform: &str,
        chat_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for messenger in &self.messengers {
            if messenger.platform() == platform {
                return messenger.send_message(chat_id, text).await;
            }
        }
        Err(format!("No messenger found for platform: {}", platform).into())
    }
}
