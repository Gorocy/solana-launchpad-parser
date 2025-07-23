use crate::geyser::QueuedTransaction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LaunchpadType {
    Pumpfun,
    Meteora,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLaunch {
    pub launchpad: LaunchpadType,
    pub token_address: String,
    pub creator: Option<String>,
    pub signature: String,
    pub slot: u64,
    pub timestamp: DateTime<Utc>,
    pub metadata: LaunchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub initial_supply: Option<u64>,
    pub mint_authority: Option<String>,
}

#[derive(Debug)]
pub enum ParseResult {
    TokenLaunch(TokenLaunch),
    Trade {
        launchpad: LaunchpadType,
        token_address: String,
        trader: String,
        amount: u64,
        signature: String,
        timestamp: DateTime<Utc>,
    },
    Other {
        launchpad: LaunchpadType,
        event_type: String,
        signature: String,
    },
    NotRelevant,
}

pub trait LaunchpadParser: Send + Sync {
    /// Returns the program IDs that this parser handles
    fn get_program_ids(&self) -> Vec<String>;

    /// Parse a transaction and return relevant events
    fn parse_transaction(
        &self,
        transaction: &QueuedTransaction,
    ) -> Result<Vec<ParseResult>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the launchpad type
    fn get_launchpad_type(&self) -> LaunchpadType;
}
