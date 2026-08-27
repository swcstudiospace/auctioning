//! HTTP handlers.

use crate::catalog;
use crate::championship;
use crate::featured;
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
            // Private-side entry for a fiat purchase. Matching public receipt
            // is log_paid_rp (paid dollars only — Afterburner pace stays off-chain).
            crate::events::credit_paid_with_card(
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
    // Advertised 1:1 — $1 (100 cents) buys 1 paid RP. Afterburner adds pace lots.
    cents / 100
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

pub async fn events_active(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let card = crate::events::active_card(&state.db, chrono::Utc::now())
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({
        "active": card,
        "note": "Paid RP stays $1=1. Active card adds event_multiplier pace only."
    })))
}

#[derive(Deserialize)]
pub struct OpenAfterburnerRequest {
    /// Hours Afterburner stays live. Default 48.
    #[serde(default)]
    pub hours: Option<i64>,
}

pub async fn open_afterburner(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    Json(req): Json<OpenAfterburnerRequest>,
) -> AppResult<Json<serde_json::Value>> {
    check_ingest(&state, &headers)?;
    let hours = req.hours.unwrap_or(48).clamp(1, 168);
    let card = crate::events::open_afterburner(&state.db, chrono::Duration::hours(hours))
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "opened": card })))
}

// ---------------------------------------------------------------------------
// Featured race + championship + calendar
// ---------------------------------------------------------------------------

pub async fn races_featured(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let windows = race_engine::list_windows(&state.db, 50)
        .await
        .map_err(AppError::from)?;
    let featured = featured_race_from_windows(&state.db, &windows, chrono::Utc::now()).await?;
    Ok(Json(json!({ "featured": featured })))
}

pub async fn championship_standings(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = championship::load_finished_session_rows(&state.db)
        .await
        .map_err(AppError::from)?;
    let results = championship::session_results_from_rows(&rows);
    let standings = championship::accumulate(&results);
    Ok(Json(json!({ "standings": standings })))
}

pub async fn races_calendar(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let windows = race_engine::list_windows(&state.db, 50)
        .await
        .map_err(AppError::from)?;
    let active_card = crate::events::active_card(&state.db, now)
        .await
        .map_err(AppError::from)?;
    let featured = featured_race_from_windows(&state.db, &windows, now).await?;
    Ok(Json(json!({
        "windows": windows,
        "active_card": active_card,
        "featured": featured,
    })))
}

async fn featured_race_from_windows(
    db: &sqlx::PgPool,
    windows: &[race_engine::RaceWindowRow],
    now: chrono::DateTime<chrono::Utc>,
) -> AppResult<Option<featured::FeaturedRace>> {
    let mut signals = Vec::new();
    for window in windows {
        if !window.status.eq_ignore_ascii_case("live") {
            continue;
        }
        let (grid, _pending) = race_engine::grid_for_window(db, window)
            .await
            .map_err(AppError::from)?;
        let events = race_engine::events_for_window(db, window.id, 100)
            .await
            .map_err(AppError::from)?;
        let unique_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT supporter_wallet)::bigint
            FROM project_allocations
            WHERE created_at >= $1 AND created_at <= $2 AND source = 'paid'
            "#,
        )
        .bind(window.starts_at)
        .bind(window.ends_at.min(now))
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;
        signals.push(featured_signals_for(
            window,
            &grid,
            &events,
            now,
            unique_payers_signal(unique_count),
        ));
    }
    Ok(featured::pick_featured(&signals))
}

fn featured_signals_for(
    window: &race_engine::RaceWindowRow,
    grid: &[race_engine::GridSlot],
    events: &[race_engine::RaceEventRow],
    now: chrono::DateTime<chrono::Utc>,
    unique_payers: i64,
) -> featured::FeaturedSignals {
    let overtake_count = events
        .iter()
        .filter(|e| e.event_type.eq_ignore_ascii_case("overtake"))
        .count() as i64;
    let has_photo = events
        .iter()
        .any(|e| e.event_type.eq_ignore_ascii_case("photo_finish"));
    let has_dark_horse = events
        .iter()
        .any(|e| e.event_type.eq_ignore_ascii_case("dark_horse_rise"));

    featured::FeaturedSignals {
        window_slug: window.slug.clone(),
        window_name: window.name.clone(),
        overtake_density: (overtake_count.saturating_mul(20)).clamp(0, 100),
        photo_finish_pressure: photo_finish_pressure(grid, has_photo),
        unique_payers,
        mix: mix_score(grid),
        time_remaining: time_remaining_score(window, now),
        freshness: if has_dark_horse { 100 } else { 0 },
        attention: 0,
        overtakes_in_window: overtake_count,
        p1_p3_cover_rp: p1_p3_cover_rp(grid),
    }
}

fn unique_payers_signal(count: i64) -> i64 {
    (count.saturating_mul(10)).clamp(0, 100)
}

fn photo_finish_pressure(grid: &[race_engine::GridSlot], has_photo: bool) -> i64 {
    if !has_photo {
        return 0;
    }
    const LARGE: i64 = 10_000;
    let mut gaps: Vec<i64> = grid
        .iter()
        .filter(|s| (1..=5).contains(&s.rank))
        .map(|s| s.gap_to_next.unwrap_or(LARGE))
        .collect();
    if gaps.is_empty() {
        return 0;
    }
    gaps.sort_unstable();
    let n = gaps.len();
    let median = if n % 2 == 1 {
        gaps[n / 2]
    } else {
        let a = gaps[n / 2 - 1];
        let b = gaps[n / 2];
        a.saturating_add(b) / 2
    };
    (100i64.saturating_sub(median.saturating_mul(10))).clamp(0, 100)
}

fn mix_score(grid: &[race_engine::GridSlot]) -> i64 {
    let mut any_paid = false;
    let mut any_community = false;
    for slot in grid.iter().filter(|s| (1..=5).contains(&s.rank)) {
        if slot.paid_rp > 0 {
            any_paid = true;
        }
        if slot.community_rp > 0 {
            any_community = true;
        }
    }
    match (any_paid, any_community) {
        (true, true) => 100,
        (true, false) => 40,
        (false, true) => 20,
        (false, false) => 0,
    }
}

fn time_remaining_score(
    window: &race_engine::RaceWindowRow,
    now: chrono::DateTime<chrono::Utc>,
) -> i64 {
    let remaining = (window.ends_at - now).num_seconds().max(0);
    let sprint = window.race_type.eq_ignore_ascii_case("GREEN_FLAG")
        || window.race_type.eq_ignore_ascii_case("SPRINT")
        || window.race_type.eq_ignore_ascii_case("PACE_LAP");
    if sprint && remaining < 3600 {
        60.max((remaining.saturating_mul(100) / 3600).clamp(0, 100))
    } else {
        let window_secs = (window.ends_at - window.starts_at).num_seconds().max(1);
        remaining
            .saturating_mul(100)
            .saturating_div(window_secs)
            .clamp(0, 100)
    }
}

fn p1_p3_cover_rp(grid: &[race_engine::GridSlot]) -> i64 {
    if grid.len() < 2 {
        0
    } else {
        let p3 = 2.min(grid.len() - 1);
        grid[0].race_rp.saturating_sub(grid[p3].race_rp)
    }
}

#[cfg(test)]
mod featured_signal_tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn slot(
        rank: i32,
        race_rp: i64,
        gap: Option<i64>,
        paid: i64,
        community: i64,
    ) -> race_engine::GridSlot {
        race_engine::GridSlot {
            handle: format!("p{rank}"),
            rank,
            race_rp,
            velocity: 0,
            momentum: 0,
            gap_to_leader: 0,
            gap_to_next: gap,
            pace_pct: None,
            lifetime_rank: None,
            burst_rp: 0,
            sustain_windows: 0,
            paid_rp: paid,
            community_rp: community,
            badge: None,
            last_overtake: None,
            clicks: 0,
            hover_footer: String::new(),
        }
    }

    fn window(race_type: &str, starts: chrono::DateTime<Utc>, ends: chrono::DateTime<Utc>) -> race_engine::RaceWindowRow {
        race_engine::RaceWindowRow {
            id: uuid::Uuid::from_u128(1),
            slug: "green".into(),
            name: "Green Flag".into(),
            race_type: race_type.into(),
            status: "live".into(),
            tag: None,
            starts_at: starts,
            ends_at: ends,
        }
    }

    #[test]
    fn mix_paid_and_community_is_100() {
        let grid = vec![
            slot(1, 100, Some(10), 80, 20),
            slot(2, 90, Some(5), 0, 90),
        ];
        assert_eq!(mix_score(&grid), 100);
    }

    #[test]
    fn mix_paid_only_is_40() {
        let grid = vec![slot(1, 100, Some(10), 100, 0), slot(2, 50, None, 50, 0)];
        assert_eq!(mix_score(&grid), 40);
    }

    #[test]
    fn mix_community_only_is_20() {
        let grid = vec![slot(1, 100, Some(10), 0, 100)];
        assert_eq!(mix_score(&grid), 20);
    }

    #[test]
    fn p1_minus_p3_cover() {
        let grid = vec![
            slot(1, 100, Some(10), 0, 0),
            slot(2, 90, Some(40), 0, 0),
            slot(3, 50, None, 0, 0),
        ];
        assert_eq!(p1_p3_cover_rp(&grid), 50);
    }

    #[test]
    fn p1_p3_with_two_slots_uses_last() {
        let grid = vec![slot(1, 100, Some(20), 0, 0), slot(2, 80, None, 0, 0)];
        assert_eq!(p1_p3_cover_rp(&grid), 20);
    }

    #[test]
    fn photo_pressure_zero_without_event() {
        let grid = vec![slot(1, 100, Some(2), 0, 0)];
        assert_eq!(photo_finish_pressure(&grid, false), 0);
    }

    #[test]
    fn photo_pressure_from_median_gap() {
        // ranks 1..=5 gaps 2,4,6 → median 4 → 100-40 = 60
        let grid = vec![
            slot(1, 100, Some(2), 0, 0),
            slot(2, 98, Some(4), 0, 0),
            slot(3, 94, Some(6), 0, 0),
        ];
        assert_eq!(photo_finish_pressure(&grid, true), 60);
    }

    #[test]
    fn sprint_under_an_hour_floors_at_60() {
        let now = Utc.with_ymd_and_hms(2026, 8, 27, 12, 0, 0).unwrap();
        let w = window(
            "GREEN_FLAG",
            now - Duration::hours(1),
            now + Duration::minutes(10),
        );
        assert_eq!(time_remaining_score(&w, now), 60);
    }

    #[test]
    fn unique_payers_signal_maps_count() {
        assert_eq!(unique_payers_signal(0), 0);
        assert_eq!(unique_payers_signal(3), 30);
        assert_eq!(unique_payers_signal(10), 100);
        assert_eq!(unique_payers_signal(99), 100);
    }
}
