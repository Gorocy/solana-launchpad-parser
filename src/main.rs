use task_ba::config;
use task_ba::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    config::init()?;

    Ok(())
}
