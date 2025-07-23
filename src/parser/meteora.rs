use crate::geyser::QueuedTransaction;
use crate::parser::{
    LaunchpadParser, LaunchpadType, ParseResult, TokenLaunch, launchpad_parser::LaunchMetadata,
};
use tracing::{debug, info};

pub struct MeteoraParser {
    program_ids: Vec<String>,
}

impl MeteoraParser {
    pub fn new() -> Self {
        Self {
            // Only MeteoraDBC program
            program_ids: vec![
                // Meteora DBC program
                "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN".to_string(),
            ],
        }
    }
}

impl LaunchpadParser for MeteoraParser {
    fn get_program_ids(&self) -> Vec<String> {
        self.program_ids.clone()
    }

    fn get_launchpad_type(&self) -> LaunchpadType {
        LaunchpadType::Meteora
    }

    fn parse_transaction(
        &self,
        transaction: &QueuedTransaction,
    ) -> Result<Vec<ParseResult>, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "🔍 Parsing MeteoraDBC transaction: {}",
            transaction.signature
        );

        for instr in &transaction.instructions {
            if self.program_ids.contains(&instr.program_id) && instr.data.len() >= 8 {
                let discriminator = &instr.data[0..8];

                // MeteoraDBC instructions (from meteoraDBC.json)
                if discriminator == [140, 85, 215, 176, 102, 54, 104, 79] {
                    info!(
                        "🎯 Found MeteoraDBC initialize_virtual_pool_with_spl_token in: {}",
                        transaction.signature
                    );

                    if let Some(token_launch) =
                        self.extract_token_launch_meteora_dbc(transaction, instr)?
                    {
                        return Ok(vec![ParseResult::TokenLaunch(token_launch)]);
                    }
                } else if discriminator == [169, 118, 51, 78, 145, 110, 220, 155] {
                    info!(
                        "🎯 Found MeteoraDBC initialize_virtual_pool_with_token2022 in: {}",
                        transaction.signature
                    );

                    if let Some(token_launch) =
                        self.extract_token_launch_meteora_dbc(transaction, instr)?
                    {
                        return Ok(vec![ParseResult::TokenLaunch(token_launch)]);
                    }
                }
            }
        }

        Ok(vec![ParseResult::NotRelevant])
    }
}

impl MeteoraParser {
    /// Extract token launch information from MeteoraDBC initialize instruction
    fn extract_token_launch_meteora_dbc(
        &self,
        transaction: &QueuedTransaction,
        instruction: &crate::geyser::TransactionInstruction,
    ) -> Result<Option<TokenLaunch>, Box<dyn std::error::Error + Send + Sync>> {
        // Try to find the base_mint from instruction accounts
        // According to MeteoraDBC IDL, account index 3 should be base_mint (newly created token)
        if let Some(mint_idx) = instruction.accounts.get(3) {
            if let Some(mint_address) = transaction.accounts.get(*mint_idx as usize) {
                // Creator should be account index 2
                let creator = instruction
                    .accounts
                    .get(2)
                    .and_then(|idx| transaction.accounts.get(*idx as usize))
                    .cloned();

                let token_launch = TokenLaunch {
                    launchpad: LaunchpadType::Meteora,
                    token_address: mint_address.clone(),
                    creator,
                    signature: transaction.signature.clone(),
                    slot: transaction.slot,
                    timestamp: transaction.received_time,
                    metadata: self.extract_metadata_from_meteora_dbc_instruction(&instruction.data),
                };

                debug!("✅ Extracted MeteoraDBC token launch: {}", mint_address);
                return Ok(Some(token_launch));
            }
        }

        debug!("❌ Could not extract mint from MeteoraDBC initialize instruction");
        Ok(None)
    }

    /// Extract metadata from MeteoraDBC instruction data
    fn extract_metadata_from_meteora_dbc_instruction(&self, data: &[u8]) -> LaunchMetadata {
        // MeteoraDBC initialize instruction format (after discriminator):
        // params: InitializePoolParameters { name: string, symbol: string, uri: string }

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

        // Try to extract name (first string in params)
        if let Some((name, new_cursor)) = self.extract_string_from_data(data, cursor) {
            cursor = new_cursor;

            // Try to extract symbol (second string in params)
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
