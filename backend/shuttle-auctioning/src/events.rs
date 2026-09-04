//! Operator event cards (Afterburner, Night Grid, …) and purchase split.
//!
//! Advertised price is always $1 = 1 paid RP. Cards add extra *pace* as
//! `event_multiplier` lots. Do not write bonus into `paid_rp`.

use crate::ledger::{self, next_week_start, RpSource, WalletLedger};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 1.0x in basis points.
pub const BPS_ONE: i64 = 10_000;
/// Afterburner: 1.5x race pace.
pub const AFTERBURNER_BPS: i64 = 15_000;
pub const AFTERBURNER_SLUG: &str = "afterburner";
pub const AFTERBURNER_NAME: &str = "Afterburner";

/// Split a purchase: paid stays 1:1; bonus is extra pace.
/// `bonus = floor(paid * (bps - 10000) / 10000)`. bps <= 10000 → bonus 0.
pub fn split_purchase(paid: i64, multiplier_bps: i64) -> (i64, i64) {
    if paid <= 0 {
        return (0, 0);
    }
    if multiplier_bps <= BPS_ONE {
        return (paid, 0);
    }
    let bonus = paid.saturating_mul(multiplier_bps - BPS_ONE) / BPS_ONE;
    (paid, bonus)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OperatorEvent {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub multiplier_bps: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub tag: Option<String>,
    pub window_id: Option<Uuid>,
}

/// Highest-bps live card overlapping `now` (no stacking).
pub async fn active_card(
    db: &PgPool,
    now: DateTime<Utc>,
) -> Result<Option<OperatorEvent>, sqlx::Error> {
    sqlx::query_as::<_, OperatorEvent>(
        r#"
        SELECT id, slug, name, multiplier_bps, starts_at, ends_at, tag, window_id
        FROM operator_events
        WHERE starts_at <= $1 AND ends_at > $1
        ORDER BY multiplier_bps DESC, starts_at DESC
        LIMIT 1
        "#,
    )
    .bind(now)
    .fetch_optional(db)
    .await
}

#[derive(Debug, Clone)]
pub struct CardCredit {
    pub ledger: WalletLedger,
    /// True when `tx_id` had already been credited; nothing was written.
    pub duplicate: bool,
    pub bonus_rp: i64,
}

/// Paid 1:1, then Afterburner (or whichever card is live) as EventMultiplier.
///
/// Idempotent on `tx_id`: concurrent deliveries of the same payment serialise
/// on a transaction-scoped advisory lock, so the paid row (unique tx_id) and
/// the bonus lot are written at most once.
pub async fn credit_paid_with_card(
    db: &PgPool,
    wallet: &str,
    paid_rp: i64,
    reason: &str,
    tx_id: Option<&str>,
) -> Result<CardCredit, sqlx::Error> {
    let tx_id = tx_id.filter(|s| !s.is_empty());
    let mut guard = db.begin().await?;
    if let Some(id) = tx_id {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(id)
            .execute(&mut *guard)
            .await?;
        let seen: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM ledger_events WHERE tx_id = $1 LIMIT 1")
                .bind(id)
                .fetch_optional(&mut *guard)
                .await?;
        if seen.is_some() {
            let ledger = ledger::ensure_wallet(db, wallet).await?;
            guard.rollback().await?;
            return Ok(CardCredit {
                ledger,
                duplicate: true,
                bonus_rp: 0,
            });
        }
    }

    let mut ledger = ledger::credit_paid(db, wallet, paid_rp, reason, tx_id).await?;
    let mut bonus_rp = 0;
    if let Some(card) = active_card(db, Utc::now()).await? {
        let (_, bonus) = split_purchase(paid_rp, card.multiplier_bps);
        if bonus > 0 {
            let expires = card.ends_at.max(next_week_start(Utc::now()));
            ledger = ledger::grant_free_lot(
                db,
                wallet,
                bonus,
                RpSource::EventMultiplier,
                &format!("event:{}", card.slug),
                expires,
            )
            .await?;
            bonus_rp = bonus;
        }
    }
    guard.commit().await?;
    Ok(CardCredit {
        ledger,
        duplicate: false,
        bonus_rp,
    })
}

/// Open Afterburner for `duration` from now. Idempotent on slug+open overlap:
/// if an Afterburner row already covers now, return it.
pub async fn open_afterburner(
    db: &PgPool,
    duration: chrono::Duration,
) -> Result<OperatorEvent, sqlx::Error> {
    let now = Utc::now();
    if let Some(existing) = active_card(db, now).await? {
        if existing.slug == AFTERBURNER_SLUG {
            return Ok(existing);
        }
    }
    let ends = now + duration;
    sqlx::query(
        r#"
        INSERT INTO operator_events (slug, name, multiplier_bps, starts_at, ends_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(AFTERBURNER_SLUG)
    .bind(AFTERBURNER_NAME)
    .bind(AFTERBURNER_BPS)
    .bind(now)
    .bind(ends)
    .execute(db)
    .await?;

    active_card(db, now)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn afterburner_splits_100_into_100_paid_50_pace() {
        assert_eq!(split_purchase(100, AFTERBURNER_BPS), (100, 50));
    }

    #[test]
    fn one_x_credits_no_bonus() {
        assert_eq!(split_purchase(100, BPS_ONE), (100, 0));
        assert_eq!(split_purchase(100, 9_999), (100, 0));
    }

    #[test]
    fn night_grid_2x_is_100_plus_100() {
        assert_eq!(split_purchase(100, 20_000), (100, 100));
    }

    #[test]
    fn pit_lane_125_on_100() {
        assert_eq!(split_purchase(100, 12_500), (100, 25));
    }

    #[test]
    fn zero_or_negative_paid_is_zero() {
        assert_eq!(split_purchase(0, AFTERBURNER_BPS), (0, 0));
        assert_eq!(split_purchase(-5, AFTERBURNER_BPS), (0, 0));
    }
}
