//! Authentication and authorization for the HTTP edge.
//!
//! Three principals, three extractors:
//!
//! * [`AuthedWallet`] — a Solana wallet that proved control by signing a server
//!   nonce (Sign-In-With-Solana style). Backed by `auth_sessions`; the bearer
//!   token is random and only its SHA-256 is stored.
//! * [`Operator`] — a human with the `OPERATOR_TOKEN`. Gates snapshots,
//!   narrative approval, OAuth login, and BI-sensitive reads.
//! * [`Ingest`] — a machine with the `INGEST_SECRET`. Gates free-RP minting,
//!   catalog import, ER tick ingest, and event cards.
//!
//! In `APP_ENV=dev` the operator and ingest gates are open when their secret is
//! unset (mirrors the previous behaviour) and `AUTH_DEV_BYPASS=true` lets a
//! request impersonate a wallet with `X-Auctioning-Dev-Wallet`. In `prod`
//! [`crate::config::AppConfig::validate`] refuses to boot without the secrets.

use crate::error::{AppError, AppResult};
use crate::ledger;
use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use sqlx::PgPool;
use std::str::FromStr;

pub const NONCE_TTL: Duration = Duration::minutes(10);
pub const SESSION_TTL: Duration = Duration::days(7);
const BEARER: &str = "bearer ";

/// Constant-time byte comparison. Length leaks; content does not.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Human-readable challenge the wallet signs. Stable format: clients may show
/// it verbatim in the Phantom prompt.
pub fn build_message(domain: &str, wallet: &str, nonce: &str, issued_at: DateTime<Utc>) -> String {
    format!(
        "{domain} wants you to sign in with your Solana account:\n{wallet}\n\n\
         Signing proves you control this wallet. No transaction is sent and no \
         fee is charged.\n\nNonce: {nonce}\nIssued At: {}",
        issued_at.to_rfc3339_opts(SecondsFormat::Secs, true)
    )
}

/// ed25519 verification of `message` against `wallet` with a base58 signature.
pub fn verify_signature(wallet: &str, message: &str, signature_b58: &str) -> bool {
    let Ok(pubkey) = Pubkey::from_str(wallet) else {
        return false;
    };
    let Ok(sig) = Signature::from_str(signature_b58) else {
        return false;
    };
    sig.verify(pubkey.as_ref(), message.as_bytes())
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn random_nonce() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get("authorization")?.to_str().ok()?;
    if raw.len() > BEARER.len() && raw[..BEARER.len()].eq_ignore_ascii_case(BEARER) {
        Some(raw[BEARER.len()..].trim())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Nonce + session persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct NonceChallenge {
    pub wallet: String,
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn issue_nonce(
    db: &PgPool,
    domain: &str,
    wallet: &str,
) -> Result<NonceChallenge, sqlx::Error> {
    let nonce = random_nonce();
    let now = Utc::now();
    let expires_at = now + NONCE_TTL;
    sqlx::query(
        "INSERT INTO auth_nonces (nonce, wallet, created_at, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&nonce)
    .bind(wallet)
    .bind(now)
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok(NonceChallenge {
        wallet: wallet.to_string(),
        message: build_message(domain, wallet, &nonce, now),
        nonce,
        expires_at,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionGrant {
    pub token: String,
    pub wallet: String,
    pub expires_at: DateTime<Utc>,
}

/// Consume a nonce and mint a session. Returns `None` when the nonce is
/// unknown, expired, already used, or the signature does not verify.
pub async fn verify_and_create_session(
    db: &PgPool,
    domain: &str,
    wallet: &str,
    nonce: &str,
    signature_b58: &str,
    user_agent: Option<&str>,
) -> Result<Option<SessionGrant>, sqlx::Error> {
    let mut tx = db.begin().await?;
    let row: Option<(DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT created_at, expires_at FROM auth_nonces
        WHERE nonce = $1 AND wallet = $2 AND consumed_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(nonce)
    .bind(wallet)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((issued_at, expires_at)) = row else {
        return Ok(None);
    };
    let now = Utc::now();
    if now > expires_at {
        return Ok(None);
    }
    let message = build_message(domain, wallet, nonce, issued_at);
    if !verify_signature(wallet, &message, signature_b58) {
        return Ok(None);
    }
    sqlx::query("UPDATE auth_nonces SET consumed_at = $2 WHERE nonce = $1")
        .bind(nonce)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    ledger::ensure_wallet_tx(&mut tx, wallet).await?;

    let token = random_token();
    let session_expires = now + SESSION_TTL;
    sqlx::query(
        r#"
        INSERT INTO auth_sessions (token_hash, wallet, created_at, expires_at, last_seen_at, user_agent)
        VALUES ($1, $2, $3, $4, $3, $5)
        "#,
    )
    .bind(hash_token(&token))
    .bind(wallet)
    .bind(now)
    .bind(session_expires)
    .bind(user_agent.map(|u| u.chars().take(256).collect::<String>()))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(SessionGrant {
        token,
        wallet: wallet.to_string(),
        expires_at: session_expires,
    }))
}

pub async fn wallet_for_token(db: &PgPool, token: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        UPDATE auth_sessions SET last_seen_at = now()
        WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()
        RETURNING wallet
        "#,
    )
    .bind(hash_token(token))
    .fetch_optional(db)
    .await?;
    Ok(row.map(|(w,)| w))
}

pub async fn revoke_token(db: &PgPool, token: &str) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE auth_sessions SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(hash_token(token))
    .execute(db)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Housekeeping: drop expired nonces and sessions. Safe to run often.
pub async fn prune_expired(db: &PgPool) -> Result<u64, sqlx::Error> {
    let a = sqlx::query("DELETE FROM auth_nonces WHERE expires_at < now() - interval '1 hour'")
        .execute(db)
        .await?
        .rows_affected();
    let b = sqlx::query("DELETE FROM auth_sessions WHERE expires_at < now() - interval '7 days'")
        .execute(db)
        .await?
        .rows_affected();
    Ok(a + b)
}

// ---------------------------------------------------------------------------
// Extractors
// ---------------------------------------------------------------------------

/// A wallet with a live session. Use instead of trusting `wallet` in a body.
#[derive(Debug, Clone)]
pub struct AuthedWallet(pub String);

impl AuthedWallet {
    /// Body-supplied wallet (legacy clients) must match the session wallet.
    pub fn require_match(&self, claimed: Option<&str>) -> AppResult<()> {
        match claimed {
            Some(w) if w != self.0 => Err(AppError::Forbidden(
                "body wallet does not match the authenticated session".into(),
            )),
            _ => Ok(()),
        }
    }
}

impl FromRequestParts<crate::AppState> for AuthedWallet {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        if state.cfg.auth_dev_bypass {
            if let Some(w) = parts
                .headers
                .get("x-auctioning-dev-wallet")
                .and_then(|v| v.to_str().ok())
                .filter(|w| ledger::valid_wallet(w))
            {
                ledger::ensure_wallet(&state.db, w).await?;
                return Ok(AuthedWallet(w.to_string()));
            }
        }
        let Some(token) = bearer(&parts.headers) else {
            return Err(AppError::Unauthorized);
        };
        if token.is_empty() || token.len() > 128 {
            return Err(AppError::Unauthorized);
        }
        match wallet_for_token(&state.db, token).await? {
            Some(wallet) => Ok(AuthedWallet(wallet)),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// Operator principal (`X-Auctioning-Operator: <token>` or bearer).
#[derive(Debug, Clone, Copy)]
pub struct Operator;

impl FromRequestParts<crate::AppState> for Operator {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        check_operator(state, &parts.headers).map(|_| Operator)
    }
}

pub fn check_operator(state: &crate::AppState, headers: &HeaderMap) -> AppResult<()> {
    match state.cfg.operator_token.as_deref() {
        None if state.cfg.is_dev() => Ok(()),
        None => Err(AppError::Unauthorized),
        Some(secret) => {
            let provided = headers
                .get("x-auctioning-operator")
                .and_then(|v| v.to_str().ok())
                .or_else(|| bearer(headers))
                .unwrap_or_default();
            if constant_time_eq(provided.as_bytes(), secret.as_bytes()) {
                Ok(())
            } else {
                Err(AppError::Unauthorized)
            }
        }
    }
}

/// Machine principal for seeding, ticks, and free-RP minting.
#[derive(Debug, Clone, Copy)]
pub struct Ingest;

impl FromRequestParts<crate::AppState> for Ingest {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        check_ingest(state, &parts.headers).map(|_| Ingest)
    }
}

pub fn check_ingest(state: &crate::AppState, headers: &HeaderMap) -> AppResult<()> {
    match state.cfg.ingest_secret.as_deref() {
        None if state.cfg.is_dev() => Ok(()),
        None => Err(AppError::Unauthorized),
        Some(secret) => {
            let provided = headers
                .get("x-auctioning-ingest")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            if constant_time_eq(provided.as_bytes(), secret.as_bytes()) {
                Ok(())
            } else {
                Err(AppError::Unauthorized)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct NonceQuery {
    pub wallet: String,
}

pub async fn nonce_handler(
    State(state): State<crate::AppState>,
    Query(q): Query<NonceQuery>,
) -> AppResult<Json<NonceChallenge>> {
    if !ledger::valid_wallet(&q.wallet) || Pubkey::from_str(&q.wallet).is_err() {
        return Err(AppError::BadRequest(
            "wallet is not a valid Solana pubkey".into(),
        ));
    }
    let challenge = issue_nonce(&state.db, &state.cfg.app_domain, &q.wallet).await?;
    Ok(Json(challenge))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub wallet: String,
    pub nonce: String,
    /// Base58 ed25519 signature of the `message` returned by the nonce call.
    pub signature: String,
}

pub async fn verify_handler(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    Json(req): Json<VerifyRequest>,
) -> AppResult<Json<SessionGrant>> {
    if !ledger::valid_wallet(&req.wallet) {
        return Err(AppError::BadRequest("wallet invalid".into()));
    }
    if req.nonce.len() > 64 || req.signature.len() > 128 {
        return Err(AppError::BadRequest("nonce or signature too long".into()));
    }
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    match verify_and_create_session(
        &state.db,
        &state.cfg.app_domain,
        &req.wallet,
        &req.nonce,
        &req.signature,
        ua,
    )
    .await?
    {
        Some(grant) => Ok(Json(grant)),
        None => Err(AppError::Unauthorized),
    }
}

pub async fn me_handler(
    State(state): State<crate::AppState>,
    AuthedWallet(wallet): AuthedWallet,
) -> AppResult<Json<serde_json::Value>> {
    let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT MAX(expires_at) FROM auth_sessions WHERE wallet = $1 AND revoked_at IS NULL",
    )
    .bind(&wallet)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(json!({
        "wallet": wallet,
        "expires_at": row.map(|(t,)| t),
    })))
}

pub async fn logout_handler(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    AuthedWallet(wallet): AuthedWallet,
) -> AppResult<Json<serde_json::Value>> {
    let revoked = match bearer(&headers) {
        Some(token) => revoke_token(&state.db, token).await?,
        None => false,
    };
    Ok(Json(json!({ "wallet": wallet, "revoked": revoked })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn message_is_stable_and_names_wallet_and_nonce() {
        let at = DateTime::parse_from_rfc3339("2026-09-04T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let m = build_message("auctioning.lol", "wallet1", "nonce1", at);
        assert!(m.starts_with("auctioning.lol wants you to sign in"));
        assert!(m.contains("\nwallet1\n"));
        assert!(m.contains("Nonce: nonce1"));
        assert!(m.contains("Issued At: 2026-09-04T00:00:00Z"));
        assert_eq!(m, build_message("auctioning.lol", "wallet1", "nonce1", at));
    }

    #[test]
    fn real_keypair_signature_verifies_and_tampering_fails() {
        let kp = Keypair::new();
        let wallet = kp.pubkey().to_string();
        let msg = build_message("auctioning.lol", &wallet, "n", Utc::now());
        let sig = kp.sign_message(msg.as_bytes()).to_string();
        assert!(verify_signature(&wallet, &msg, &sig));
        assert!(!verify_signature(&wallet, &format!("{msg}x"), &sig));
        let other = Keypair::new().pubkey().to_string();
        assert!(!verify_signature(&other, &msg, &sig));
        assert!(!verify_signature(&wallet, &msg, "not-base58-!!"));
        assert!(!verify_signature("junk", &msg, &sig));
    }

    #[test]
    fn tokens_are_random_urlsafe_and_hash_is_hex() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert!(a
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_eq!(hash_token(&a).len(), 64);
        assert_eq!(hash_token(&a), hash_token(&a));
    }

    #[test]
    fn bearer_header_parsing() {
        let mut h = HeaderMap::new();
        assert!(bearer(&h).is_none());
        h.insert("authorization", "Bearer abc".parse().unwrap());
        assert_eq!(bearer(&h), Some("abc"));
        h.insert("authorization", "bearer   xyz ".parse().unwrap());
        assert_eq!(bearer(&h), Some("xyz"));
        h.insert("authorization", "Basic abc".parse().unwrap());
        assert!(bearer(&h).is_none());
    }

    #[test]
    fn require_match_rejects_other_wallet() {
        let a = AuthedWallet("A".into());
        assert!(a.require_match(None).is_ok());
        assert!(a.require_match(Some("A")).is_ok());
        assert!(a.require_match(Some("B")).is_err());
    }
}
