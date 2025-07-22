use task_ba::config;
use task_ba::error::Result;
use task_ba::geyser::GeyserClient;
use tracing::{info, warn};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let (geyser_config, config) = config::init().await?;

    info!("geyser_config: {:?}", geyser_config);
    info!("config: {:?}", config);

    // Create Geyser client with queue size of 5000 transactions (increased for performance)
    let geyser_client = GeyserClient::new(geyser_config, config, 5000);
    
    // Start client in background
    let _geyser_handle = geyser_client.start();
    
    // Start queue monitoring thread with batch processing
    let queue = geyser_client.get_queue().clone();
    let _queue_handle = tokio::spawn(async move {
        info!("Starting queue monitoring thread...");
        
        loop {
            sleep(Duration::from_millis(100)).await; // Much faster processing
            
            // Process transactions in batches for better performance
            let transactions = queue.pop_batch(10).await;
            if !transactions.is_empty() {
                info!("[QUEUE] Processing batch of {} transactions", transactions.len());
                for transaction in transactions {
                    info!("Processing: {} (slot: {}, accounts: {})", 
                          transaction.signature, 
                          transaction.slot,
                          transaction.accounts.len());
                }
            }
        }
    });
    
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
