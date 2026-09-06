//! Supabase Auth principals.
//!
//! The marketing site signs people in with Supabase (email OTP / OAuth) and
//! sends the Supabase access token as the bearer. The API verifies it by
//! introspection — `GET {SUPABASE_URL}/auth/v1/user` with the project's
//! publishable/anon key — so no signing-key handling or JWT crypto lives
//! here and key rotation on Supabase's side is invisible to us.
//!
//! A Supabase user acts through a deterministic synthetic wallet
//! (`sb` + base58(user uuid)), so every wallet-keyed ledger (weekly RP,
//! support allocations, sessions) works unchanged. Wallet sign-in
//! (`auth.rs`) is untouched; both principals resolve to an [`AuthedWallet`].
//!
//! Results are cached for a short TTL keyed by the token's SHA-256 so a page
//! that fires several authed calls does not hit Supabase for each.

use crate::error::{AppError, AppResult};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Cache window for a verified token. Short on purpose: sign-out on the
/// Supabase side takes effect within this bound.
const CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_TOKEN_LEN: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
}

/// Cheap shape check so wallet-session bearers never round-trip to Supabase.
pub fn looks_like_jwt(token: &str) -> bool {
    token.len() >= 64
        && token.len() <= MAX_TOKEN_LEN
        && token.starts_with("eyJ")
        && token.matches('.').count() == 2
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// Deterministic ledger wallet for a Supabase user: base58 of the uuid bytes
/// with an `sb` prefix (both letters are in the base58 alphabet). 24 chars,
/// so it passes `ledger::valid_wallet` and can never collide with a real
/// 32-byte Solana key (those encode to 43–44 chars).
pub fn wallet_for_user(user_id: &Uuid) -> String {
    format!("sb{}", bs58::encode(user_id.as_bytes()).into_string())
}

#[derive(Debug, Clone)]
pub struct SupabasePrincipal {
    pub user_id: Uuid,
    pub wallet: String,
    pub email: Option<String>,
    pub provider: Option<String>,
}

struct Cached {
    principal: SupabasePrincipal,
    until: Instant,
}

fn cache() -> &'static Mutex<HashMap<String, Cached>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Cached>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("auctioning-api/supabase-auth")
            .build()
            .expect("reqwest client")
    })
}

fn token_key(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(Deserialize)]
struct SupabaseUser {
    id: Uuid,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    app_metadata: Option<AppMetadata>,
}

#[derive(Deserialize)]
struct AppMetadata {
    #[serde(default)]
    provider: Option<String>,
}

/// Verify a Supabase access token and return the principal it maps to.
/// `Ok(None)` means Supabase rejected the token (401/403); transport or
/// upstream failures surface as 503 so a Supabase outage never looks like a
/// bad password.
pub async fn resolve(
    db: &sqlx::PgPool,
    cfg: &SupabaseConfig,
    token: &str,
) -> AppResult<Option<SupabasePrincipal>> {
    if !looks_like_jwt(token) {
        return Ok(None);
    }
    let key = token_key(token);
    if let Some(hit) = cache().lock().ok().and_then(|c| {
        c.get(&key)
            .filter(|h| h.until > Instant::now())
            .map(|h| h.principal.clone())
    }) {
        return Ok(Some(hit));
    }

    let url = format!("{}/auth/v1/user", cfg.url.trim_end_matches('/'));
    let resp = http()
        .get(&url)
        .header("apikey", &cfg.anon_key)
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "supabase auth unreachable");
            AppError::Unavailable("supabase auth unreachable".into())
        })?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Ok(None);
    }
    if !status.is_success() {
        tracing::warn!(status = %status, "supabase auth unexpected status");
        return Err(AppError::Unavailable("supabase auth error".into()));
    }
    let user: SupabaseUser = resp
        .json()
        .await
        .map_err(|_| AppError::Unavailable("supabase auth malformed response".into()))?;

    let wallet = wallet_for_user(&user.id);
    let provider = user.app_metadata.and_then(|m| m.provider);
    let email = user
        .email
        .filter(|e| !e.is_empty())
        .map(|e| e.chars().take(320).collect::<String>());

    let mut tx = db.begin().await?;
    crate::ledger::ensure_wallet_tx(&mut tx, &wallet).await?;
    sqlx::query(
        r#"
        INSERT INTO supabase_identities (user_id, wallet, email, provider)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id) DO UPDATE SET
            email = COALESCE(EXCLUDED.email, supabase_identities.email),
            provider = COALESCE(EXCLUDED.provider, supabase_identities.provider),
            last_seen_at = now()
        "#,
    )
    .bind(user.id)
    .bind(&wallet)
    .bind(&email)
    .bind(&provider)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let principal = SupabasePrincipal {
        user_id: user.id,
        wallet,
        email,
        provider,
    };
    if let Ok(mut c) = cache().lock() {
        if c.len() > 10_000 {
            c.retain(|_, v| v.until > Instant::now());
        }
        c.insert(
            key,
            Cached {
                principal: principal.clone(),
                until: Instant::now() + CACHE_TTL,
            },
        );
    }
    Ok(Some(principal))
}

/// Drop a token from the cache (client sign-out).
pub fn forget(token: &str) {
    if let Ok(mut c) = cache().lock() {
        c.remove(&token_key(token));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_shape_check() {
        let fake = format!(
            "eyJ{}.{}.{}",
            "a".repeat(40),
            "b".repeat(40),
            "c".repeat(40)
        );
        assert!(looks_like_jwt(&fake));
        assert!(!looks_like_jwt("randomsessiontoken"));
        assert!(!looks_like_jwt("eyJ.short.x"));
        assert!(!looks_like_jwt(&format!(
            "eyJ{}.{}",
            "a".repeat(40),
            "b".repeat(40)
        )));
    }

    #[test]
    fn wallet_is_deterministic_and_valid() {
        let id = Uuid::parse_str("4bafa87f-0e22-4557-b761-a355473f0d3e").unwrap();
        let w = wallet_for_user(&id);
        assert_eq!(w, wallet_for_user(&id));
        assert!(w.starts_with("sb"));
        assert!(w.len() <= 24, "{w}");
        assert!(crate::ledger::valid_wallet(&w));
    }
}
