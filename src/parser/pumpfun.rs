use crate::geyser::QueuedTransaction;
use crate::parser::{
    LaunchpadParser, LaunchpadType, ParseResult, TokenLaunch, launchpad_parser::LaunchMetadata,
};
use tracing::{debug, info};

pub struct PumpfunParser {
    program_id: String,
}

impl PumpfunParser {
    pub fn new() -> Self {
        Self {
            program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string(),
        }
    }
}

impl LaunchpadParser for PumpfunParser {
    fn get_program_ids(&self) -> Vec<String> {
        vec![self.program_id.clone()]
    }

    fn get_launchpad_type(&self) -> LaunchpadType {
        LaunchpadType::Pumpfun
    }

    fn parse_transaction(
        &self,
        transaction: &QueuedTransaction,
    ) -> Result<Vec<ParseResult>, Box<dyn std::error::Error + Send + Sync>> {
        debug!("🔍 Parsing PumpFun transaction: {}", transaction.signature);

        // Check for create instruction discriminator: [24, 30, 200, 40, 5, 28, 7, 119]
        for instr in &transaction.instructions {
            if instr.program_id == self.program_id
                && instr.data.len() >= 8
                && instr.data[0..8] == [24, 30, 200, 40, 5, 28, 7, 119]
            {
                info!(
                    "🎯 Found PumpFun CREATE instruction in: {}",
                    transaction.signature
                );

                if let Some(token_launch) = self.extract_token_launch(transaction, instr)? {
                    return Ok(vec![ParseResult::TokenLaunch(token_launch)]);
                }
            }
        }

        Ok(vec![ParseResult::NotRelevant])
    }
}

impl PumpfunParser {
    /// Extract token launch information from create instruction
    fn extract_token_launch(
        &self,
        transaction: &QueuedTransaction,
        instruction: &crate::geyser::TransactionInstruction,
    ) -> Result<Option<TokenLaunch>, Box<dyn std::error::Error + Send + Sync>> {
        // Try to find the mint from instruction accounts
        // According to IDL, account 0 should be the mint
        if let Some(mint_idx) = instruction.accounts.get(0) {
            if let Some(mint_address) = transaction.accounts.get(*mint_idx as usize) {
                let creator = transaction.accounts.get(0).cloned();

                let token_launch = TokenLaunch {
                    launchpad: LaunchpadType::Pumpfun,
                    token_address: mint_address.clone(),
                    creator,
                    signature: transaction.signature.clone(),
                    slot: transaction.slot,
                    timestamp: transaction.received_time,
                    metadata: self.extract_metadata_from_instruction(&instruction.data),
                };

                debug!("✅ Extracted PumpFun token launch: {}", mint_address);
                return Ok(Some(token_launch));
            }
        }

        debug!("❌ Could not extract mint from PumpFun create instruction");
        Ok(None)
    }

    /// Extract metadata from instruction data
    fn extract_metadata_from_instruction(&self, data: &[u8]) -> LaunchMetadata {
        // PumpFun create instruction format (after discriminator):
        // name: string, symbol: string, uri: string, creator: pubkey

        if data.len() < 8 {
            return LaunchMetadata {
                name: None,
                symbol: None,
                uri: None,
                initial_supply: None,
                mint_authority: None,
            };
        }

        // Skip discriminator (8 bytes)
        let mut cursor = 8;

        // Try to extract name (first string)
        if let Some((name, new_cursor)) = self.extract_string_from_data(data, cursor) {
            cursor = new_cursor;

            // Try to extract symbol (second string)
            if let Some((symbol, _)) = self.extract_string_from_data(data, cursor) {
                return LaunchMetadata {
                    name: Some(name),
                    symbol: Some(symbol),
                    uri: None,
                    initial_supply: None,
                    mint_authority: None,
                };
            }
        }

        LaunchMetadata {
            name: None,
            symbol: None,
            uri: None,
            initial_supply: None,
            mint_authority: None,
        }
    }

    /// Extract string from instruction data
    fn extract_string_from_data(&self, data: &[u8], start: usize) -> Option<(String, usize)> {
        if start + 4 > data.len() {
            return None;
        }

        // Read string length (4 bytes, little endian)
        let len = u32::from_le_bytes([
            data[start],
            data[start + 1],
            data[start + 2],
            data[start + 3],
        ]) as usize;

        let str_start = start + 4;
        let str_end = str_start + len;

        if str_end > data.len() {
            return None;
        }

        if let Ok(string) = String::from_utf8(data[str_start..str_end].to_vec()) {
            Some((string, str_end))
        } else {
            None
        }
    }
}
