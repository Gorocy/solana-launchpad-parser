use crate::config::error::Result;
use std::env;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct RabbitMQConfig {
    pub url: String,
    pub exchange_name: String,
    pub queue_name: String,
    pub routing_key: String,
}

impl RabbitMQConfig {
    /// Load RabbitMQ configuration from environment variables, providing sensible defaults
    pub async fn from_env() -> Result<Self> {
        info!("Loading RabbitMQ configuration from environment");

        debug!("Getting RABBITMQ_URL from env");
        let url = env::var("RABBITMQ_URL")
            .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

        debug!("Getting RABBITMQ_EXCHANGE from env");
        let exchange_name =
            env::var("RABBITMQ_EXCHANGE").unwrap_or_else(|_| "token_launches".to_string());

        debug!("Getting RABBITMQ_QUEUE from env");
        let queue_name =
            env::var("RABBITMQ_QUEUE").unwrap_or_else(|_| "launches_queue".to_string());

        debug!("Getting RABBITMQ_ROUTING_KEY from env");
        let routing_key =
            env::var("RABBITMQ_ROUTING_KEY").unwrap_or_else(|_| "launch.detected".to_string());

        Ok(Self {
            url,
            exchange_name,
            queue_name,
            routing_key,
        })
    }
}
