use thiserror::Error;
use tracing_log::log::SetLoggerError;

pub type Result<T> = std::result::Result<T, ErrorConfig>;

#[derive(Error, Debug)]
pub enum ErrorConfig {
    #[error(transparent)]
    TracingLog(#[from] SetLoggerError),
}
