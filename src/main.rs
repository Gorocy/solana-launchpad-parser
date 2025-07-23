use std::sync::Arc;
use task_ba::config;
use task_ba::error::Result;
use task_ba::geyser::GeyserClient;
use task_ba::parser::ParserManager;
use task_ba::rabbitmq::{RabbitMQProducer};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Install the default Rustls crypto provider (ring) before any TLS/crypto operations
    _ = CryptoProvider::install_default(default_provider());
    let ((geyser_config, config), rabbitmq_cfg) = config::init().await?;

    // Initialize RabbitMQ producer
    let mut producer = RabbitMQProducer::new(rabbitmq_cfg);
    if let Err(e) = producer.init().await {
        error!("Failed to initialize RabbitMQ producer: {e}");
    }
    let producer = Arc::new(producer);

    debug!("geyser_config: {:?}", geyser_config);
    debug!("config: {:?}", config);

    // Create Geyser client with queue size of 5000 transactions (increased for performance)
    let geyser_client = GeyserClient::new(geyser_config, config, 5000);

    // Start client in background
    let _geyser_handle = geyser_client.start();

    // Create parser manager (parsers are automatically registered)
    let parser_manager = ParserManager::new(Some(producer));

    info!("Parser manager initialized with all launchpad parsers");

    // Start parser manager processing
    let queue = geyser_client.get_queue().clone();
    let _parser_handle = tokio::spawn(async move {
        parser_manager
            .start_processing(Arc::new(queue))
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
