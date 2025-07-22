use anyhow::{Context, Result};
use bs58;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use solana_stream_sdk::{
    GeyserGrpcClient, GeyserSubscribeRequest, GeyserSubscribeUpdate, GeyserUpdateOneof,
};
use std::time::Duration;
use tokio::task::JoinHandle;
use tonic::transport::ClientTlsConfig;
use tracing::{error, info, warn};

use crate::config::grpc::{Config, GeyserConfig, commitment_from_str};
use crate::geyser::{QueuedTransaction, TransactionQueue};

/// Main Geyser client
pub struct GeyserClient {
    geyser_config: GeyserConfig,
    config: Config,
    transaction_queue: TransactionQueue,
}

impl GeyserClient {
    /// Creates a new Geyser client
    pub fn new(geyser_config: GeyserConfig, config: Config, queue_size: usize) -> Self {
        Self {
            geyser_config,
            config,
            transaction_queue: TransactionQueue::new(queue_size),
        }
    }

    /// Returns reference to transaction queue
    pub fn get_queue(&self) -> &TransactionQueue {
        &self.transaction_queue
    }

    /// Builds subscription request based on configuration
    fn build_subscribe_request(&self) -> GeyserSubscribeRequest {
        use solana_stream_sdk::{
            GeyserSubscribeRequestFilterAccounts, GeyserSubscribeRequestFilterBlocks,
            GeyserSubscribeRequestFilterBlocksMeta, GeyserSubscribeRequestFilterEntry,
            GeyserSubscribeRequestFilterSlots, GeyserSubscribeRequestFilterTransactions,
        };

        GeyserSubscribeRequest {
            commitment: self.config.commitment.as_deref().map(commitment_from_str),
            transactions: self
                .config
                .transactions
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterTransactions::from(v)))
                .collect(),
            accounts: self
                .config
                .accounts
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterAccounts::from(v)))
                .collect(),
            slots: self
                .config
                .slots
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterSlots::from(v)))
                .collect(),
            blocks: self
                .config
                .blocks
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterBlocks::from(v)))
                .collect(),
            blocks_meta: self
                .config
                .blocks_meta
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterBlocksMeta::from(v)))
                .collect(),
            entry: self
                .config
                .entry
                .iter()
                .map(|(k, v)| (k.clone(), GeyserSubscribeRequestFilterEntry::from(v)))
                .collect(),
            transactions_status: Default::default(),
            accounts_data_slice: vec![],
            from_slot: None,
            ping: None,
        }
    }

    /// Processes Geyser message and adds relevant transactions to queue
    async fn process_message(&self, msg: &GeyserSubscribeUpdate) {
        match &msg.update_oneof {
            Some(GeyserUpdateOneof::Transaction(tx_info)) => {
                let received_time = Utc::now();
                let slot = tx_info.slot;

                if let Some(tx) = &tx_info.transaction {
                    if let Some(inner_tx) = &tx.transaction {
                        // Get transaction signature
                        if let Some(sig) = inner_tx.signatures.first() {
                            let signature = bs58::encode(sig).into_string();

                            // Collect all accounts from transaction
                            let mut accounts = Vec::new();

                            // Add accounts from account_keys
                            if let Some(message) = &inner_tx.message {
                                for account_key in &message.account_keys {
                                    accounts.push(bs58::encode(account_key).into_string());
                                }
                            }

                            // Check if transaction contains accounts of interest
                            let should_queue = self.should_queue_transaction(&accounts);

                            if should_queue {
                                let queued_tx = QueuedTransaction {
                                    signature: signature.clone(),
                                    slot,
                                    received_time,
                                    accounts,
                                };

                                self.transaction_queue.push(queued_tx).await;
                                // Reduced logging frequency for performance
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Checks if transaction should be queued based on contained accounts
    fn should_queue_transaction(&self, transaction_accounts: &[String]) -> bool {
        // Check transaction filters from configuration
        for (_filter_name, tx_filter) in &self.config.transactions {
            if let Some(account_include) = &tx_filter.account_include {
                // Check if transaction contains any accounts of interest
                for target_account in account_include {
                    if transaction_accounts.contains(target_account) {
                        return true;
                    }
                }
            }

            if let Some(account_required) = &tx_filter.account_required {
                // Check if transaction contains all required accounts
                let has_all_required = account_required
                    .iter()
                    .all(|required_account| transaction_accounts.contains(required_account));

                if has_all_required {
                    return true;
                }
            }
        }

        false
    }

    /// Starts Geyser client in separate task
    pub fn start(&self) -> JoinHandle<Result<()>> {
        let geyser_config = self.geyser_config.clone();
        let request = self.build_subscribe_request();
        let transaction_queue = self.transaction_queue.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            info!("Starting Geyser client...");

            loop {
                if let Err(e) =
                    Self::run_stream_loop(&geyser_config, &request, &transaction_queue, &config)
                        .await
                {
                    error!("Error in Geyser stream: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        })
    }

    /// Main loop for handling Geyser stream
    async fn run_stream_loop(
        geyser_config: &GeyserConfig,
        request: &GeyserSubscribeRequest,
        transaction_queue: &TransactionQueue,
        config: &Config,
    ) -> Result<()> {
        // Connect to Geyser GRPC
        info!("Connecting to Geyser GRPC: {}", geyser_config.grpc_endpoint);

        let mut builder = GeyserGrpcClient::build_from_shared(geyser_config.grpc_endpoint.clone())
            .context("Failed to build GRPC client")?;

        builder = builder
            .x_token(Some(geyser_config.x_token.clone()))
            .context("Failed to set token")?;

        if geyser_config.grpc_endpoint.starts_with("https://") {
            builder = builder
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .context("Failed to configure TLS")?;
        }

        let mut client = builder
            .connect()
            .await
            .context("Cannot connect to Geyser GRPC")?;

        info!("Connected to Geyser GRPC");

        // Create bidirectional stream
        let (mut sink, mut stream) = client.subscribe().await?;

        // Send subscription request
        sink.send(request.clone()).await?;
        info!("Sent Geyser subscription request");

        // Main message receiving loop
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    // Create temporary client for message processing
                    let temp_client = GeyserClient {
                        geyser_config: geyser_config.clone(),
                        config: config.clone(),
                        transaction_queue: transaction_queue.clone(),
                    };

                    temp_client.process_message(&msg).await;
                }
                Err(e) => {
                    error!("Stream error: {:?}, reconnecting...", e);
                    return Err(e.into());
                }
            }
        }

        warn!("Stream ended, reconnecting...");
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}
