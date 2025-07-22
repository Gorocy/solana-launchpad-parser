pub mod error;
pub mod grpc;

use tracing::{debug, error, info, trace, warn};
use tracing_subscriber;

use crate::config::{
    error::Result,
    grpc::{Config, GeyserConfig, config_grpc},
};
use dotenv::dotenv;

pub async fn init() -> Result<(GeyserConfig, Config)> {
    dotenv().ok();

    let result = config_grpc();

    tracing_subscriber::fmt::init();
    // tracing_log::LogTracer::init()?;

    // mock for testing purposes
    info!("Starting task-ba");
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");

    result.await
}
