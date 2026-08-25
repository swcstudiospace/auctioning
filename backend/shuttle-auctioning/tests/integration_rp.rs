//! Integration tests for the priority-#1 dual-source RP model + seeding.
//!
//! These hit a REAL Postgres. They self-skip when DATABASE_URL is unset:
//!   DATABASE_URL=postgres://... cargo test -p shuttle-auctioning --test integration_rp
//!
//! The run RESETS the public schema at startup (drop + migrate), so point
//! DATABASE_URL at a disposable database only.

// The lib's modules are private (single deployable crate), so splice their
// sources in. tests/inc/*.rs are generated copies of src/{ledger,catalog}.rs
// with their `//!` headers rewritten to `//` (inner doc comments are illegal
// mid-file). Regenerate with scripts/gen-integration-includes.sh if those
// modules change shape.
include!("inc/ledger.rs");
include!("inc/catalog.rs");

// Re-export the spliced modules' public items so the test body reads like
// normal lib code.
use catalog::{
    allocate_to_project, allocations_for, get_project, import_projects, list_projects,
    parse_import_payload,
};
use ledger::{
    claim_weekly, credit_paid, ensure_wallet, events_for, expire_due_lots, grant_free_lot,
    next_week_start, reconcile_free_rp_cache, RpSource,
};

#[tokio::test]
async fn full_priority_one_flow() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to postgres");

    // Reset for repeatable runs against a disposable DB.
    // Separate statements because sqlx::query uses prepared statements which
    // do not support multiple commands.
    sqlx::query("DROP SCHEMA IF EXISTS public CASCADE;")
        .execute(&pool)
        .await
        .expect("drop schema");
    sqlx::query("CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .expect("create schema");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let suffix = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let w = format!("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVF{suffix}");
    // Make a valid-looking base58 wallet: the test validator rejects 0 O I l (and 0).
    // Map bad chars to safe ones so ensure_wallet etc. pass.
    let w: String = w
        .chars()
        .map(|c| match c {
            '0' | 'O' | 'o' | 'I' | 'i' | 'l' | 'L' => '1',
            c if c.is_ascii_alphanumeric() => c,
            _ => 'a',
        })
        .collect();
    // Trim/pad to plausible Solana address length if needed (the good example is ~44)
    let w = if w.len() > 44 { w[..44].to_string() } else { w };

    // 1) Seeding: import a batch, then re-import (idempotent).
    let batch = parse_import_payload(
        r#"[
            {"stable_id": "it:alpha", "display_name": "Alpha Co", "tags": [" AI ", "dev-tools"], "source_ref": "snap:1"},
            {"stable_id": "it:beta", "display_name": "Beta Pty", "tags": ["marketing"]},
            {"stable_id": "it:gamma", "display_name": "Gamma Labs", "url": "https://gamma.example"}
        ]"#,
    )
    .unwrap();
    let (imported, updated) = import_projects(&pool, &batch).await.unwrap();
    assert_eq!(imported, 3, "first import inserts all three");
    assert_eq!(updated, 0);

    let (imported2, updated2) = import_projects(&pool, &batch).await.unwrap();
    assert_eq!(imported2, 0, "reseed imports nothing");
    assert_eq!(updated2, 3, "reseed refreshes all three");

    // Descriptive fields update without clobbering identity.
    let mut v2 = parse_import_payload(
        r#"[{"stable_id": "it:alpha", "display_name": "Alpha Co v2", "blurb": "now with blurb"}]"#,
    )
    .unwrap();
    v2[0].owner_wallet = None;
    let (_, u) = import_projects(&pool, &v2).await.unwrap();
    assert_eq!(u, 1);
    // "it:alpha" sanitizes (rsplit(':').next()) to handle "alpha" and updates in place.
    let alpha = get_project(&pool, "alpha")
        .await
        .unwrap()
        .expect("sanitized handle exists");
    assert_eq!(alpha.display_name.as_deref(), Some("Alpha Co v2"));
    assert_eq!(alpha.total_rp, 0, "imports land at zero RP");

    // 2) Weekly stipend -> lot with next-Monday expiry, cache consistent.
    let now = chrono::Utc::now();
    let claimed = claim_weekly(&pool, &w, 50, next_week_start(now))
        .await
        .unwrap();
    assert_eq!(claimed, Some(50));
    let again = claim_weekly(&pool, &w, 50, next_week_start(now))
        .await
        .unwrap();
    assert_eq!(again, None, "second claim in the same week is refused");

    // 3) Bonus lot stacks FIFO alongside the stipend.
    grant_free_lot(
        &pool,
        &w,
        30,
        RpSource::Bonus,
        "content:test",
        now + chrono::Duration::hours(2),
    )
    .await
    .unwrap();
    let l = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!(l.free_rp, 80);

    // 4) Project allocation drains earliest-expiring lot first and records provenance.
    // NOTE: use the *sanitized handle* ("alpha"), not the stable_id ("it:alpha").
    let outcome = allocate_to_project(&pool, &w, "alpha", 40, Some("they shipped"))
        .await
        .unwrap()
        .expect("allocation succeeds");
    assert_eq!(outcome.from_free, 40);
    assert_eq!(outcome.from_paid, 0);
    assert_eq!(outcome.project_total_rp, 40);
    assert_eq!(outcome.allocation.bucket, "free");
    assert_eq!(
        outcome.allocation.source, "bonus",
        "drained the short-dated bonus lot first"
    );
    let l = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!(l.free_rp, 40, "cache tracks the drain");

    // 5) Paid RP stays separate and cashable-provenance typed.
    let l = credit_paid(&pool, &w, 100, "whop:payment.succeeded", Some("tx123"))
        .await
        .unwrap();
    assert_eq!((l.paid_rp, l.free_rp), (100, 40));

    // Mixed allocation: free drains out, remainder comes from paid.
    let mixed = allocate_to_project(&pool, &w, "beta", 90, None)
        .await
        .unwrap()
        .expect("mixed allocation succeeds");
    assert_eq!((mixed.from_free, mixed.from_paid), (40, 50));
    assert_eq!(mixed.project_total_rp, 90);
    let l = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!((l.paid_rp, l.free_rp), (50, 0));

    // Insufficient funds: atomic refusal, no partial writes anywhere.
    let broke = allocate_to_project(&pool, &w, "alpha", 999_999, None)
        .await
        .unwrap();
    assert!(broke.is_none());
    let alpha_after = get_project(&pool, "alpha").await.unwrap().unwrap();
    assert_eq!(
        alpha_after.total_rp, 40,
        "failed allocation leaves project untouched"
    );

    // Unknown project: clean 404-shaped None.
    let unknown = allocate_to_project(&pool, &w, "does-not-exist", 5, None)
        .await
        .unwrap();
    assert!(unknown.is_none());

    // Allocation history is complete and ordered.
    let hist = allocations_for(&pool, "alpha", 10).await.unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0].amount, 40);

    // 6) Expiry sweep lapses stale lots into the audit trail.
    // Seed a synthetic already-due lot via raw INSERT (grant_free_lot always
    // uses DEFAULT now() for granted_at and would violate the > check if we
    // passed a past expires_at). We satisfy (expires_at > granted_at) while
    // making both in the past so the sweep will pick it up.
    sqlx::query(
        r#"INSERT INTO free_rp_lots
             (wallet, amount, remaining, source, reason, granted_at, expires_at)
           VALUES ($1, 25, 25, 'event_multiplier', 'double-rp-hour',
                   now() - interval '10 min', now() - interval '5 min')"#,
    )
    .bind(&w)
    .execute(&pool)
    .await
    .unwrap();
    // Record a matching inflow event so "event_multiplier" appears in the
    // sources_seen audit (the expire path itself leaves source NULL by design).
    sqlx::query(
        r#"INSERT INTO ledger_events (wallet, kind, source, amount, reason)
           VALUES ($1, 'free', 'event_multiplier', 25, 'double-rp-hour')"#,
    )
    .bind(&w)
    .execute(&pool)
    .await
    .unwrap();
    // Bump the wallet cache to match (the lot is "active" from the row's POV).
    sqlx::query("UPDATE wallets SET free_rp = free_rp + 25 WHERE wallet = $1")
        .bind(&w)
        .execute(&pool)
        .await
        .unwrap();

    let l_before = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!(l_before.free_rp, 25);
    let moved = expire_due_lots(&pool).await.unwrap();
    assert!(moved >= 1, "at least our wallet was swept");
    let l_after = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!(l_after.free_rp, 0, "only the expired lot remained");

    let events = events_for(&pool, &w, 50).await.unwrap();
    assert!(
        events.iter().any(|e| e.kind == "expire" && e.amount == 25),
        "expiry lands in the append-only audit trail"
    );
    let sources_seen: Vec<&str> = events.iter().filter_map(|e| e.source.as_deref()).collect();
    assert!(sources_seen.contains(&"paid"));
    assert!(sources_seen.contains(&"free_weekly"));
    assert!(sources_seen.contains(&"bonus"));
    assert!(sources_seen.contains(&"event_multiplier"));

    // 7) Cache drift repair.
    sqlx::query("UPDATE wallets SET free_rp = 777 WHERE wallet = $1")
        .bind(&w)
        .execute(&pool)
        .await
        .unwrap();
    let repaired = reconcile_free_rp_cache(&pool).await.unwrap();
    assert!(repaired >= 1);
    let l_fixed = ensure_wallet(&pool, &w).await.unwrap();
    assert_eq!(l_fixed.free_rp, 0);

    // 8) Board ranking reflects allocations.
    let board = list_projects(&pool, 10).await.unwrap();
    let alpha_row = board.iter().find(|p| p.handle == "alpha").unwrap();
    let beta_row = board.iter().find(|p| p.handle == "beta").unwrap();
    assert_eq!(alpha_row.total_rp, 40);
    assert_eq!(beta_row.total_rp, 90);
    assert!(beta_row.rank <= alpha_row.rank, "higher RP ranks better");

    pool.close().await;
}
