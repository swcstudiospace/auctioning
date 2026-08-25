//! Runtime configuration. All secrets come from Shuttle SecretStore
//! (`Secrets.toml` locally / shuttle secrets in prod). No secrets in code.

use shuttle_runtime::SecretStore;

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Whop API key (server-side) for membership verification.
    pub whop_api_key: Option<String>,
    /// Whop webhook signing secret for HMAC verification.
    pub whop_webhook_secret: Option<String>,
    /// Free weekly RP amount (promotional, non-cashable, off-chain only).
    pub weekly_free_rp: i64,
    /// Solana program id (base58) of the deployed Anchor program.
    pub program_id: String,
    /// Mainnet RPC for on-chain reads/confirmation.
    pub mainnet_rpc: String,
    /// MagicBlock ER RPC endpoint (ephemeral chain).
    pub er_rpc: String,
    /// Base58 secret of the backend race/settle authority. Production:
    /// load from Vault/KMS instead — this is a dev convenience only.
    pub authority_secret_b58: Option<String>,
    /// Max race duration before forced settle (seconds).
    pub max_race_secs: u64,
    /// Shared secret required on gameplay/narrative ingest endpoints so the
    /// public cannot mint free RP directly (`X-Auctioning-Ingest` header).
    pub ingest_secret: Option<String>,
}

impl AppConfig {
    pub fn from_secret_store(store: &SecretStore) -> Self {
        Self {
            whop_api_key: store.get("WHOP_API_KEY"),
            whop_webhook_secret: store.get("WHOP_WEBHOOK_SECRET"),
            weekly_free_rp: store
                .get("WEEKLY_FREE_RP")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            program_id: store
                .get("PROGRAM_ID")
                .unwrap_or_else(|| "AuCT1oN1Ng111111111111111111111111111111111".to_string()),
            mainnet_rpc: store
                .get("MAINNET_RPC")
                .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".into()),
            er_rpc: store
                .get("ER_RPC")
                .unwrap_or_else(|| "https://devnet-er.magicblock.app".into()),
            authority_secret_b58: store.get("AUTHORITY_SECRET"),
            max_race_secs: store
                .get("MAX_RACE_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            ingest_secret: store.get("INGEST_SECRET"),
        }
    }
}
