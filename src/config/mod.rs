pub mod error;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber;

use crate::config::error::Result;
use dotenv::dotenv;

pub fn init() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();
    tracing_log::LogTracer::init()?;

    // mock for testing purposes
    info!("Starting task-ba");
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");

    Ok(())
}
