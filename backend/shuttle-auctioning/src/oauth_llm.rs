//! SuperGrok Heavy OAuth — sole LLM provider login (operator/backend).
//!
//! Authorize/token URLs default to the on-disk grok.ego.engineer connector
//! (`/authorize`, `/token`, client id `grok`). Access tokens are stored in
//! `oauth_tokens` and never returned on tape APIs or written to logs.
//! Logged-out / refresh-fail / unusable completion → templates.

use crate::error::{AppError, AppResult};
use crate::narrative::{LlmEnricher, LlmError, NarrativeChannel, NarrativeInput};
use anyhow;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub const PROVIDER: &str = "supergrok_heavy";

#[derive(Debug, Clone)]
pub struct SuperGrokOauthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scope: String,
    /// Chat completions endpoint. Empty = enrichment disabled even if logged in.
    pub completion_url: String,
}

impl SuperGrokOauthConfig {
    pub fn from_parts(
        authorize_url: Option<String>,
        token_url: Option<String>,
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_uri: Option<String>,
        completion_url: Option<String>,
    ) -> Self {
        Self {
            authorize_url: authorize_url
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://grok.ego.engineer/authorize".into()),
            token_url: token_url
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://grok.ego.engineer/token".into()),
            client_id: client_id.filter(|s| !s.is_empty()).unwrap_or_else(|| "grok".into()),
            client_secret: client_secret.unwrap_or_default(),
            redirect_uri: redirect_uri.unwrap_or_default(),
            scope: "mcp".into(),
            completion_url: completion_url.unwrap_or_default(),
        }
    }

    pub fn configured(&self) -> bool {
        !self.client_secret.is_empty() && !self.redirect_uri.is_empty()
    }
}

fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, hash)
}

pub struct AuthStart {
    pub authorize_url: String,
    pub state: String,
}

pub async fn start_login(db: &PgPool, cfg: &SuperGrokOauthConfig) -> Result<AuthStart, sqlx::Error> {
    let state = Uuid::new_v4().to_string();
    let verifier = Uuid::new_v4().simple().to_string();
    sqlx::query("INSERT INTO oauth_states (state, code_verifier) VALUES ($1, $2)")
        .bind(&state)
        .bind(&verifier)
        .execute(db)
        .await?;
    let challenge = pkce_challenge(&verifier);
    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        cfg.authorize_url,
        urlencoding(&cfg.client_id),
        urlencoding(&cfg.redirect_uri),
        urlencoding(&cfg.scope),
        urlencoding(&state),
        challenge,
    );
    Ok(AuthStart {
        authorize_url: url,
        state,
    })
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

pub async fn logged_in(db: &PgPool) -> Result<bool, sqlx::Error> {
    let row: Option<(Option<chrono::DateTime<Utc>>,)> =
        sqlx::query_as("SELECT expires_at FROM oauth_tokens WHERE provider = $1")
            .bind(PROVIDER)
            .fetch_optional(db)
            .await?;
    Ok(row.is_some())
}

/// Persist tokens. Callers must not log `access` / `refresh`.
pub async fn store_tokens(
    db: &PgPool,
    access: &str,
    refresh: Option<&str>,
    expires_in: Option<i64>,
) -> Result<(), sqlx::Error> {
    let expires = expires_in.map(|s| Utc::now() + Duration::seconds(s));
    sqlx::query(
        r#"
        INSERT INTO oauth_tokens (provider, access_token, refresh_token, expires_at, updated_at)
        VALUES ($1, $2, $3, $4, now())
        ON CONFLICT (provider) DO UPDATE
          SET access_token = EXCLUDED.access_token,
              refresh_token = COALESCE(EXCLUDED.refresh_token, oauth_tokens.refresh_token),
              expires_at = EXCLUDED.expires_at,
              updated_at = now()
        "#,
    )
    .bind(PROVIDER)
    .bind(access)
    .bind(refresh)
    .bind(expires)
    .execute(db)
    .await?;
    tracing::info!(provider = PROVIDER, "oauth token stored");
    Ok(())
}

pub async fn load_access_token(db: &PgPool) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT access_token FROM oauth_tokens WHERE provider = $1")
        .bind(PROVIDER)
        .fetch_optional(db)
        .await
}

/// Consume a one-time PKCE state row. Returns the stored `code_verifier`.
pub async fn take_state(db: &PgPool, state: &str) -> Result<Option<String>, sqlx::Error> {
    let verifier: Option<String> =
        sqlx::query_scalar("SELECT code_verifier FROM oauth_states WHERE state = $1")
            .bind(state)
            .fetch_optional(db)
            .await?;
    if verifier.is_some() {
        sqlx::query("DELETE FROM oauth_states WHERE state = $1")
            .bind(state)
            .execute(db)
            .await?;
    }
    Ok(verifier)
}

/// Exchange an authorization code for tokens. Never logs access/refresh tokens.
pub async fn exchange_code(
    db: &PgPool,
    cfg: &SuperGrokOauthConfig,
    state: &str,
    code: &str,
) -> Result<(), anyhow::Error> {
    let verifier = take_state(db, state)
        .await?
        .ok_or_else(|| anyhow::anyhow!("unknown oauth state"))?;
    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&client_secret={}&code_verifier={}",
        urlencoding(code),
        urlencoding(&cfg.redirect_uri),
        urlencoding(&cfg.client_id),
        urlencoding(&cfg.client_secret),
        urlencoding(&verifier),
    );
    let resp = reqwest::Client::new()
        .post(&cfg.token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("token exchange request failed"))?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("token exchange http error"));
    }
    let tokens: TokenResponse = resp
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("token exchange parse error"))?;
    let access = tokens
        .access_token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing access_token"))?;
    store_tokens(db, &access, tokens.refresh_token.as_deref(), tokens.expires_in).await?;
    Ok(())
}

/// Reject LLM copy that invents ranks / RP amounts not on the source.
pub fn grounded(body: &str, input: &NarrativeInput) -> bool {
    if body.trim().is_empty() {
        return false;
    }
    let allowed_ranks: Vec<String> = [input.from_rank, input.to_rank]
        .into_iter()
        .flatten()
        .map(|n| format!("P{n}"))
        .collect();
    let mut i = 0;
    let b = body.as_bytes();
    while i + 1 < b.len() {
        if b[i] == b'P' && b[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let token = &body[i..j];
            if !allowed_ranks.iter().any(|a| a == token) {
                return false;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    if let Some(delta) = input.rp_delta {
        let claim = format!("{delta} RP");
        if body.contains(" RP") && !body.contains(&claim) && !body.contains(&delta.to_string()) {
            // Mentions RP but not the source delta — treat as invented.
            return false;
        }
    } else if body.to_ascii_lowercase().contains(" rp") {
        return false;
    }
    true
}

/// Optional SuperGrok polish. Empty completion_url → templates. Failures → templates.
pub async fn polish(
    cfg: &SuperGrokOauthConfig,
    access_token: &str,
    channel: NarrativeChannel,
    template: &str,
    input: &NarrativeInput,
) -> Result<String, LlmError> {
    let _ = channel;
    if cfg.completion_url.is_empty() {
        return Err(LlmError::NotConfigured);
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| LlmError::Failed("http".into()))?;
    let payload = json!({
        "model": "grok-4",
        "messages": [
            {
                "role": "system",
                "content": "Polish this race copy. Do not invent ranks or RP amounts."
            },
            {
                "role": "user",
                "content": template
            }
        ]
    });
    let resp = client
        .post(&cfg.completion_url)
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|_| LlmError::Failed("http".into()))?;
    if !resp.status().is_success() {
        return Err(LlmError::Failed("http".into()));
    }
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| LlmError::Failed("parse".into()))?;
    let body = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("content").and_then(|v| v.as_str()))
        .ok_or_else(|| LlmError::Failed("parse".into()))?
        .to_string();
    if !grounded(&body, input) {
        return Err(LlmError::Failed("ungrounded".into()));
    }
    Ok(body)
}

fn oauth_cfg_from_env() -> SuperGrokOauthConfig {
    SuperGrokOauthConfig::from_parts(
        std::env::var("SUPERGROK_AUTHORIZE_URL").ok(),
        std::env::var("SUPERGROK_TOKEN_URL").ok(),
        std::env::var("SUPERGROK_CLIENT_ID").ok(),
        std::env::var("SUPERGROK_CLIENT_SECRET").ok(),
        std::env::var("SUPERGROK_REDIRECT_URI").ok(),
        std::env::var("SUPERGROK_COMPLETION_URL").ok(),
    )
}

fn check_ingest(state: &crate::AppState, headers: &HeaderMap) -> AppResult<()> {
    match &state.cfg.ingest_secret {
        None => Ok(()),
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

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub async fn login_handler(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    check_ingest(&state, &headers)?;
    let cfg = oauth_cfg_from_env();
    if !cfg.configured() {
        return Err(AppError::BadRequest("oauth not configured".into()));
    }
    let start = start_login(&state.db, &cfg).await?;
    Ok(Json(json!({
        "authorize_url": start.authorize_url,
        "state": start.state,
    })))
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn callback_handler(
    State(state): State<crate::AppState>,
    Query(q): Query<CallbackQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let cfg = oauth_cfg_from_env();
    exchange_code(&state.db, &cfg, &q.state, &q.code)
        .await
        .map_err(|_| AppError::Unauthorized)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn status_handler(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(json!({ "logged_in": logged_in(&state.db).await? })))
}

pub struct LoggedOutEnricher;

impl LlmEnricher for LoggedOutEnricher {
    fn enrich(
        &self,
        _channel: NarrativeChannel,
        _template: &str,
        _input: &NarrativeInput,
    ) -> Result<String, LlmError> {
        Err(LlmError::NotConfigured)
    }
}

pub struct RefreshFailEnricher;

impl LlmEnricher for RefreshFailEnricher {
    fn enrich(
        &self,
        _channel: NarrativeChannel,
        _template: &str,
        _input: &NarrativeInput,
    ) -> Result<String, LlmError> {
        Err(LlmError::Failed("oauth refresh failed".into()))
    }
}

/// In-memory enricher that applies grounding. Used by tests and as the last
/// step before accepting a live completion.
pub struct GroundedEnricher<F> {
    pub inner: F,
}

impl<F> LlmEnricher for GroundedEnricher<F>
where
    F: Fn(NarrativeChannel, &str, &NarrativeInput) -> Result<String, LlmError>,
{
    fn enrich(
        &self,
        channel: NarrativeChannel,
        template: &str,
        input: &NarrativeInput,
    ) -> Result<String, LlmError> {
        let raw = (self.inner)(channel, template, input)?;
        if grounded(&raw, input) {
            Ok(raw)
        } else {
            Err(LlmError::Failed("ungrounded completion".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::{generate_narrative, NarrativeSource};
    use crate::race_engine::RaceEventKind;
    use chrono::TimeZone;

    fn input() -> NarrativeInput {
        NarrativeInput {
            event_id: "evt-1".into(),
            occurred_at: Utc.with_ymd_and_hms(2026, 8, 25, 15, 4, 5).unwrap(),
            kind: RaceEventKind::Overtake,
            project_handle: "beta".into(),
            other_handle: Some("alpha".into()),
            title: "t".into(),
            summary: "s".into(),
            from_rank: Some(3),
            to_rank: Some(1),
            rp_delta: Some(12),
            window_slug: None,
            window_name: None,
        }
    }

    #[test]
    fn logged_out_and_refresh_fail_stay_on_templates() {
        let inp = input();
        let at = inp.occurred_at;
        let off = generate_narrative(&inp, None, at);
        let out = generate_narrative(&inp, Some(&LoggedOutEnricher), at);
        let fail = generate_narrative(&inp, Some(&RefreshFailEnricher), at);
        assert_eq!(off, out);
        assert_eq!(off, fail);
        assert!(off.posts.iter().all(|p| p.source == NarrativeSource::Template));
    }

    #[test]
    fn invented_rank_is_discarded() {
        let inp = input();
        let e = GroundedEnricher {
            inner: |_, _, _| Ok("beta jumped to P9 with 999 RP".into()),
        };
        let bundle = generate_narrative(&inp, Some(&e), inp.occurred_at);
        assert!(bundle
            .posts
            .iter()
            .all(|p| p.source == NarrativeSource::Template));
        assert!(bundle.posts.iter().all(|p| !p.body.contains("P9")));
    }

    #[test]
    fn grounded_polish_is_accepted() {
        let inp = input();
        let e = GroundedEnricher {
            inner: |_, tmpl, _| Ok(format!("POLISH P1 P3 12 RP {tmpl}")),
        };
        let bundle = generate_narrative(&inp, Some(&e), inp.occurred_at);
        assert!(bundle.posts.iter().all(|p| p.source == NarrativeSource::Llm));
        assert!(bundle.posts[0].body.starts_with("POLISH"));
    }

    #[test]
    fn rp_mention_without_source_delta_is_rejected() {
        let mut inp = input();
        inp.rp_delta = None;
        assert!(!grounded("they spent 50 RP", &inp));
        inp.rp_delta = Some(12);
        assert!(grounded("margin 12 RP at P1", &inp));
    }

    #[test]
    fn grounded_still_holds_on_source_facts() {
        let inp = input();
        assert!(grounded("beta to P1 from P3 with 12 RP", &inp));
        assert!(!grounded("beta jumped to P9", &inp));
        assert!(!grounded("", &inp));
    }

    #[tokio::test]
    async fn polish_empty_completion_url_is_not_configured() {
        let cfg = SuperGrokOauthConfig::from_parts(None, None, None, None, None, None);
        assert!(cfg.completion_url.is_empty());
        let err = polish(&cfg, "tok", NarrativeChannel::Timeline, "tmpl", &input())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::NotConfigured));
    }
}
