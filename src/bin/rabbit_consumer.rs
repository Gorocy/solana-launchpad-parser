use anyhow::Result;
use rustls::crypto::{CryptoProvider, ring::default_provider};
use task_ba::config::rabbit::RabbitMQConfig;
use task_ba::rabbitmq::RabbitMQConsumer;
use tracing::{info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging and crypto provider
    _ = CryptoProvider::install_default(default_provider());
    tracing_subscriber::fmt::init();

    // Load RabbitMQ configuration from env vars (with defaults)
    let cfg = RabbitMQConfig::from_env().await?;
    info!("Starting standalone RabbitMQ consumer with config: {:?}", cfg);

    let mut consumer = RabbitMQConsumer::new(cfg);
    consumer.init().await?;

    // Start consuming in background
    let handle = consumer.start_consuming().await?;

    info!("Consumer running. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received. Closing consumer...");

    // Graceful shutdown
    consumer.close().await?;
    handle.abort();

    info!("Consumer stopped. Goodbye!");
    Ok(())
} 
