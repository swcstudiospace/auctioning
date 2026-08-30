mod ledger {
    // Private RP ledger. Postgres is the source of truth for free/promo RP and
    // per-user private accounting; paid RP totals mirror the on-chain ledger.
    //
    // RP provenance model (priority #1):
    // - Every inflow carries a typed `source`: paid | free_weekly | bonus | event_multiplier.
    // - Free/promotional RP lives as FIFO expiry lots (`free_rp_lots`). The
    //   `wallets.free_rp` column is a derived cache of active lots and is kept
    //   honest by `expire_due_lots` + `reconcile_free_rp_cache` (run at boot).
    // - Spends drain the earliest-expiring lots first; project allocations
    //   reference the exact lot they drained (see catalog.rs) for end-to-end
    //   provenance.

use chrono::Datelike;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Maximum plausible base58 Solana address length (44). Guard against junk.
pub const MAX_WALLET_LEN: usize = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RpSource {
    /// Mirrors an on-chain receipt; cashable provenance.
    Paid,
    /// The weekly community stipend.
    FreeWeekly,
    /// Earned via content reads / narrative engagement.
    Bonus,
    /// Admin-declared multiplier windows (Double RP Hour, Night Race, ...).
    EventMultiplier,
}

impl RpSource {
    pub fn as_str(self) -> &'static str {
        match self {
            RpSource::Paid => "paid",
            RpSource::FreeWeekly => "free_weekly",
            RpSource::Bonus => "bonus",
            RpSource::EventMultiplier => "event_multiplier",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "paid" => Some(RpSource::Paid),
            "free_weekly" => Some(RpSource::FreeWeekly),
            "bonus" => Some(RpSource::Bonus),
            "event_multiplier" => Some(RpSource::EventMultiplier),
            _ => None,
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct WalletLedger {
    pub wallet: String,
    pub paid_rp: i64,
    pub free_rp: i64,
    pub spent_rp: i64,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct LedgerEvent {
    pub id: Uuid,
    pub wallet: String,
    pub kind: String,
    /// Typed inflow source; NULL for outflows (spend/expire).
    pub source: Option<String>,
    pub amount: i64,
    pub reason: String,
    pub tx_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct RaceRow {
    pub id: Uuid,
    pub project_pda: String,
    pub race_id: i64,
    pub er_session: Option<String>,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
    pub settle_tx: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("invalid solana wallet address")]
    Invalid,
}

impl From<WalletError> for sqlx::Error {
    fn from(e: WalletError) -> Self {
        sqlx::Error::Configuration(e.to_string().into())
    }
}

/// Loose base58 check: right length, right alphabet. Full curve validation is
/// the client's job; this only stops obvious garbage from entering the DB.
pub fn valid_wallet(w: &str) -> bool {
    !(w.is_empty() || w.len() > MAX_WALLET_LEN)
        && w.bytes()
            .all(|b| matches!(b, b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z'))
}

#[derive(Debug, sqlx::FromRow)]
pub struct WeeklyClaim {
    pub id: Uuid,
    pub wallet: String,
    pub amount: i64,
    /// Start of the ISO week this claim covers (unique per wallet).
    pub week_start: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// ISO week start (Monday 00:00 UTC) containing `now`.
pub fn current_week_start() -> DateTime<Utc> {
    week_start_of(Utc::now())
}

pub fn week_start_of(now: DateTime<Utc>) -> DateTime<Utc> {
    let days_since_monday = now.date_naive().weekday().num_days_from_monday() as i64;
    let d = chrono::Duration::days(days_since_monday);
    let midnight = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight")
        .and_utc();
    midnight - d
}

/// Next Monday 00:00 UTC strictly after `now` — the default promo expiry
/// boundary ("stipends reset weekly").
pub fn next_week_start(now: DateTime<Utc>) -> DateTime<Utc> {
    week_start_of(now) + chrono::Duration::weeks(1)
}

/// Fetch or create the ledger row for a wallet.
pub async fn ensure_wallet(db: &PgPool, wallet: &str) -> Result<WalletLedger, sqlx::Error> {
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT (wallet) DO NOTHING")
        .bind(wallet)
        .execute(db)
        .await?;

    sqlx::query_as::<_, WalletLedger>(
        "SELECT wallet, paid_rp, free_rp, spent_rp FROM wallets WHERE wallet = $1;",
    )
    .bind(wallet)
    .fetch_one(db)
    .await
}

/// Grant promotional RP as a FIFO expiry lot. The wallet cache and the
/// append-only ledger are updated in the same transaction.
pub async fn grant_free_lot(
    db: &PgPool,
    wallet: &str,
    amount: i64,
    source: RpSource,
    reason: &str,
    expires_at: DateTime<Utc>,
) -> Result<WalletLedger, sqlx::Error> {
    let mut tx = db.begin().await?;
    let updated = grant_free_lot_tx(&mut tx, wallet, amount, source, reason, expires_at).await?;
    tx.commit().await?;
    Ok(updated)
}

/// Transaction-scoped variant shared by weekly claims and content rewards.
pub(crate) async fn grant_free_lot_tx(
    tx: &mut Transaction<'static, Postgres>,
    wallet: &str,
    amount: i64,
    source: RpSource,
    reason: &str,
    expires_at: DateTime<Utc>,
) -> Result<WalletLedger, sqlx::Error> {
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    if amount <= 0 {
        return Err(sqlx::Error::Configuration(
            "free RP amount must be positive".into(),
        ));
    }
    if !matches!(
        source,
        RpSource::FreeWeekly | RpSource::Bonus | RpSource::EventMultiplier
    ) {
        return Err(sqlx::Error::Configuration(
            "only promotional sources may mint free lots".into(),
        ));
    }

    sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(wallet)
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO free_rp_lots (wallet, amount, remaining, source, reason, expires_at)
        VALUES ($1, $2, $2, $3, $4, $5)
        "#,
    )
    .bind(wallet)
    .bind(amount)
    .bind(source.as_str())
    .bind(reason)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    sqlx::query("UPDATE wallets SET free_rp = free_rp + $1, updated_at = now() WHERE wallet = $2")
        .bind(amount)
        .bind(wallet)
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_events (wallet, kind, source, amount, reason)
        VALUES ($1, 'free', $2, $3, $4)
        "#,
    )
    .bind(wallet)
    .bind(source.as_str())
    .bind(amount)
    .bind(reason)
    .execute(&mut **tx)
    .await?;

    let updated = sqlx::query_as::<_, WalletLedger>(
        "SELECT wallet, paid_rp, free_rp, spent_rp FROM wallets WHERE wallet = $1",
    )
    .bind(wallet)
    .fetch_one(&mut **tx)
    .await?;
    Ok(updated)
}

/// Credit PAID RP (fiat/on-chain mirror). Spend provenance only — not a
/// cash-out token. No expiry; never touched by the lot machinery.
///
/// When `tx_id` is set (Whop payment id / chain signature) the insert is
/// idempotent: a second call with the same id returns the existing wallet.
pub async fn credit_paid(
    db: &PgPool,
    wallet: &str,
    amount: i64,
    reason: &str,
    tx_id: Option<&str>,
) -> Result<WalletLedger, sqlx::Error> {
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    if amount <= 0 {
        return Err(sqlx::Error::Configuration(
            "paid RP amount must be positive".into(),
        ));
    }
    let tx_id = tx_id.filter(|s| !s.is_empty());
    if let Some(id) = tx_id {
        let already =
            sqlx::query_scalar::<_, i32>("SELECT 1 FROM ledger_events WHERE tx_id = $1 LIMIT 1")
                .bind(id)
                .fetch_optional(db)
                .await?;
        if already.is_some() {
            return ensure_wallet(db, wallet).await;
        }
    }
    let mut tx = db.begin().await?;

    sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(wallet)
        .execute(&mut *tx)
        .await?;

    let updated = sqlx::query_as::<_, WalletLedger>(
        "UPDATE wallets SET paid_rp = paid_rp + $1, updated_at = now() \
         WHERE wallet = $2 RETURNING wallet, paid_rp, free_rp, spent_rp",
    )
    .bind(amount)
    .bind(wallet)
    .fetch_one(&mut *tx)
    .await?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO ledger_events (wallet, kind, source, amount, reason, tx_id)
        VALUES ($1, 'paid', 'paid', $2, $3, $4)
        "#,
    )
    .bind(wallet)
    .bind(amount)
    .bind(reason)
    .bind(tx_id)
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(ref db_err)) = inserted {
        if db_err.constraint() == Some("uq_ledger_events_tx_id") {
            tx.rollback().await?;
            return ensure_wallet(db, wallet).await;
        }
    }
    inserted?;

    tx.commit().await?;
    Ok(updated)
}

/// One lot drained by a spend, with its provenance.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct LotDrain {
    pub lot_id: Uuid,
    pub amount: i64,
    pub source: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SpendBreakdown {
    pub total: i64,
    pub from_free: i64,
    pub from_paid: i64,
    pub lots: Vec<LotDrain>,
    #[serde(flatten)]
    pub ledger: WalletLedger,
}

/// Spend free-then-paid RP from a wallet. Atomic: refuses to go negative and
/// records exactly how much came out of each lot/bucket. Free RP leaves first
/// (earliest expiry first) so promotional balance drains before cashable paid
/// balance.
pub async fn spend(
    db: &PgPool,
    wallet: &str,
    amount: i64,
    reason: &str,
) -> Result<Option<SpendBreakdown>, sqlx::Error> {
    if amount <= 0 {
        return Err(sqlx::Error::Configuration(
            "spend amount must be positive".into(),
        ));
    }
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    let mut tx = db.begin().await?;
    let breakdown = spend_inner(&mut tx, wallet, amount, reason).await?;
    match breakdown {
        Some(_) => tx.commit().await?,
        None => tx.rollback().await?,
    }
    Ok(breakdown)
}

/// Core FIFO spend inside an existing transaction (shared with project
/// allocations). Locks the wallet row, then drains lots oldest-expiry-first.
/// Caller owns commit/rollback.
pub(crate) async fn spend_inner(
    tx: &mut Transaction<'static, Postgres>,
    wallet: &str,
    amount: i64,
    reason: &str,
) -> Result<Option<SpendBreakdown>, sqlx::Error> {
    debug_assert!(amount > 0);

    let row = sqlx::query_as::<_, WalletLedger>(
        "SELECT wallet, paid_rp, free_rp, spent_rp FROM wallets WHERE wallet = $1 FOR UPDATE",
    )
    .bind(wallet)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(l) = row else {
        return Ok(None);
    };
    let available = l.free_rp.saturating_add(l.paid_rp);
    if available < amount {
        // Insufficient funds: no partial spend.
        return Ok(None);
    }

    let from_free = amount.min(l.free_rp);
    let mut from_paid = amount - from_free;

    // Drain lots FIFO by expiry.
    let mut lots: Vec<LotDrain> = Vec::new();
    if from_free > 0 {
        let active = sqlx::query_as::<_, (Uuid, i64, String)>(
            "SELECT id, remaining, source FROM free_rp_lots \
             WHERE wallet = $1 AND remaining > 0 \
             ORDER BY expires_at ASC, granted_at ASC \
             FOR UPDATE",
        )
        .bind(wallet)
        .fetch_all(&mut **tx)
        .await?;

        let mut left = from_free;
        for (lot_id, remaining, source) in active {
            if left == 0 {
                break;
            }
            let take = left.min(remaining);
            sqlx::query("UPDATE free_rp_lots SET remaining = remaining - $1 WHERE id = $2")
                .bind(take)
                .bind(lot_id)
                .execute(&mut **tx)
                .await?;
            lots.push(LotDrain {
                lot_id,
                amount: take,
                source: source.clone(),
            });
            left -= take;
        }
        debug_assert_eq!(left, 0, "cache said {from_free} free was available");

        sqlx::query(
            "UPDATE wallets SET free_rp = free_rp - $1, updated_at = now() WHERE wallet = $2",
        )
        .bind(from_free)
        .bind(wallet)
        .execute(&mut **tx)
        .await?;
    }

    if from_paid > 0 {
        sqlx::query(
            "UPDATE wallets SET paid_rp = paid_rp - $1, updated_at = now() WHERE wallet = $2",
        )
        .bind(from_paid)
        .bind(wallet)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query(
        "UPDATE wallets SET spent_rp = spent_rp + $1, updated_at = now() WHERE wallet = $2",
    )
    .bind(amount)
    .bind(wallet)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO ledger_events (wallet, kind, amount, reason) VALUES ($1, 'spend', $2, $3)",
    )
    .bind(wallet)
    .bind(amount)
    .bind(reason)
    .execute(&mut **tx)
    .await?;

    let updated = sqlx::query_as::<_, WalletLedger>(
        "SELECT wallet, paid_rp, free_rp, spent_rp FROM wallets WHERE wallet = $1",
    )
    .bind(wallet)
    .fetch_one(&mut **tx)
    .await?;

    let _ = &mut from_paid; // consumed above; keep the borrow checker calm
    Ok(Some(SpendBreakdown {
        total: amount,
        from_free,
        from_paid,
        lots,
        ledger: updated,
    }))
}

/// Claim the free weekly RP. Idempotent per (wallet, week) via unique index.
/// The stipend lands as a lot expiring at `expires_at` (next Monday 00:00 UTC
/// by convention). Returns Some(amount) if freshly claimed, None if already
/// claimed this week.
pub async fn claim_weekly(
    db: &PgPool,
    wallet: &str,
    amount: i64,
    expires_at: DateTime<Utc>,
) -> Result<Option<i64>, sqlx::Error> {
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    let mut tx = db.begin().await?;

    sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(wallet)
        .execute(&mut *tx)
        .await?;

    let week_start = current_week_start();
    let inserted = sqlx::query(
        r#"
        INSERT INTO weekly_claims (wallet, amount, week_start)
        VALUES ($1, $2, $3)
        ON CONFLICT (wallet, week_start) DO NOTHING
        "#,
    )
    .bind(wallet)
    .bind(amount)
    .bind(week_start)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if inserted == 0 {
        tx.rollback().await?;
        return Ok(None); // already claimed this week
    }

    grant_free_lot_tx(
        &mut tx,
        wallet,
        amount,
        RpSource::FreeWeekly,
        "weekly_promo",
        expires_at,
    )
    .await?;

    tx.commit().await?;
    Ok(Some(amount))
}

/// Append-only history for a wallet (newest first).
pub async fn events_for(
    db: &PgPool,
    wallet: &str,
    limit: i64,
) -> Result<Vec<LedgerEvent>, sqlx::Error> {
    sqlx::query_as::<_, LedgerEvent>(
        r#"
        SELECT id, wallet, kind, source, amount, reason, tx_id, created_at
        FROM ledger_events WHERE wallet = $1
        ORDER BY created_at DESC LIMIT $2
        "#,
    )
    .bind(wallet)
    .bind(limit.clamp(1, 200))
    .fetch_all(db)
    .await
}

/// Expire every lapsed lot and keep the wallet cache + audit trail honest.
/// Returns the number of wallets whose balance moved. Safe to run
/// concurrently-ish (single UPDATE statements; boot-time cadence).
pub async fn expire_due_lots(db: &PgPool) -> Result<u64, sqlx::Error> {
    // 1) Aggregate lapsed amounts per wallet and decrement caches where the
    //    cache still covers the lapse (guards against pre-existing drift).
    let decremented = sqlx::query(
        r#"
        WITH lapsed AS (
            SELECT id, wallet, remaining FROM free_rp_lots
            WHERE remaining > 0 AND expires_at <= now()
        ), per_wallet AS (
            SELECT wallet, SUM(remaining) AS total FROM lapsed GROUP BY wallet
        ), updated AS (
            UPDATE wallets w
            SET free_rp = w.free_rp - pw.total, updated_at = now()
            FROM per_wallet pw
            WHERE w.wallet = pw.wallet AND w.free_rp >= pw.total
            RETURNING w.wallet
        )
        INSERT INTO ledger_events (wallet, kind, amount, reason)
        SELECT u.wallet, 'expire', pw.total, 'promotional rp expired'
        FROM per_wallet pw JOIN updated u ON u.wallet = pw.wallet
        "#,
    )
    .execute(db)
    .await?
    .rows_affected();

    // 2) Zero out the lapsed lots regardless (idempotent).
    sqlx::query(
        "UPDATE free_rp_lots SET remaining = 0 WHERE remaining > 0 AND expires_at <= now()",
    )
    .execute(db)
    .await?;

    Ok(decremented)
}

/// Repair drift between the `wallets.free_rp` cache and the true sum of
/// active lots. Returns how many wallet rows were corrected. Runs at boot;
/// cheap enough to call after any bulk operation.
pub async fn reconcile_free_rp_cache(db: &PgPool) -> Result<usize, sqlx::Error> {
    // Wallets WITH lots: cache must equal the sum.
    let a = sqlx::query(
        r#"
        UPDATE wallets w
        SET free_rp = s.total, updated_at = now()
        FROM (
            SELECT wallet, SUM(remaining) AS total
            FROM free_rp_lots WHERE remaining > 0
            GROUP BY wallet
        ) s
        WHERE s.wallet = w.wallet AND w.free_rp <> s.total
        "#,
    )
    .execute(db)
    .await?
    .rows_affected();

    // Wallets with cached balance but NO active lots: cache resets to zero.
    let b = sqlx::query(
        r#"
        UPDATE wallets w
        SET free_rp = 0, updated_at = now()
        WHERE w.free_rp <> 0
          AND NOT EXISTS (
            SELECT 1 FROM free_rp_lots l
            WHERE l.wallet = w.wallet AND l.remaining > 0
          )
        "#,
    )
    .execute(db)
    .await?
    .rows_affected();

    Ok((a + b) as usize)
}

// ---------------------------------------------------------------------------
// Narrative / content engine stubs
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ContentItem {
    pub slug: String,
    pub title: String,
    pub body_md: String,
    pub rp_reward: i64,
    /// Optional ISO date (YYYY-MM-DD) when this becomes claimable.
    pub available_from: Option<chrono::NaiveDate>,
}

/// Seed narrative content if empty. Idempotent; runs at boot next to migrations
/// so every deploy carries its story content with it.
pub async fn seed_content(db: &PgPool) -> Result<(), sqlx::Error> {
    let items: &[(&str, &str, &str, i64, Option<&str>)] = &[
        (
            "the-first-auction",
            "The First Auction",
            "Every ledger starts somewhere. Before auctioning.lol there was a\
             spreadsheet, a promise, and a coffee shop in Brisbane that wanted\
             its regulars to own a piece of the noise.",
            25,
            None,
        ),
        (
            "why-free-rp-cant-be-sold",
            "Why Your Weekly RP Can't Be Sold",
            "Free RP is a thank-you, not a currency. It lives off-chain, it\
             never touches a market, and it can never be cashed out — that's\
             what keeps the game fun instead of financial.",
            15,
            None,
        ),
        (
            "races-explained",
            "Races, Explained",
            "Sixteen entrants. Fifty-millisecond ticks. One immutable ranking,\
             settled to mainnet when the dust clears. Here's how ephemeral\
             rollups make a leaderboard feel like a sport.",
            15,
            None,
        ),
        (
            "outbid-heritage",
            "Outbid Heritage",
            "The projects that seeded this world came from outbid.lol. Their\
             auction histories are the founding lore — read them before you\
             bid against them.",
            20,
            None,
        ),
    ];

    for (slug, title, body, reward, avail) in items {
        sqlx::query(
            r#"
            INSERT INTO content_items (slug, title, body_md, rp_reward, available_from)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
        )
        .bind(slug)
        .bind(title)
        .bind(body)
        .bind(reward)
        .bind(avail.map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()))
        .execute(db)
        .await?;
    }
    Ok(())
}

/// List currently claimable content.
pub async fn list_content(db: &PgPool) -> Result<Vec<ContentItem>, sqlx::Error> {
    sqlx::query_as::<_, ContentItem>(
        r#"
        SELECT slug, title, body_md, rp_reward, available_from
        FROM content_items
        WHERE available_from IS NULL OR available_from <= CURRENT_DATE
        ORDER BY rp_reward DESC, slug ASC
        "#,
    )
    .fetch_all(db)
    .await
}

/// Reward reading/engaging with one content item. Once per (wallet, slug).
/// Rewards land as bonus lots expiring at `expires_at`.
pub async fn content_read_reward(
    db: &PgPool,
    wallet: &str,
    slug: &str,
    expires_at: DateTime<Utc>,
) -> Result<Option<(i64, String)>, sqlx::Error> {
    if !valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }
    let mut tx = db.begin().await?;

    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT rp_reward FROM content_items WHERE slug = $1 AND (available_from IS NULL OR available_from <= CURRENT_DATE)",
    )
    .bind(slug)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((reward,)) = row else {
        tx.rollback().await?;
        return Ok(None);
    };

    let inserted = sqlx::query(
        "INSERT INTO content_reads (wallet, slug) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(wallet)
    .bind(slug)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if inserted == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    grant_free_lot_tx(
        &mut tx,
        wallet,
        reward,
        RpSource::Bonus,
        &format!("content:{slug}"),
        expires_at,
    )
    .await?;

    tx.commit().await?;
    Ok(Some((
        reward,
        "promotional RP — non-cashable, off-chain only".to_string(),
    )))
}

// ---------------------------------------------------------------------------
// Race sessions (private-side bookkeeping around the MagicBlock ER)
// ---------------------------------------------------------------------------

/// Open a private-side race record. The public mirror happens via open_race
/// on-chain by the client/backend authority.
pub async fn open_race_row(
    db: &PgPool,
    project_pda: &str,
    er_session: Option<&str>,
) -> Result<RaceRow, sqlx::Error> {
    let mut tx = db.begin().await?;
    let next: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(race_id), -1) + 1 FROM races WHERE project_pda = $1")
            .bind(project_pda)
            .fetch_one(&mut *tx)
            .await?;
    let inserted = sqlx::query_as::<_, RaceRow>(
        r#"
        INSERT INTO races (project_pda, race_id, er_session)
        VALUES ($1, $2, $3)
        RETURNING id, project_pda, race_id, er_session, status, opened_at, settled_at, settle_tx
        "#,
    )
    .bind(project_pda)
    .bind(next.0.max(0))
    .bind(er_session)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(inserted)
}

/// Mark a race settled and store its on-chain settlement signature.
pub async fn settle_race_row(
    db: &PgPool,
    project_pda: &str,
    race_id: i64,
    settle_tx: &str,
) -> Result<RaceRow, sqlx::Error> {
    let updated = sqlx::query_as::<_, RaceRow>(
        r#"
        UPDATE races SET status = 'settled', settled_at = now(), settle_tx = $3
        WHERE project_pda = $1 AND race_id = $2 AND status IN ('open', 'settling')
        RETURNING id, project_pda, race_id, er_session, status, opened_at, settled_at, settle_tx
        "#,
    )
    .bind(project_pda)
    .bind(race_id)
    .bind(settle_tx)
    .fetch_optional(db)
    .await?;
    Ok(updated.expect("race row existed"))
}

/// All race rows for a project (newest first).
pub async fn races_for(db: &PgPool, project_pda: &str) -> Result<Vec<RaceRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceRow>(
        r#"
        SELECT id, project_pda, race_id, er_session, status, opened_at, settled_at, settle_tx
        FROM races WHERE project_pda = $1
        ORDER BY race_id DESC
        "#,
    )
    .bind(project_pda)
    .fetch_all(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week_start_is_monday_midnight_utc() {
        let ws = current_week_start();
        assert_eq!(ws.weekday(), chrono::Weekday::Mon);
        assert_eq!(ws.time(), chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn wallet_validation_accepts_base58_rejects_junk() {
        assert!(valid_wallet("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"));
        assert!(!valid_wallet(""));
        assert!(!valid_wallet("0OIl-invalid")); // 0, O, I, l are not base58 chars
        assert!(!valid_wallet(&"a".repeat(45)));
    }

    #[test]
    fn week_math_is_stable_across_a_known_tuesday() {
        // 2026-08-25 was a Tuesday; Monday 00:00 UTC is the week start.
        let tuesday =
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 25, 15, 30, 0).unwrap();
        let ws = week_start_of(tuesday);
        assert_eq!(ws.weekday(), chrono::Weekday::Mon);
        assert_eq!(
            ws,
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 24, 0, 0, 0).unwrap()
        );
        // Next boundary is strictly after: the following Monday.
        let nxt = next_week_start(tuesday);
        assert_eq!(
            nxt,
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 31, 0, 0, 0).unwrap()
        );
        // A claim made exactly ON a Monday boundary expires the NEXT Monday.
        let monday_noon =
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 24, 12, 0, 0).unwrap();
        assert_eq!(
            next_week_start(monday_noon),
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 31, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn rp_sources_round_trip() {
        for s in [
            RpSource::Paid,
            RpSource::FreeWeekly,
            RpSource::Bonus,
            RpSource::EventMultiplier,
        ] {
            assert_eq!(RpSource::parse(s.as_str()), Some(s));
        }
        assert_eq!(RpSource::parse("nonsense"), None);
    }
}
}
