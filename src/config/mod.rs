pub mod error;
pub mod grpc;
pub mod rabbit;

use tracing::{debug, error, info, trace, warn};
use tracing_subscriber;

use crate::config::{
    rabbit::RabbitMQConfig, error::Result, grpc::{config_grpc, Config, GeyserConfig}
};
use dotenv::dotenv;

pub async fn init() -> Result<((GeyserConfig, Config), RabbitMQConfig)> {
    dotenv().ok();

    let result = config_grpc();
    let rabbitmq_config = rabbit::RabbitMQConfig::from_env();
    tracing_subscriber::fmt::init();
    // tracing_log::LogTracer::init()?;

    // mock for testing purposes
    info!("Starting task-ba");
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");

    Ok((result.await?, rabbitmq_config.await?))
}
