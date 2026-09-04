//! Business-intelligence surface.
//!
//! Everything here is a read over tables the engine already writes
//! (`project_allocations`, `rank_snapshots`, `ledger_events`, `projects`).
//! No new truth is created: a stat is a fold, never a stored counter.
//!
//! * `GET /v1/stats/overview`            — platform pulse (public, cached 30 s)
//! * `GET /v1/projects/{handle}/stats`   — one car's telemetry (public)
//! * `GET /v1/wallets/me/history`        — the caller's own ledger (session)
//! * `GET /v1/stats/revenue`             — paid-RP revenue by day (operator)

use crate::auth::{AuthedWallet, Operator};
use crate::error::{AppError, AppResult};
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Overview {
    pub projects: i64,
    pub wallets: i64,
    pub paid_rp_total: i64,
    pub free_rp_active: i64,
    pub allocations_24h: i64,
    pub race_rp_24h: i64,
    pub unique_supporters_7d: i64,
    pub paid_rp_7d: i64,
    pub live_windows: i64,
}

pub async fn overview(db: &PgPool) -> Result<Overview, sqlx::Error> {
    sqlx::query_as::<_, Overview>(
        r#"
        SELECT
          (SELECT COUNT(*) FROM projects)::bigint                                   AS projects,
          (SELECT COUNT(*) FROM wallets)::bigint                                    AS wallets,
          (SELECT COALESCE(SUM(amount), 0) FROM ledger_events WHERE kind = 'paid')::bigint
                                                                                    AS paid_rp_total,
          (SELECT COALESCE(SUM(remaining), 0) FROM free_rp_lots
             WHERE remaining > 0 AND expires_at > now())::bigint                    AS free_rp_active,
          (SELECT COUNT(*) FROM project_allocations
             WHERE created_at > now() - interval '24 hours')::bigint                AS allocations_24h,
          (SELECT COALESCE(SUM(amount), 0) FROM project_allocations
             WHERE created_at > now() - interval '24 hours')::bigint                AS race_rp_24h,
          (SELECT COUNT(DISTINCT supporter_wallet) FROM project_allocations
             WHERE created_at > now() - interval '7 days')::bigint                  AS unique_supporters_7d,
          (SELECT COALESCE(SUM(amount), 0) FROM project_allocations
             WHERE source = 'paid' AND created_at > now() - interval '7 days')::bigint
                                                                                    AS paid_rp_7d,
          (SELECT COUNT(*) FROM race_windows WHERE status = 'live')::bigint         AS live_windows
        "#,
    )
    .fetch_one(db)
    .await
}

type OverviewCache = Mutex<Option<(Instant, Overview)>>;

fn overview_cache() -> &'static OverviewCache {
    static CACHE: OnceLock<OverviewCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub async fn overview_handler(
    State(state): State<crate::AppState>,
) -> AppResult<Json<serde_json::Value>> {
    if let Ok(c) = overview_cache().lock() {
        if let Some((at, o)) = c.as_ref() {
            if at.elapsed().as_secs() < 30 {
                return Ok(Json(json!({ "overview": o, "cached": true })));
            }
        }
    }
    let o = overview(&state.db).await?;
    if let Ok(mut c) = overview_cache().lock() {
        *c = Some((Instant::now(), o.clone()));
    }
    Ok(Json(json!({ "overview": o, "cached": false })))
}

// ---------------------------------------------------------------------------
// Per-project telemetry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProjectLifetime {
    pub race_rp: i64,
    pub paid_rp: i64,
    pub community_rp: i64,
    pub supporters: i64,
    pub allocations: i64,
    pub first_fuel_at: Option<DateTime<Utc>>,
    pub last_fuel_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RankPoint {
    pub window_slug: String,
    pub race_type: String,
    pub snapshot_at: DateTime<Utc>,
    pub rank: i32,
    pub race_rp: i64,
    pub velocity: i64,
    pub momentum: i64,
    pub paid_rp: i64,
    pub community_rp: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FinishRow {
    pub window_slug: String,
    pub race_type: String,
    pub rank: i32,
    pub race_rp: i64,
    pub ends_at: DateTime<Utc>,
}

pub async fn project_lifetime(db: &PgPool, handle: &str) -> Result<ProjectLifetime, sqlx::Error> {
    sqlx::query_as::<_, ProjectLifetime>(
        r#"
        SELECT COALESCE(SUM(amount), 0)::bigint                                    AS race_rp,
               COALESCE(SUM(amount) FILTER (WHERE source = 'paid'), 0)::bigint     AS paid_rp,
               COALESCE(SUM(amount) FILTER (WHERE source <> 'paid'), 0)::bigint    AS community_rp,
               COUNT(DISTINCT supporter_wallet)::bigint                             AS supporters,
               COUNT(*)::bigint                                                     AS allocations,
               MIN(created_at)                                                      AS first_fuel_at,
               MAX(created_at)                                                      AS last_fuel_at
        FROM project_allocations
        WHERE project_handle = $1
        "#,
    )
    .bind(handle)
    .fetch_one(db)
    .await
}

pub async fn rank_history(
    db: &PgPool,
    handle: &str,
    limit: i64,
) -> Result<Vec<RankPoint>, sqlx::Error> {
    sqlx::query_as::<_, RankPoint>(
        r#"
        SELECT w.slug AS window_slug, w.race_type, s.snapshot_at, s.rank, s.race_rp,
               s.velocity, s.momentum, s.paid_rp, s.community_rp
        FROM rank_snapshots s
        JOIN race_windows w ON w.id = s.race_window_id
        WHERE s.project_handle = $1
        ORDER BY s.snapshot_at DESC
        LIMIT $2
        "#,
    )
    .bind(handle)
    .bind(limit.clamp(1, 500))
    .fetch_all(db)
    .await
}

pub async fn finishes(db: &PgPool, handle: &str) -> Result<Vec<FinishRow>, sqlx::Error> {
    sqlx::query_as::<_, FinishRow>(
        r#"
        SELECT window_slug, race_type, rank, race_rp, ends_at
        FROM v_window_finals
        WHERE project_handle = $1 AND status IN ('finished', 'archived')
        ORDER BY ends_at DESC
        LIMIT 100
        "#,
    )
    .bind(handle)
    .fetch_all(db)
    .await
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
}

pub async fn project_stats_handler(
    State(state): State<crate::AppState>,
    Path(handle): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let project = crate::catalog::get_project(&state.db, &handle)
        .await?
        .ok_or(AppError::NotFound)?;
    let lifetime = project_lifetime(&state.db, &handle).await?;
    let history = rank_history(&state.db, &handle, q.limit.unwrap_or(50)).await?;
    let finishes = finishes(&state.db, &handle).await?;
    let wins = finishes.iter().filter(|f| f.rank == 1).count();
    let podiums = finishes.iter().filter(|f| f.rank <= 3).count();
    let best_rank = finishes.iter().map(|f| f.rank).min();
    let paid_share_pct = if lifetime.race_rp > 0 {
        Some(lifetime.paid_rp * 100 / lifetime.race_rp)
    } else {
        None
    };
    Ok(Json(json!({
        "project": project,
        "lifetime": lifetime,
        "paid_share_pct": paid_share_pct,
        "record": {
            "starts": finishes.len(),
            "wins": wins,
            "podiums": podiums,
            "best_rank": best_rank,
        },
        "finishes": finishes,
        "rank_history": history,
    })))
}

// ---------------------------------------------------------------------------
// Wallet history (own data only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LedgerRow {
    pub kind: String,
    pub source: Option<String>,
    pub amount: i64,
    pub reason: String,
    pub tx_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AllocationRow {
    pub project_handle: String,
    pub amount: i64,
    pub bucket: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LotRow {
    pub amount: i64,
    pub remaining: i64,
    pub source: String,
    pub reason: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub async fn wallet_history_handler(
    State(state): State<crate::AppState>,
    AuthedWallet(wallet): AuthedWallet,
    Query(q): Query<HistoryQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let balance = crate::ledger::ensure_wallet(&state.db, &wallet).await?;
    let events: Vec<LedgerRow> = sqlx::query_as(
        r#"
        SELECT kind, source, amount, reason, tx_id, created_at
        FROM ledger_events WHERE wallet = $1
        ORDER BY created_at DESC LIMIT $2
        "#,
    )
    .bind(&wallet)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    let allocations: Vec<AllocationRow> = sqlx::query_as(
        r#"
        SELECT project_handle, amount, bucket, source, created_at
        FROM project_allocations WHERE supporter_wallet = $1
        ORDER BY created_at DESC LIMIT $2
        "#,
    )
    .bind(&wallet)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    let lots: Vec<LotRow> = sqlx::query_as(
        r#"
        SELECT amount, remaining, source, reason, granted_at, expires_at
        FROM free_rp_lots WHERE wallet = $1 AND remaining > 0 AND expires_at > now()
        ORDER BY expires_at ASC
        "#,
    )
    .bind(&wallet)
    .fetch_all(&state.db)
    .await?;
    let next_expiry = lots.first().map(|l| l.expires_at);
    Ok(Json(json!({
        "wallet": wallet,
        "balance": balance,
        "active_lots": lots,
        "next_free_rp_expiry": next_expiry,
        "events": events,
        "allocations": allocations,
        "note": "free RP is promotional, non-cashable and expires per lot",
    })))
}

// ---------------------------------------------------------------------------
// Revenue (operator)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RevenueDay {
    pub day: DateTime<Utc>,
    pub paid_rp: i64,
    pub purchases: i64,
    pub buyers: i64,
    pub bonus_rp: i64,
}

#[derive(Debug, Deserialize)]
pub struct RevenueQuery {
    pub days: Option<i64>,
}

pub async fn revenue_handler(
    State(state): State<crate::AppState>,
    _op: Operator,
    Query(q): Query<RevenueQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let rows: Vec<RevenueDay> = sqlx::query_as(
        r#"
        SELECT date_trunc('day', created_at)                                          AS day,
               COALESCE(SUM(amount) FILTER (WHERE kind = 'paid'), 0)::bigint           AS paid_rp,
               COUNT(*) FILTER (WHERE kind = 'paid')::bigint                           AS purchases,
               COUNT(DISTINCT wallet) FILTER (WHERE kind = 'paid')::bigint             AS buyers,
               COALESCE(SUM(amount) FILTER (WHERE source = 'event_multiplier'), 0)::bigint
                                                                                        AS bonus_rp
        FROM ledger_events
        WHERE created_at > now() - ($1::bigint * interval '1 day')
          AND kind IN ('paid', 'free')
        GROUP BY 1
        ORDER BY 1 DESC
        "#,
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    let total_paid: i64 = rows.iter().map(|r| r.paid_rp).sum();
    Ok(Json(json!({
        "days": days,
        "total_paid_rp": total_paid,
        "usd_equivalent": total_paid,
        "series": rows,
        "note": "$1 = 1 paid RP; bonus_rp is event_multiplier pace and is not revenue",
    })))
}
