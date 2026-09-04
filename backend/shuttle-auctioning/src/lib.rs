//! Shuttle backend for auctioning.lol.
//!
//! Private RP ledger (Postgres), free weekly promo RP, Whop webhook dual-write
//! and the race settlement worker. Public paid RP lives on-chain (Anchor
//! program); this service is the source of truth for everything private/free.
//!
//! Security model (see `auth.rs`): wallets prove control by signing a nonce
//! and get a bearer session; operators carry `OPERATOR_TOKEN`; machines carry
//! `INGEST_SECRET`. Outside `APP_ENV=dev` the service refuses to boot with any
//! of those unset (`config::AppConfig::validate`).

pub mod auth;
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
pub mod ratelimit;
pub mod stats;
pub mod ticks;
pub mod whop;

use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use shuttle_runtime::SecretStore;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub use error::{AppError, AppResult};

/// 1 MiB is generous for every JSON body we accept; the 5 000-row catalog
/// import is the largest at roughly 700 KiB.
const MAX_BODY_BYTES: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub cfg: Arc<config::AppConfig>,
    pub limiter: ratelimit::RateLimiter,
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let cfg = config::AppConfig::from_secret_store(&secrets);
    Ok(build_app(pool, cfg).await.into())
}

/// Run migrations and boot-time sweeps, then build the router.
///
/// Panics (refuses to serve) when the configuration is unsafe for its
/// environment or migrations fail — both are deploy errors, not runtime ones.
pub async fn build_app(pool: sqlx::PgPool, cfg: config::AppConfig) -> Router {
    if let Err(problems) = cfg.validate() {
        for p in &problems {
            tracing::error!(problem = %p, "invalid configuration");
        }
        panic!("refusing to start with invalid configuration: {problems:?}");
    }
    tracing::info!(env = cfg.env.as_str(), domain = %cfg.app_domain, "booting auctioning-backend");

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
        limiter: ratelimit::RateLimiter::new(),
    };
    router(state)
}

fn cors_layer(cfg: &config::AppConfig) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-auctioning-ingest"),
            HeaderName::from_static("x-auctioning-operator"),
            HeaderName::from_static("x-auctioning-dev-wallet"),
            HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([HeaderName::from_static("x-request-id")])
        .max_age(Duration::from_secs(600));
    if cfg.allowed_origins.is_empty() {
        // Dev only — validate() forbids this outside dev.
        return base.allow_origin(AllowOrigin::any());
    }
    let origins: Vec<HeaderValue> = cfg
        .allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    base.allow_origin(AllowOrigin::list(origins))
}

/// Pure router construction; `build_app` does the boot work. Tests can call
/// this with a lazy pool to exercise routing without a database.
pub fn router(state: AppState) -> Router {
    let per_min = state.cfg.rate_limit_per_min;
    let lim = state.limiter.clone();
    let strict = |name: &'static str| ratelimit::layer(lim.clone(), name, per_min.clamp(5, 30));
    let normal = |name: &'static str| ratelimit::layer(lim.clone(), name, per_min);

    Router::new()
        .route("/healthz", get(handlers::health))
        .route("/readyz", get(handlers::ready))
        // --- auth ----------------------------------------------------------
        .route(
            "/v1/auth/nonce",
            get(auth::nonce_handler).layer(strict("auth_nonce")),
        )
        .route(
            "/v1/auth/verify",
            post(auth::verify_handler).layer(strict("auth_verify")),
        )
        .route("/v1/auth/me", get(auth::me_handler))
        .route("/v1/auth/logout", post(auth::logout_handler))
        // --- rp ------------------------------------------------------------
        .route("/v1/rp/{wallet}", get(handlers::get_rp))
        .route("/v1/rp/earn", post(handlers::earn_rp))
        .route("/v1/rp/spend", post(handlers::spend_rp).layer(normal("spend")))
        .route(
            "/v1/rp/claim-weekly",
            post(handlers::claim_weekly).layer(normal("claim")),
        )
        .route("/v1/wallets/me/history", get(stats::wallet_history_handler))
        // --- content + catalog --------------------------------------------
        .route("/v1/content", get(handlers::list_content))
        .route("/v1/content/read", post(handlers::content_read))
        // submit_project checks the "submit" bucket itself: the same path
        // serves the public catalog GET, which must stay unthrottled.
        .route(
            "/v1/projects",
            get(handlers::list_projects).post(handlers::submit_project),
        )
        .route("/v1/projects/import", post(handlers::import_projects))
        .route("/v1/projects/{handle}", get(handlers::get_project))
        .route(
            "/v1/projects/{handle}/stats",
            get(stats::project_stats_handler),
        )
        .route(
            "/v1/projects/{handle}/support",
            post(handlers::support_project).layer(normal("support")),
        )
        .route(
            "/v1/projects/{handle}/allocations",
            get(handlers::project_allocations),
        )
        .route(
            "/v1/projects/{handle}/click",
            post(handlers::record_project_click).layer(normal("click")),
        )
        // --- whop ----------------------------------------------------------
        .route("/v1/whop/webhook", post(handlers::whop_webhook))
        .route(
            "/v1/whop/membership/{wallet}",
            get(handlers::whop_membership),
        )
        // --- on-chain race mirror -----------------------------------------
        .route("/v1/races/open", post(handlers::races_open))
        .route(
            "/v1/races/{project_pda}/{race_id}/settle",
            post(handlers::races_settle),
        )
        .route("/v1/races/{project_pda}", get(handlers::races_list))
        .route(
            "/v1/projects/{wallet}/public",
            get(handlers::public_ledger_link),
        )
        // --- windowed race engine -----------------------------------------
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
            "/v1/races/windows/{slug}/ticks",
            post(ticks::ingest_window_tick),
        )
        .route(
            "/v1/races/sessions/{session_id}/grid",
            get(ticks::session_grid_handler),
        )
        // --- on-chain prep (unsigned txs for Phantom) ---------------------
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
        // --- narrative / oauth (operator) ---------------------------------
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
        // --- events + stats -----------------------------------------------
        .route("/v1/events/active", get(handlers::events_active))
        .route("/v1/events/afterburner", post(handlers::open_afterburner))
        .route("/v1/stats/overview", get(stats::overview_handler))
        .route("/v1/stats/revenue", get(stats::revenue_handler))
        .layer(cors_layer(&state.cfg))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .with_state(state)
}
