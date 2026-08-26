//! HTTP handlers.

use crate::catalog;
use crate::error::{AppError, AppResult};
use crate::ledger;
use crate::narrative;
use crate::oauth_llm;
use crate::onchain::{
    self, PrepareLogPaidRequest, PrepareLogPaidResponse, PrepareOpenRaceRequest,
    PrepareOpenRaceResponse, PrepareRegisterRequest, PrepareRegisterResponse,
    PrepareSettleRaceRequest, PrepareSettleRaceResponse,
};
use crate::race_engine;
use crate::whop::{self, WhopWebhookEvent};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub async fn health(State(_state): State<crate::AppState>) -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "service": "auctioning-backend" }))
}

#[derive(Serialize)]
pub struct RpView {
    pub wallet: String,
    pub paid_rp: i64,
    pub free_rp: i64,
    pub spent_rp: i64,
    /// Free RP is promotional and can never be cashed out.
    pub free_rp_non_cashable: bool,
}

impl From<ledger::WalletLedger> for RpView {
    fn from(l: ledger::WalletLedger) -> Self {
        Self {
            wallet: l.wallet,
            paid_rp: l.paid_rp,
            free_rp: l.free_rp,
            spent_rp: l.spent_rp,
            free_rp_non_cashable: true,
        }
    }
}

pub async fn get_rp(
    State(state): State<crate::AppState>,
    Path(wallet): Path<String>,
) -> AppResult<Json<RpView>> {
    let row = ledger::ensure_wallet(&state.db, &wallet)
        .await
        .map_err(AppError::from)?;
    Ok(Json(row.into()))
}

#[derive(Deserialize)]
pub struct EarnRequest {
    pub wallet: String,
    /// RP earned from gameplay/narrative actions (free bucket).
    pub amount: i64,
    pub reason: String,
}

/// Gameplay/narrative ingest. Requires the shared ingest secret so arbitrary
/// clients cannot mint free RP (`X-Auctioning-Ingest` header). Earnings land
/// in the FREE bucket — never the paid/cashable one.
pub async fn earn_rp(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    Json(req): Json<EarnRequest>,
) -> AppResult<Json<RpView>> {
    check_ingest(&state, &headers)?;
    if req.amount <= 0 || req.amount > 10_000 {
        return Err(AppError::BadRequest("amount out of range".into()));
    }
    if req.reason.len() > 128 {
        return Err(AppError::BadRequest("reason too long".into()));
    }
    // Gameplay/narrative earnings land in the FREE bucket — never the paid/
    // cashable one — as a bonus lot that expires with the current promo week.
    let expires_at = ledger::next_week_start(chrono::Utc::now());
    let updated = ledger::grant_free_lot(
        &state.db,
        &req.wallet,
        req.amount,
        ledger::RpSource::Bonus,
        &req.reason,
        expires_at,
    )
    .await
    .map_err(AppError::from)?;
    Ok(Json(updated.into()))
}

fn check_ingest(state: &crate::AppState, headers: &HeaderMap) -> AppResult<()> {
    match &state.cfg.ingest_secret {
        None => Ok(()), // dev mode: open ingest, documented in README
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

#[derive(Deserialize)]
pub struct WeeklyClaimRequest {
    pub wallet: String,
}

pub async fn claim_weekly(
    State(state): State<crate::AppState>,
    Json(req): Json<WeeklyClaimRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let amount = state.cfg.weekly_free_rp;
    // Stipends expire when the week rolls over: next Monday 00:00 UTC.
    let expires_at = ledger::next_week_start(chrono::Utc::now());
    match ledger::claim_weekly(&state.db, &req.wallet, amount, expires_at)
        .await
        .map_err(AppError::from)?
    {
        Some(a) => Ok(Json(json!({
            "claimed": true,
            "amount": a,
            "expires_at": expires_at.to_rfc3339(),
            "note": "promotional RP — non-cashable, non-transferable, expires with the week"
        }))),
        None => Err(AppError::RateLimited), // already claimed this week
    }
}

#[derive(Deserialize)]
pub struct SpendRequest {
    pub wallet: String,
    pub amount: i64,
    pub reason: String,
}

/// Spend RP (bids, race entries, cosmetics). Free bucket drains first; the
/// response records the split for transparency. Atomic — no partial spends.
pub async fn spend_rp(
    State(state): State<crate::AppState>,
    Json(req): Json<SpendRequest>,
) -> AppResult<Json<ledger::SpendBreakdown>> {
    if req.amount <= 0 || req.amount > 1_000_000 {
        return Err(AppError::BadRequest("amount out of range".into()));
    }
    if req.reason.len() > 128 {
        return Err(AppError::BadRequest("reason too long".into()));
    }
    match ledger::spend(&state.db, &req.wallet, req.amount, &req.reason)
        .await
        .map_err(AppError::from)?
    {
        Some(breakdown) => Ok(Json(breakdown)),
        None => Err(AppError::InsufficientFunds),
    }
}

#[derive(Deserialize)]
pub struct ContentReadRequest {
    pub wallet: String,
    pub slug: String,
}

pub async fn list_content(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let items = ledger::list_content(&state.db)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "items": items })))
}

pub async fn content_read(
    State(state): State<crate::AppState>,
    Json(req): Json<ContentReadRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Content bonus RP expires with the current promo week as well.
    let expires_at = ledger::next_week_start(chrono::Utc::now());
    match ledger::content_read_reward(&state.db, &req.wallet, &req.slug, expires_at)
        .await
        .map_err(AppError::from)?
    {
        Some((reward, note)) => Ok(Json(json!({
            "rewarded": true,
            "amount": reward,
            "expires_at": expires_at.to_rfc3339(),
            "note": note
        }))),
        None => Err(AppError::RateLimited), // already read / unknown slug
    }
}

/// Whop webhook: verify HMAC, parse, dual-write to the private ledger.
/// The on-chain side of the dual-write is executed by the payer's own
/// log_paid_rp transaction (client-side), keeping custody with the payer.
pub async fn whop_webhook(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let secret = state
        .cfg
        .whop_webhook_secret
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("WHOP_WEBHOOK_SECRET unset")))?;

    let sig = headers
        .get("x-whop-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if !whop::verify_webhook_signature(secret, &body, sig) {
        return Err(AppError::Unauthorized);
    }

    let ev: WhopWebhookEvent =
        serde_json::from_slice(&body).map_err(|e| AppError::BadRequest(e.to_string()))?;
    let paid = whop::parse_paid_event(&ev);

    match (&paid.wallet, ev.r#type.as_str()) {
        (Some(wallet), t) if t.contains("payment") || t.contains("membership") => {
            // Private-side entry for a fiat purchase. The matching public
            // receipt appears when the payer's wallet signs log_paid_rp.
            ledger::credit_paid(
                &state.db,
                wallet,
                rp_from_cents(paid.amount_cents.unwrap_or(0)),
                &format!("whop:{t}"),
                paid.payment_id.as_deref(),
            )
            .await
            .map_err(AppError::from)?;
            tracing::info!(wallet, event = %t, "whop paid event recorded");
            Ok(Json(json!({ "ok": true, "recorded": true })))
        }
        _ => {
            // Unhandled/irrelevant event types are ACKed to stop retries.
            Ok(Json(json!({ "ok": true, "ignored": true })))
        }
    }
}

fn rp_from_cents(cents: i64) -> i64 {
    // Pricing v1: 100c (= $1) buys 1_000 RP. Configurable later via app settings.
    cents.saturating_mul(10)
}

/// Server-side membership verification against Whop's REST API.
pub async fn whop_membership(
    State(state): State<crate::AppState>,
    Path(wallet): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let Some(key) = state.cfg.whop_api_key.clone() else {
        return Err(AppError::Internal(anyhow::anyhow!("WHOP_API_KEY unset")));
    };
    let valid = whop::has_active_membership(&key, &wallet)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("whop lookup failed: {e}")))?;
    Ok(Json(
        json!({ "wallet": wallet, "active_membership": valid }),
    ))
}

// ---------------------------------------------------------------------------
// Race session bookkeeping (private side; public mirror settles on-chain)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct OpenRaceRequest {
    pub project_pda: String,
    /// Optional MagicBlock ER session identifier.
    pub er_session: Option<String>,
}

pub async fn races_open(
    State(state): State<crate::AppState>,
    Json(req): Json<OpenRaceRequest>,
) -> AppResult<Json<ledger::RaceRow>> {
    if req.project_pda.is_empty() || req.project_pda.len() > 64 {
        return Err(AppError::BadRequest("project_pda invalid".into()));
    }
    let row = ledger::open_race_row(&state.db, &req.project_pda, req.er_session.as_deref())
        .await
        .map_err(AppError::from)?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct SettleRaceRequest {
    /// On-chain signature of the settle_race transaction on mainnet.
    pub settle_tx: String,
}

pub async fn races_settle(
    State(state): State<crate::AppState>,
    Path((project_pda, race_id)): Path<(String, i64)>,
    Json(req): Json<SettleRaceRequest>,
) -> AppResult<Json<ledger::RaceRow>> {
    let row = ledger::settle_race_row(&state.db, &project_pda, race_id, &req.settle_tx)
        .await
        .map_err(AppError::from)?;
    Ok(Json(row))
}

pub async fn races_list(
    State(state): State<crate::AppState>,
    Path(project_pda): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = ledger::races_for(&state.db, &project_pda)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "project_pda": project_pda, "races": rows })))
}

/// Link out to the public Solana explorer view of this project's ledger.
pub async fn public_ledger_link(
    State(state): State<crate::AppState>,
    Path(wallet): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "wallet": wallet,
        "cluster": "mainnet-beta",
        "explorer": format!("https://explorer.solana.com/address/{}", wallet),
        "program_id": state.cfg.program_id,
        "note": "paid RP provenance lives on-chain; free RP is off-chain only"
    }))
}

// ---------------------------------------------------------------------------
// Project catalog: seeding/import, board, support (priority #1 surface)
// ---------------------------------------------------------------------------

/// Import/seed projects. Idempotent upserts keyed by stable_id. Requires the
/// ingest secret like earn_rp so random clients cannot rewrite the catalog.
#[derive(Deserialize)]
pub struct ImportRequest {
    pub projects: Vec<catalog::ImportProject>,
}

pub async fn import_projects(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    Json(req): Json<ImportRequest>,
) -> AppResult<Json<serde_json::Value>> {
    check_ingest(&state, &headers)?;
    if req.projects.is_empty() {
        return Err(AppError::BadRequest("empty import batch".into()));
    }
    if req.projects.len() > 5_000 {
        return Err(AppError::BadRequest("batch too large (max 5000)".into()));
    }
    let (imported, updated) = catalog::import_projects(&state.db, &req.projects)
        .await
        .map_err(AppError::from)?;
    tracing::info!(imported, updated, "project import applied");
    Ok(Json(json!({ "imported": imported, "updated": updated })))
}

pub async fn list_projects(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let projects = catalog::list_projects(&state.db, 500)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "projects": projects })))
}

pub async fn get_project(
    State(state): State<crate::AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    match catalog::get_project(&state.db, &handle)
        .await
        .map_err(AppError::from)?
    {
        Some(p) => Ok(Json(json!(p))),
        None => Err(AppError::NotFound),
    }
}

#[derive(Deserialize)]
pub struct SupportRequest {
    pub wallet: String,
    pub amount: i64,
    /// Optional free-text why ("their new app went viral").
    pub reason: Option<String>,
}

/// Allocate RP to a project — free lots drain FIFO first, then paid.
/// Atomic; writes one immutable allocation row per call.
pub async fn support_project(
    State(state): State<crate::AppState>,
    Path(handle): Path<String>,
    Json(req): Json<SupportRequest>,
) -> AppResult<Json<catalog::SupportOutcome>> {
    if req.amount <= 0 || req.amount > 1_000_000 {
        return Err(AppError::BadRequest("amount out of range".into()));
    }
    if req.reason.as_ref().is_some_and(|r| r.len() > 128) {
        return Err(AppError::BadRequest("reason too long".into()));
    }
    match catalog::allocate_to_project(
        &state.db,
        &req.wallet,
        &handle,
        req.amount,
        req.reason.as_deref(),
    )
    .await
    .map_err(AppError::from)?
    {
        Some(outcome) => Ok(Json(outcome)),
        // Unknown wallet/project and insufficient funds all land here; a 404
        // for unknown projects vs. conflict for funds keeps clients informed.
        None => {
            let known = catalog::get_project(&state.db, &handle)
                .await
                .map_err(AppError::from)?
                .is_some();
            if known {
                Err(AppError::InsufficientFunds)
            } else {
                Err(AppError::NotFound)
            }
        }
    }
}

pub async fn project_allocations(
    State(state): State<crate::AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    if catalog::get_project(&state.db, &handle)
        .await
        .map_err(AppError::from)?
        .is_none()
    {
        return Err(AppError::NotFound);
    }
    let rows = catalog::allocations_for(&state.db, &handle, 200)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "allocations": rows })))
}

/// Prepare an unsigned register_project transaction for the connected wallet.
/// Client signs with Phantom and broadcasts. Wires dApp <-> Anchor.
pub async fn prepare_register_project(
    State(state): State<crate::AppState>,
    Json(req): Json<PrepareRegisterRequest>,
) -> AppResult<Json<PrepareRegisterResponse>> {
    let resp = onchain::prepare_register_project(&state.cfg, req)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(resp))
}

/// Prepare log_paid_rp tx for client to sign. This logs paid RP as immutable receipt on-chain.
pub async fn prepare_log_paid_rp(
    State(state): State<crate::AppState>,
    Json(req): Json<PrepareLogPaidRequest>,
) -> AppResult<Json<PrepareLogPaidResponse>> {
    let resp = onchain::prepare_log_paid_rp(&state.cfg, req)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(resp))
}
pub async fn prepare_open_race(
    State(state): State<crate::AppState>,
    Json(req): Json<PrepareOpenRaceRequest>,
) -> AppResult<Json<PrepareOpenRaceResponse>> {
    let resp = onchain::prepare_open_race(&state.cfg, req)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(resp))
}
pub async fn prepare_settle_race(
    State(state): State<crate::AppState>,
    Json(req): Json<PrepareSettleRaceRequest>,
) -> AppResult<Json<PrepareSettleRaceResponse>> {
    let resp = onchain::prepare_settle_race(&state.cfg, req)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(resp))
}

// ---------------------------------------------------------------------------
// Race engine: live grid + snapshots + narrative events
// ---------------------------------------------------------------------------

pub async fn lifetime_grid(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let grid = race_engine::lifetime_grid(&state.db)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "grid": grid })))
}

pub async fn list_race_windows(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let windows = race_engine::list_windows(&state.db, 50)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "windows": windows })))
}

pub async fn race_window_grid(
    State(state): State<crate::AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    let (grid, events) = race_engine::grid_for_window(&state.db, &window)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({
        "window": window,
        "grid": grid,
        "pending_events": events,
    })))
}

pub async fn race_window_snapshot(
    State(state): State<crate::AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    let (grid, events) = race_engine::persist_snapshot(&state.db, &window)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({
        "window": window,
        "grid": grid,
        "events": events,
    })))
}

pub async fn race_window_events(
    State(state): State<crate::AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    let events = race_engine::events_for_window(&state.db, window.id, 100)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "window": window.slug, "events": events })))
}

/// SLICE A: template-first narrative bundle for one persisted race event.
/// Optional LLM is not required; missing/failed polish stays on templates.
pub async fn narrate_event(
    State(state): State<crate::AppState>,
    Path((slug, event_id)): Path<(String, uuid::Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    let row = race_engine::event_by_id(&state.db, event_id)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    if row.race_window_id != window.id {
        return Err(AppError::NotFound);
    }
    let input = narrative::NarrativeInput::from_row(&row, Some(&window)).ok_or_else(|| {
        AppError::BadRequest("event is missing a project or has an unknown type".into())
    })?;
    let mut bundle = narrative::generate_narrative(&input, None, chrono::Utc::now());
    let oauth_cfg = state.cfg.supergrok();
    if !oauth_cfg.completion_url.is_empty() {
        if let Ok(Some(token)) = oauth_llm::load_access_token(&state.db).await {
            for post in bundle.posts.iter_mut() {
                match oauth_llm::polish(&oauth_cfg, &token, post.channel, &post.body, &input).await
                {
                    Ok(body) => {
                        post.body = body;
                        post.source = narrative::NarrativeSource::Llm;
                    }
                    Err(_) => {}
                }
            }
        }
    }
    let stored = narrative::persist_bundle(&state.db, event_id, &bundle)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({
        "event_id": event_id,
        "window": window.slug,
        "bundle": bundle,
        "stored": stored,
    })))
}

pub async fn race_window_tape(
    State(state): State<crate::AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;
    let posts = narrative::tape_for_window(&state.db, window.id, 100)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "window": window.slug, "posts": posts })))
}
