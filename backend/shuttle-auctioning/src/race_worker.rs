//! Race settlement worker.
//!
//! Every 15s this pings MagicBlock ER and flags open races that have exceeded
//! `max_race_secs`. When `er_ws` is set, a long-lived accountSubscribe loop
//! ingests ER ticks. Delegate instructions are built when an authority key is
//! present but never sent (no SOL). The HTTP settle path already exists.

use auctioning_core::{RaceSession, RaceSessionConfig, TickEnvelope};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use sqlx::PgPool;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// Built by the caller (from `AppConfig` later). Not loaded here.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub mainnet_rpc: String,
    pub er_rpc: String,
    pub er_ws: String,
    pub max_race_secs: u64,
    pub authority_secret_b58: Option<String>,
}

/// Non-final snapshots older than this are deleted (final per window is kept).
const SNAPSHOT_RETENTION_DAYS: i64 = 90;

fn should_subscribe(er_ws: &str) -> bool {
    !er_ws.is_empty()
}

/// Spawn the 15s poll loop. The handle is abortable; the loop never panics.
/// A second task runs the ER websocket ingest loop when `er_ws` is set.
pub fn spawn(db: PgPool, cfg: WorkerConfig) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn({
        let db = db.clone();
        let cfg = cfg.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));
            loop {
                interval.tick().await;
                run_once(&db, &cfg).await;
            }
        }
    });
    if should_subscribe(&cfg.er_ws) {
        tokio::spawn(ws_loop(db, cfg));
    }
    handle
}

async fn run_once(db: &PgPool, cfg: &WorkerConfig) {
    let session_cfg = RaceSessionConfig {
        mainnet_rpc: cfg.mainnet_rpc.clone(),
        er_rpc: cfg.er_rpc.clone(),
        er_ws: cfg.er_ws.clone(),
        project_pda: Pubkey::default().to_string(),
        authority_secret_b58: cfg.authority_secret_b58.clone(),
        max_race_secs: cfg.max_race_secs,
    };

    match RaceSession::new(session_cfg, Pubkey::default(), 0) {
        Ok(session) => match session.ping_er().await {
            Ok(slot) => tracing::info!(slot, "er ping ok"),
            Err(e) => tracing::warn!(error = %e, "er ping failed"),
        },
        Err(e) => tracing::warn!(error = %e, "race session init failed"),
    }

    if should_subscribe(&cfg.er_ws) {
        tracing::info!("er ws subscribe armed");
    }

    match overdue_open_races(db, cfg.max_race_secs).await {
        Ok(overdue) => {
            for (project_pda, race_id) in overdue {
                tracing::info!(project_pda, race_id, "race overdue for settle");
            }
        }
        Err(e) => tracing::warn!(error = %e, "overdue open races query failed"),
    }

    tick_calendar(db).await;

    if let Some(secret) = cfg.authority_secret_b58.as_deref() {
        match auctioning_core::load_authority(secret) {
            Ok(auth) => match open_race_rows(db).await {
                Ok(rows) => {
                    for (project_pda, _race_id) in rows {
                        match auctioning_core::parse_pubkey(&project_pda) {
                            Ok(account) => {
                                let _ix = auctioning_core::build_delegate_instruction(
                                    &auth.pubkey(),
                                    &account,
                                );
                                tracing::info!(project_pda, "delegate ix built");
                            }
                            Err(e) => tracing::warn!(
                                error = %e,
                                project_pda,
                                "delegate pubkey parse failed"
                            ),
                        }
                    }
                }
                Err(e) => tracing::warn!(error = %e, "open races query failed"),
            },
            Err(e) => tracing::warn!(error = %e, "authority load failed"),
        }
    }
}

async fn ws_loop(db: PgPool, cfg: WorkerConfig) {
    loop {
        match tokio_tungstenite::connect_async(cfg.er_ws.as_str()).await {
            Ok((mut ws, _)) => {
                let mut id = 1u64;
                let mut pdas: Vec<String> = match overdue_open_races(&db, cfg.max_race_secs).await {
                    Ok(rows) => rows.into_iter().map(|(pda, _)| pda).collect(),
                    Err(e) => {
                        tracing::warn!(error = %e, "overdue open races query failed");
                        Vec::new()
                    }
                };
                pdas.push(Pubkey::default().to_string());
                let mut send_ok = true;
                for pda in &pdas {
                    let req = auctioning_core::account_subscribe_request(id, pda);
                    id += 1;
                    if let Err(e) = ws.send(Message::Text(req.to_string().into())).await {
                        tracing::warn!(
                            error = %e,
                            project_pda = %pda,
                            "accountSubscribe send failed"
                        );
                        send_ok = false;
                        break;
                    }
                }
                if send_ok {
                    loop {
                        match ws.next().await {
                            Some(Ok(Message::Text(text))) => {
                                if let Some(env) =
                                    auctioning_core::parse_tick_notification(text.as_str())
                                {
                                    ingest_parsed(&db, env).await;
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(e)) => {
                                tracing::warn!(error = %e, "er ws read failed");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => tracing::warn!(error = %e, "er ws connect failed"),
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn tick_calendar(db: &PgPool) {
    if let Err(e) = crate::race_engine::archive_expired_windows(db, Utc::now()).await {
        tracing::warn!(error = %e, "archive_expired_windows failed");
    }
    if let Err(e) = crate::race_engine::backfill_archived_finals(db).await {
        tracing::warn!(error = %e, "backfill_archived_finals failed");
    }
    match crate::race_engine::list_windows(db, 50).await {
        Ok(windows) => {
            for w in windows.iter().filter(|w| w.status == "live") {
                match crate::race_engine::persist_snapshot_if_events(db, w).await {
                    Ok((_, events)) if !events.is_empty() => {
                        tracing::info!(slug = %w.slug, n = events.len(), "race events persisted");
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(
                        error = %e,
                        slug = %w.slug,
                        "persist_snapshot_if_events failed"
                    ),
                }
            }
        }
        Err(e) => tracing::warn!(error = %e, "list_windows failed"),
    }
    housekeeping(db).await;
    match crate::narrative::mint_unposted_events(db, 20).await {
        Ok(n) if n > 0 => tracing::info!(events = n, "narrative drafts minted"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "mint_unposted_events failed"),
    }
}

/// Hourly retention: expired auth rows and old non-final snapshots.
async fn housekeeping(db: &PgPool) {
    use std::sync::atomic::{AtomicI64, Ordering};
    static LAST: AtomicI64 = AtomicI64::new(0);
    let now = Utc::now().timestamp();
    if now - LAST.load(Ordering::Relaxed) < 3600 {
        return;
    }
    LAST.store(now, Ordering::Relaxed);
    match crate::auth::prune_expired(db).await {
        Ok(n) if n > 0 => tracing::info!(rows = n, "pruned expired auth rows"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "auth prune failed"),
    }
    match crate::race_engine::prune_snapshots(db, SNAPSHOT_RETENTION_DAYS).await {
        Ok(n) if n > 0 => tracing::info!(rows = n, "pruned old rank snapshots"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "snapshot prune failed"),
    }
}

async fn ingest_parsed(db: &PgPool, env: TickEnvelope) {
    match crate::race_engine::ensure_default_window(db).await {
        Ok(window) => {
            if let Err(e) = crate::ticks::ingest_tick(db, &window, &env).await {
                tracing::warn!(error = %e, "tick ingest failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "ensure_default_window failed"),
    }
}

async fn open_race_rows(db: &PgPool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT project_pda, race_id FROM races WHERE status = 'open'")
        .fetch_all(db)
        .await
}

/// Open races whose age is at least `max_race_secs`. Does not settle.
pub async fn overdue_open_races(
    db: &PgPool,
    max_race_secs: u64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows: Vec<(String, i64, DateTime<Utc>)> =
        sqlx::query_as("SELECT project_pda, race_id, opened_at FROM races WHERE status = 'open'")
            .fetch_all(db)
            .await?;
    let now = Utc::now();
    Ok(rows
        .into_iter()
        .filter(|(_, _, opened_at)| is_overdue(*opened_at, now, max_race_secs))
        .map(|(project_pda, race_id, _)| (project_pda, race_id))
        .collect())
}

fn is_overdue(opened_at: DateTime<Utc>, now: DateTime<Utc>, max_secs: u64) -> bool {
    now.signed_duration_since(opened_at).num_seconds() >= max_secs as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(offset_secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
            + chrono::Duration::seconds(offset_secs)
    }

    fn cfg() -> WorkerConfig {
        WorkerConfig {
            mainnet_rpc: "http://127.0.0.1:1".into(),
            er_rpc: "http://127.0.0.1:1".into(),
            er_ws: "ws://127.0.0.1:1".into(),
            max_race_secs: 300,
            authority_secret_b58: None,
        }
    }

    #[test]
    fn is_overdue_false_before_max() {
        let opened = ts(0);
        assert!(!is_overdue(opened, ts(299), 300));
    }

    #[test]
    fn is_overdue_true_at_exact_max() {
        let opened = ts(0);
        assert!(is_overdue(opened, ts(300), 300));
    }

    #[test]
    fn is_overdue_true_after_max() {
        let opened = ts(0);
        assert!(is_overdue(opened, ts(301), 300));
    }

    #[test]
    fn is_overdue_false_when_opened_in_the_future() {
        assert!(!is_overdue(ts(10), ts(0), 300));
    }

    #[test]
    fn is_overdue_zero_max_is_due_at_or_after_open() {
        let opened = ts(0);
        assert!(is_overdue(opened, opened, 0));
        assert!(!is_overdue(opened, ts(-1), 0));
    }

    #[test]
    fn empty_er_ws_does_not_subscribe() {
        assert!(!should_subscribe(""));
        assert!(should_subscribe("wss://er.example/ws"));
    }

    #[tokio::test]
    async fn spawn_returns_abortable_handle() {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:1/auctioning")
            .expect("lazy pool");
        let mut empty = cfg();
        empty.er_ws.clear();
        let handle = spawn(db, empty);
        handle.abort();
        let err = handle.await.expect_err("aborted task");
        assert!(err.is_cancelled());
    }
}
