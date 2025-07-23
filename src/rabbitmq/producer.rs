use anyhow::{Context, Result};
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::config::rabbit::RabbitMQConfig;
use crate::parser::TokenLaunch;

#[derive(Clone)]
pub struct RabbitMQProducer {
    config: RabbitMQConfig,
    connection: Option<Arc<Connection>>,
    channel: Option<Arc<Mutex<Channel>>>,
}

impl RabbitMQProducer {
    pub fn new(config: RabbitMQConfig) -> Self {
        Self {
            config,
            connection: None,
            channel: None,
        }
    }

    /// Initialize connection, exchange and queue declarations
    pub async fn init(&mut self) -> Result<()> {
        info!("🐰 Initializing RabbitMQ producer...");

        // Create connection
        let connection = Connection::connect(&self.config.url, ConnectionProperties::default())
            .await
            .context("Failed to connect to RabbitMQ")?;

        info!("✅ Connected to RabbitMQ: {}", self.config.url);

        // Create channel
        let channel = connection
            .create_channel()
            .await
            .context("Failed to create channel")?;

        info!("✅ Created RabbitMQ channel");

        // Declare exchange
        channel
            .exchange_declare(
                &self.config.exchange_name,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .context("Failed to declare exchange")?;

        debug!("✅ Declared exchange: {}", self.config.exchange_name);

        // Declare queue
        channel
            .queue_declare(
                &self.config.queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .context("Failed to declare queue")?;

        debug!("✅ Declared queue: {}", self.config.queue_name);

        // Bind queue to exchange
        channel
            .queue_bind(
                &self.config.queue_name,
                &self.config.exchange_name,
                &self.config.routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to bind queue to exchange")?;

        debug!(
            "✅ Bound queue {} to exchange {} with routing key {}",
            self.config.queue_name, self.config.exchange_name, self.config.routing_key
        );

        self.connection = Some(Arc::new(connection));
        self.channel = Some(Arc::new(Mutex::new(channel)));

        info!("🚀 RabbitMQ producer initialized successfully");
        Ok(())
    }

    /// Publish a token launch event to RabbitMQ
    pub async fn publish_token_launch(&self, token_launch: &TokenLaunch) -> Result<()> {
        if let Some(channel_arc) = &self.channel {
            let channel = channel_arc.lock().await;

            // Serialize token launch to JSON
            let payload =
                serde_json::to_vec(token_launch).context("Failed to serialize token launch")?;

            // Publish message
            channel
                .basic_publish(
                    &self.config.exchange_name,
                    &self.config.routing_key,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default()
                        .with_content_type("application/json".into())
                        .with_delivery_mode(2), // Persistent message
                )
                .await
                .context("Failed to publish message")?;

            debug!(
                "📤 Published token launch to RabbitMQ: {} ({})",
                token_launch.token_address, token_launch.signature
            );

            Ok(())
        } else {
            Err(anyhow::anyhow!("RabbitMQ producer not initialized"))
        }
    }

    /// Simple health-check helper
    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection {
            connection.status().connected()
        } else {
            false
        }
    }

    /// Attempt to reconnect on connection loss
    pub async fn reconnect(&mut self) -> Result<()> {
        warn!("🔄 Attempting to reconnect to RabbitMQ...");
        self.connection = None;
        self.channel = None;
        self.init().await
    }

    /// Gracefully close connection
    pub async fn close(&self) -> Result<()> {
        if let Some(connection) = &self.connection {
            connection
                .close(200, "Normal shutdown")
                .await
                .context("Failed to close connection")?;
            info!("✅ RabbitMQ connection closed gracefully");
        }
        Ok(())
    }
}
