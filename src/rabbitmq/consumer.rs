use anyhow::{Context, Result};
use futures::StreamExt;
use lapin::{
    Channel, Connection, ConnectionProperties, Consumer, ExchangeKind,
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::config::rabbit::RabbitMQConfig;
use crate::parser::TokenLaunch;

pub struct RabbitMQConsumer {
    config: RabbitMQConfig,
    connection: Option<Arc<Connection>>,
    channel: Option<Channel>,
}

impl RabbitMQConsumer {
    pub fn new(config: RabbitMQConfig) -> Self {
        Self {
            config,
            connection: None,
            channel: None,
        }
    }

    /// Set up the connection, exchange and queue
    pub async fn init(&mut self) -> Result<()> {
        info!("🐰 Initializing RabbitMQ consumer...");

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

        // Bind queue
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
        self.channel = Some(channel);

        info!("🚀 RabbitMQ consumer initialized successfully");
        Ok(())
    }

    /// Spawn a background task that consumes messages
    pub async fn start_consuming(&mut self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        if let Some(channel) = &self.channel {
            let consumer = channel
                .basic_consume(
                    &self.config.queue_name,
                    "token_launch_consumer",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .context("Failed to create consumer")?;

            info!(
                "🔍 Started consuming from queue: {}",
                self.config.queue_name
            );

            let handle = tokio::spawn(async move { Self::consume_messages(consumer).await });

            Ok(handle)
        } else {
            Err(anyhow::anyhow!("RabbitMQ consumer not initialized"))
        }
    }

    /// Consume messages loop
    async fn consume_messages(mut consumer: Consumer) -> Result<()> {
        info!("📥 Starting message consumption loop...");

        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    let payload = &delivery.data;

                    match serde_json::from_slice::<TokenLaunch>(payload) {
                        Ok(token_launch) => {
                            info!("📨 Received token launch: {}", token_launch.token_address);

                            // Process token launch
                            if let Err(e) = Self::process_token_launch(&token_launch).await {
                                error!("❌ Error processing token launch: {}", e);
                            }

                            // Acknowledge message
                            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                error!("❌ Failed to acknowledge message: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("❌ Failed to deserialize message: {}", e);
                            // acknowledge malformed message to avoid redeliveries
                            if let Err(ack_err) = delivery.ack(BasicAckOptions::default()).await {
                                error!("❌ Failed to acknowledge malformed message: {}", ack_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Error receiving message: {}", e);
                    // avoid tight loop on errors
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        warn!("📥 Message consumption loop ended");
        Ok(())
    }

    async fn process_token_launch(token_launch: &TokenLaunch) -> Result<()> {
        // Placeholder for business logic
        info!("===================");
        info!("=== CONSUMED TOKEN LAUNCH ===");
        info!("Launchpad: {:?}", token_launch.launchpad);
        info!("CA: {}", token_launch.token_address);
        if let Some(creator) = &token_launch.creator {
            info!("Creator: {}", creator);
        }
        if let Some(name) = &token_launch.metadata.name {
            info!("Name: {}", name);
        }
        if let Some(symbol) = &token_launch.metadata.symbol {
            info!("Symbol: {}", symbol);
        }
        info!("Verify: https://solscan.io/tx/{}", token_launch.signature);
        info!("===================");

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection {
            connection.status().connected()
        } else {
            false
        }
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
