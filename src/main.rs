use std::sync::Arc;
use task_ba::config;
use task_ba::error::Result;
use task_ba::geyser::GeyserClient;
use task_ba::parser::ParserManager;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let (geyser_config, config) = config::init().await?;

    debug!("geyser_config: {:?}", geyser_config);
    debug!("config: {:?}", config);

    // Create Geyser client with queue size of 5000 transactions (increased for performance)
    let geyser_client = GeyserClient::new(geyser_config, config, 5000);

    // Start client in background
    let _geyser_handle = geyser_client.start();

    // Create parser manager (parsers are automatically registered)
    let parser_manager = ParserManager::new();

    info!("Parser manager initialized with all launchpad parsers");

    // Start parser manager processing
    let queue = geyser_client.get_queue().clone();
    let _parser_handle = tokio::spawn(async move {
        parser_manager
            .start_processing(Arc::new(queue.clone()))
            .await;
    });

    info!("Parser manager started successfully");

    // Main application loop with reduced logging frequency
    let main_queue = geyser_client.get_queue().clone();
    loop {
        sleep(Duration::from_secs(10)).await;
        let queue_size = main_queue.len().await;
        if queue_size > 1000 {
            warn!("Queue status: {} elements", queue_size);
        } else if queue_size > 0 {
            info!("Queue status: {} elements", queue_size);
        }
    }
}
