use crate::geyser::{QueuedTransaction, TransactionQueue};
use crate::parser::{LaunchpadParser, ParseResult, TokenLaunch};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{error, info, trace, warn};

pub struct ParserManager {
    parsers: Vec<Box<dyn LaunchpadParser + Send + Sync>>,
    program_id_to_parser: HashMap<String, usize>,
}

impl ParserManager {
    pub fn new() -> Self {
        let mut parsers: Vec<Box<dyn LaunchpadParser + Send + Sync>> = Vec::new();
        let mut program_id_to_parser = HashMap::new();

        // Add PumpFun parser
        let pumpfun_parser = Box::new(crate::parser::pumpfun::PumpfunParser::new());
        let parser_index = parsers.len();
        for program_id in pumpfun_parser.get_program_ids() {
            program_id_to_parser.insert(program_id, parser_index);
        }
        parsers.push(pumpfun_parser);

        // Add Meteora DBC parser
        let meteora_parser = Box::new(crate::parser::meteora::MeteoraParser::new());
        let parser_index = parsers.len();
        for program_id in meteora_parser.get_program_ids() {
            program_id_to_parser.insert(program_id, parser_index);
        }
        parsers.push(meteora_parser);

        Self {
            parsers,
            program_id_to_parser,
        }
    }

    /// Start processing transactions from the queue
    pub async fn start_processing(&self, queue: Arc<TransactionQueue>) {
        info!("🚀 Starting transaction parser manager");

        loop {
            let transactions = queue.pop_batch(10).await;

            if transactions.is_empty() {
                sleep(Duration::from_millis(1)).await;
                continue;
            }

            trace!("📦 Processing batch of {} transactions", transactions.len());

            for transaction in transactions {
                if let Err(e) = self.process_transaction(&transaction).await {
                    error!(
                        "❌ Error processing transaction {}: {}",
                        transaction.signature, e
                    );
                }
            }
        }
    }

    /// Process a single transaction
    async fn process_transaction(
        &self,
        transaction: &QueuedTransaction,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut relevant_parsers = Vec::new();

        // Check which parsers should handle this transaction based on program IDs in instructions
        for instruction in &transaction.instructions {
            if let Some(&parser_index) = self.program_id_to_parser.get(&instruction.program_id) {
                if !relevant_parsers.contains(&parser_index) {
                    relevant_parsers.push(parser_index);
                }
            }
        }

        if relevant_parsers.is_empty() {
            return Ok(());
        }

        // Process with each relevant parser
        for &parser_index in &relevant_parsers {
            if let Some(parser) = self.parsers.get(parser_index) {
                match parser.parse_transaction(transaction) {
                    Ok(results) => {
                        for result in results {
                            match result {
                                ParseResult::TokenLaunch(launch) => {
                                    self.handle_token_launch(launch).await?;
                                }
                                ParseResult::Trade { .. } => {
                                    // Skip trading events for now, only interested in launches
                                }
                                ParseResult::Other { .. } => {
                                    // Skip other events for now, only interested in launches
                                }
                                ParseResult::NotRelevant => {
                                    // Skip irrelevant transactions
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  Parser error for {}: {}", transaction.signature, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a detected token launch
    async fn handle_token_launch(
        &self,
        launch: TokenLaunch,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("===================");
        info!("=== TOKEN LAUNCH ===");
        info!("Launchpad: {:?}", launch.launchpad);
        info!("CA: {}", launch.token_address);
        if let Some(creator) = &launch.creator {
            info!("Creator: {}", creator);
        }
        if let Some(name) = &launch.metadata.name {
            info!("Name: {}", name);
        }
        if let Some(symbol) = &launch.metadata.symbol {
            info!("Symbol: {}", symbol);
        }
        info!("Verify: https://solscan.io/tx/{}", launch.signature);
        info!("===================");

        Ok(())
    }
}
