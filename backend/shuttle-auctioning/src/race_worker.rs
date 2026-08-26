//! Race settlement worker.
//!
//! Every 15s this pings MagicBlock ER and flags open races that have exceeded
//! `max_race_secs`. v1 does **not** send settlement transactions (no authority
//! key in the worker). The HTTP settle path already exists.

use auctioning_core::{RaceSession, RaceSessionConfig};
use chrono::{DateTime, Utc};
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use std::time::Duration;

/// Built by the caller (from `AppConfig` later). Not loaded here.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub mainnet_rpc: String,
    pub er_rpc: String,
    pub er_ws: String,
    pub max_race_secs: u64,
    pub authority_secret_b58: Option<String>,
}

/// Spawn the 15s poll loop. The handle is abortable; the loop never panics.
pub fn spawn(db: PgPool, cfg: WorkerConfig) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            run_once(&db, &cfg).await;
        }
    })
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

    match overdue_open_races(db, cfg.max_race_secs).await {
        Ok(overdue) => {
            for (project_pda, race_id) in overdue {
                tracing::info!(project_pda, race_id, "race overdue for settle");
            }
        }
        Err(e) => tracing::warn!(error = %e, "overdue open races query failed"),
    }
}

/// Open races whose age is at least `max_race_secs`. Does not settle.
pub async fn overdue_open_races(
    db: &PgPool,
    max_race_secs: u64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows: Vec<(String, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT project_pda, race_id, opened_at FROM races WHERE status = 'open'",
    )
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
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap() + chrono::Duration::seconds(offset_secs)
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

    #[tokio::test]
    async fn spawn_returns_abortable_handle() {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://127.0.0.1:1/auctioning")
            .expect("lazy pool");
        let handle = spawn(db, cfg());
        handle.abort();
        let err = handle.await.expect_err("aborted task");
        assert!(err.is_cancelled());
    }
}
