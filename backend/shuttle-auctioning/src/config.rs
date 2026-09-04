//! Runtime configuration. All secrets come from Shuttle SecretStore
//! (`Secrets.toml` locally / shuttle secrets in prod) or the process
//! environment for the api-runner. No secrets in code.

use shuttle_runtime::SecretStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Staging,
    Prod,
}

impl AppEnv {
    fn parse(s: Option<String>) -> Self {
        match s.as_deref().map(str::to_ascii_lowercase).as_deref() {
            Some("prod") | Some("production") => AppEnv::Prod,
            Some("staging") | Some("stage") => AppEnv::Staging,
            _ => AppEnv::Dev,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AppEnv::Dev => "dev",
            AppEnv::Staging => "staging",
            AppEnv::Prod => "prod",
        }
    }
}

/// How Whop encodes `amount` in webhook payloads. Whop's REST/v2 payloads use
/// decimal currency units (19.9 == $19.90); some integrations forward cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyUnit {
    Dollars,
    Cents,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub env: AppEnv,
    /// Domain shown in the sign-in message and used for CORS defaults.
    pub app_domain: String,
    /// Exact origins allowed by CORS. Empty in dev = permissive.
    pub allowed_origins: Vec<String>,
    /// Whop API key (server-side) for membership verification.
    pub whop_api_key: Option<String>,
    /// Whop webhook signing secret for HMAC verification.
    pub whop_webhook_secret: Option<String>,
    pub whop_amount_unit: MoneyUnit,
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
    /// Shared secret for machine ingest (`X-Auctioning-Ingest`).
    pub ingest_secret: Option<String>,
    /// Operator token for human/admin actions (`X-Auctioning-Operator`).
    pub operator_token: Option<String>,
    /// Dev only: honour `X-Auctioning-Dev-Wallet` instead of a session.
    pub auth_dev_bypass: bool,
    /// Per-IP request budget per minute for public write endpoints.
    pub rate_limit_per_min: u32,
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

fn non_empty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.trim().is_empty())
}

impl AppConfig {
    fn from_get(get: impl Fn(&str) -> Option<String>) -> Self {
        let env = AppEnv::parse(get("APP_ENV"));
        Self {
            env,
            app_domain: non_empty(get("APP_DOMAIN")).unwrap_or_else(|| "auctioning.lol".into()),
            allowed_origins: get("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            whop_api_key: non_empty(get("WHOP_API_KEY")),
            whop_webhook_secret: non_empty(get("WHOP_WEBHOOK_SECRET")),
            whop_amount_unit: match get("WHOP_AMOUNT_UNIT").as_deref() {
                Some("cents") => MoneyUnit::Cents,
                _ => MoneyUnit::Dollars,
            },
            weekly_free_rp: get("WEEKLY_FREE_RP")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            program_id: non_empty(get("PROGRAM_ID"))
                .unwrap_or_else(|| "3GGYRVymmKQhmxP9nw9yPs8HCf7YWw7WViPjkKFkZNGs".to_string()),
            mainnet_rpc: non_empty(get("MAINNET_RPC"))
                .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".into()),
            er_rpc: non_empty(get("ER_RPC"))
                .unwrap_or_else(|| "https://devnet-er.magicblock.app".into()),
            er_ws: get("ER_WS").unwrap_or_default(),
            authority_secret_b58: non_empty(get("AUTHORITY_SECRET")),
            max_race_secs: get("MAX_RACE_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            ingest_secret: non_empty(get("INGEST_SECRET")),
            operator_token: non_empty(get("OPERATOR_TOKEN")),
            auth_dev_bypass: env == AppEnv::Dev
                && get("AUTH_DEV_BYPASS")
                    .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                    .unwrap_or(false),
            rate_limit_per_min: get("RATE_LIMIT_PER_MIN")
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            narrative_llm_url: non_empty(get("NARRATIVE_LLM_URL")),
            narrative_llm_key: non_empty(get("NARRATIVE_LLM_KEY")),
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

    pub fn is_dev(&self) -> bool {
        self.env == AppEnv::Dev
    }

    /// Refuse to serve a production build with open gates. Returns every
    /// problem at once so an operator fixes the deploy in one pass.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();
        if self.env != AppEnv::Dev {
            if self.ingest_secret.is_none() {
                problems.push("INGEST_SECRET must be set outside dev".into());
            }
            if self.operator_token.is_none() {
                problems.push("OPERATOR_TOKEN must be set outside dev".into());
            }
            if self.whop_webhook_secret.is_none() {
                problems.push("WHOP_WEBHOOK_SECRET must be set outside dev".into());
            }
            if self.allowed_origins.is_empty() {
                problems.push("ALLOWED_ORIGINS must list the site origins outside dev".into());
            }
            if self.auth_dev_bypass {
                problems.push("AUTH_DEV_BYPASS is only honoured in dev".into());
            }
        }
        if let Some(s) = &self.ingest_secret {
            if s.len() < 16 {
                problems.push("INGEST_SECRET is shorter than 16 chars".into());
            }
        }
        if let Some(s) = &self.operator_token {
            if s.len() < 16 {
                problems.push("OPERATOR_TOKEN is shorter than 16 chars".into());
            }
        }
        if self.weekly_free_rp < 0 || self.weekly_free_rp > 10_000 {
            problems.push("WEEKLY_FREE_RP out of range (0..=10000)".into());
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(pairs: &[(&str, &str)]) -> AppConfig {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        AppConfig::from_get(|k| map.get(k).cloned())
    }

    #[test]
    fn dev_defaults_are_open_and_valid() {
        let c = cfg(&[]);
        assert_eq!(c.env, AppEnv::Dev);
        assert!(c.validate().is_ok());
        assert!(c.allowed_origins.is_empty());
        assert_eq!(c.whop_amount_unit, MoneyUnit::Dollars);
        assert!(!c.auth_dev_bypass);
    }

    #[test]
    fn prod_without_secrets_fails_with_every_problem() {
        let c = cfg(&[("APP_ENV", "prod")]);
        let errs = c.validate().unwrap_err();
        assert_eq!(errs.len(), 4, "{errs:?}");
    }

    #[test]
    fn prod_with_secrets_and_origins_is_valid() {
        let c = cfg(&[
            ("APP_ENV", "production"),
            ("INGEST_SECRET", "0123456789abcdef0123"),
            ("OPERATOR_TOKEN", "0123456789abcdef0123"),
            ("WHOP_WEBHOOK_SECRET", "whsec"),
            (
                "ALLOWED_ORIGINS",
                "https://auctioning.lol/, https://www.auctioning.lol",
            ),
        ]);
        assert_eq!(c.env, AppEnv::Prod);
        assert!(c.validate().is_ok());
        assert_eq!(
            c.allowed_origins,
            vec!["https://auctioning.lol", "https://www.auctioning.lol"]
        );
    }

    #[test]
    fn short_secrets_are_rejected_even_in_dev() {
        let c = cfg(&[("INGEST_SECRET", "short")]);
        assert!(c.validate().is_err());
    }

    #[test]
    fn dev_bypass_only_in_dev() {
        assert!(cfg(&[("AUTH_DEV_BYPASS", "true")]).auth_dev_bypass);
        assert!(!cfg(&[("APP_ENV", "staging"), ("AUTH_DEV_BYPASS", "true")]).auth_dev_bypass);
    }

    #[test]
    fn whop_unit_parses() {
        assert_eq!(
            cfg(&[("WHOP_AMOUNT_UNIT", "cents")]).whop_amount_unit,
            MoneyUnit::Cents
        );
        assert_eq!(
            cfg(&[("WHOP_AMOUNT_UNIT", "dollars")]).whop_amount_unit,
            MoneyUnit::Dollars
        );
    }
}
