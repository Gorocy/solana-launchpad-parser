use task_ba::config;
use task_ba::error::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let (geyser_config, config) = config::init().await?;

    info!("geyser_config: {:?}", geyser_config);
    info!("config: {:?}", config);

    Ok(())
}
