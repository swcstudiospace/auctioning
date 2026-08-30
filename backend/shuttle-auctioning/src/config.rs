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
    /// MagicBlock ER websocket endpoint. Empty is fine for local/dev.
    pub er_ws: String,
    /// Base58 secret of the backend race/settle authority. Production:
    /// load from Vault/KMS instead — this is a dev convenience only.
    pub authority_secret_b58: Option<String>,
    /// Max race duration before forced settle (seconds).
    pub max_race_secs: u64,
    /// Shared secret required on gameplay/narrative ingest endpoints so the
    /// public cannot mint free RP directly (`X-Auctioning-Ingest` header).
    pub ingest_secret: Option<String>,
    /// Optional narrative polish endpoint. Empty = templates only.
    pub narrative_llm_url: Option<String>,
    pub narrative_llm_key: Option<String>,
    pub supergrok_authorize_url: Option<String>,
    pub supergrok_token_url: Option<String>,
    pub supergrok_client_id: Option<String>,
    pub supergrok_client_secret: Option<String>,
    pub supergrok_redirect_uri: Option<String>,
    pub supergrok_completion_url: Option<String>,
}

impl AppConfig {
    fn from_get(get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            whop_api_key: get("WHOP_API_KEY"),
            whop_webhook_secret: get("WHOP_WEBHOOK_SECRET"),
            weekly_free_rp: get("WEEKLY_FREE_RP")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            program_id: get("PROGRAM_ID")
                .unwrap_or_else(|| "3GGYRVymmKQhmxP9nw9yPs8HCf7YWw7WViPjkKFkZNGs".to_string()),
            mainnet_rpc: get("MAINNET_RPC")
                .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".into()),
            er_rpc: get("ER_RPC").unwrap_or_else(|| "https://devnet-er.magicblock.app".into()),
            er_ws: get("ER_WS").unwrap_or_default(),
            authority_secret_b58: get("AUTHORITY_SECRET"),
            max_race_secs: get("MAX_RACE_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            ingest_secret: get("INGEST_SECRET"),
            narrative_llm_url: get("NARRATIVE_LLM_URL"),
            narrative_llm_key: get("NARRATIVE_LLM_KEY"),
            supergrok_authorize_url: get("SUPERGROK_AUTHORIZE_URL"),
            supergrok_token_url: get("SUPERGROK_TOKEN_URL"),
            supergrok_client_id: get("SUPERGROK_CLIENT_ID"),
            supergrok_client_secret: get("SUPERGROK_CLIENT_SECRET"),
            supergrok_redirect_uri: get("SUPERGROK_REDIRECT_URI"),
            supergrok_completion_url: get("SUPERGROK_COMPLETION_URL"),
        }
    }

    pub fn from_secret_store(store: &SecretStore) -> Self {
        Self::from_get(|k| store.get(k))
    }

    pub fn from_env() -> Self {
        Self::from_get(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
    }

    /// Live polish is opt-in. Templates always run when this is false.
    pub fn narrative_llm_enabled(&self) -> bool {
        self.narrative_llm_url
            .as_deref()
            .is_some_and(|u| !u.is_empty())
            && self
                .narrative_llm_key
                .as_deref()
                .is_some_and(|k| !k.is_empty())
    }

    pub fn supergrok(&self) -> crate::oauth_llm::SuperGrokOauthConfig {
        crate::oauth_llm::SuperGrokOauthConfig::from_parts(
            self.supergrok_authorize_url.clone(),
            self.supergrok_token_url.clone(),
            self.supergrok_client_id.clone(),
            self.supergrok_client_secret.clone(),
            self.supergrok_redirect_uri.clone(),
            self.supergrok_completion_url.clone(),
        )
    }
}
