//! Shuttle backend for auctioning.lol.
//!
//! Private RP ledger (Postgres), free weekly promo RP, Whop webhook dual-write
//! and the race settlement worker. Public paid RP lives on-chain (Anchor
//! program); this service is the source of truth for everything private/free.
//!
//! Priority #1 additions: typed RP sources, FIFO expiry lots for promo RP,
//! project catalog with idempotent outbid.lol import path, and the immutable
//! per-project allocation ledger.

// Public for the Postgres-backed smoke test (tests/smoke_db.rs); internal
// modules stay `pub` but the crate surface is otherwise unused externally.
pub mod catalog;
pub mod championship;
pub mod config;
mod error;
pub mod events;
pub mod featured;
mod handlers;
pub mod ledger;
pub mod narrative;
pub mod oauth_llm;
mod onchain;
pub mod publish;
pub mod race_engine;
pub mod race_worker;
pub mod ticks;
mod whop;

use axum::routing::{get, post};
use axum::Router;
use shuttle_runtime::SecretStore;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub cfg: Arc<config::AppConfig>,
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let cfg = config::AppConfig::from_secret_store(&secrets);
    Ok(build_app(pool, cfg).await.into())
}

pub async fn build_app(pool: sqlx::PgPool, cfg: config::AppConfig) -> Router {
    // Run embedded migrations on boot.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    ledger::seed_content(&pool)
        .await
        .expect("content seeding failed");

    // Promo-RP housekeeping: lapse expired lots into the audit trail and
    // repair any drift between wallets.free_rp and its lot sum. Cheap at boot
    // and it keeps "expires on a schedule" true even if this deploy follows a
    // long downtime that spanned a week boundary.
    match ledger::expire_due_lots(&pool).await {
        Ok(n) if n > 0 => tracing::info!(wallets = n, "expired promo rp lots"),
        Ok(_) => {}
        Err(e) => tracing::error!("expire_due_lots failed: {e}"),
    }
    match ledger::reconcile_free_rp_cache(&pool).await {
        Ok(n) if n > 0 => tracing::warn!(rows = n, "repaired free_rp cache drift"),
        Ok(_) => {}
        Err(e) => tracing::error!("reconcile_free_rp_cache failed: {e}"),
    }

    match race_engine::ensure_default_window(&pool).await {
        Ok(w) => tracing::info!(slug = %w.slug, "default race window ready"),
        Err(e) => tracing::error!("ensure_default_window failed: {e}"),
    }

    race_worker::spawn(
        pool.clone(),
        race_worker::WorkerConfig {
            mainnet_rpc: cfg.mainnet_rpc.clone(),
            er_rpc: cfg.er_rpc.clone(),
            er_ws: cfg.er_ws.clone(),
            max_race_secs: cfg.max_race_secs,
            authority_secret_b58: cfg.authority_secret_b58.clone(),
        },
    );

    let state = AppState {
        db: pool,
        cfg: Arc::new(cfg),
    };

    Router::new()
        .route("/healthz", get(handlers::health))
        .route("/v1/rp/{wallet}", get(handlers::get_rp))
        .route("/v1/rp/earn", post(handlers::earn_rp))
        .route("/v1/rp/spend", post(handlers::spend_rp))
        .route("/v1/rp/claim-weekly", post(handlers::claim_weekly))
        .route("/v1/content", get(handlers::list_content))
        .route("/v1/content/read", post(handlers::content_read))
        .route(
            "/v1/projects",
            get(handlers::list_projects).post(handlers::submit_project),
        )
        .route("/v1/projects/import", post(handlers::import_projects))
        .route("/v1/projects/{handle}", get(handlers::get_project))
        .route(
            "/v1/projects/{handle}/support",
            post(handlers::support_project),
        )
        .route(
            "/v1/projects/{handle}/allocations",
            get(handlers::project_allocations),
        )
        .route(
            "/v1/projects/{handle}/click",
            post(handlers::record_project_click),
        )
        .route("/v1/whop/webhook", post(handlers::whop_webhook))
        .route(
            "/v1/whop/membership/{wallet}",
            get(handlers::whop_membership),
        )
        .route("/v1/races/open", post(handlers::races_open))
        .route("/v1/grid", get(handlers::lifetime_grid))
        .route("/v1/races/windows", get(handlers::list_race_windows))
        .route("/v1/races/featured", get(handlers::races_featured))
        .route("/v1/races/calendar", get(handlers::races_calendar))
        .route("/v1/championship", get(handlers::championship_standings))
        .route(
            "/v1/races/windows/{slug}/grid",
            get(handlers::race_window_grid),
        )
        .route(
            "/v1/races/windows/{slug}/snapshot",
            post(handlers::race_window_snapshot),
        )
        .route(
            "/v1/races/windows/{slug}/events",
            get(handlers::race_window_events),
        )
        .route(
            "/v1/races/windows/{slug}/events/{event_id}/narrate",
            post(handlers::narrate_event),
        )
        .route(
            "/v1/races/windows/{slug}/tape",
            get(handlers::race_window_tape),
        )
        .route(
            "/v1/races/{project_pda}/{race_id}/settle",
            post(handlers::races_settle),
        )
        .route("/v1/races/{project_pda}", get(handlers::races_list))
        .route(
            "/v1/projects/{wallet}/public",
            get(handlers::public_ledger_link),
        )
        .route(
            "/v1/onchain/prepare-register",
            post(handlers::prepare_register_project),
        )
        .route(
            "/v1/onchain/prepare-log-paid",
            post(handlers::prepare_log_paid_rp),
        )
        .route(
            "/v1/onchain/prepare-open-race",
            post(handlers::prepare_open_race),
        )
        .route(
            "/v1/onchain/prepare-settle-race",
            post(handlers::prepare_settle_race),
        )
        .route(
            "/v1/races/windows/{slug}/ticks",
            post(ticks::ingest_window_tick),
        )
        .route(
            "/v1/races/sessions/{session_id}/grid",
            get(ticks::session_grid_handler),
        )
        .route("/v1/oauth/supergrok/login", get(oauth_llm::login_handler))
        .route(
            "/v1/oauth/supergrok/callback",
            get(oauth_llm::callback_handler),
        )
        .route("/v1/oauth/supergrok/status", get(oauth_llm::status_handler))
        .route(
            "/v1/narrative/posts/{id}/approve",
            post(publish::approve_handler),
        )
        .route("/v1/narrative/posts/{id}/skip", post(publish::skip_handler))
        .route(
            "/v1/narrative/posts/{id}/mark-published",
            post(publish::mark_published_handler),
        )
        .route("/v1/narrative/queue", get(publish::queue_handler))
        .route("/v1/events/active", get(handlers::events_active))
        .route("/v1/events/afterburner", post(handlers::open_afterburner))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
