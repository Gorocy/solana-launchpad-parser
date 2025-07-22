use thiserror::Error;

use crate::config::error::ErrorConfig;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ErrorConfig),
}
