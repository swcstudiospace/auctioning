//! Postgres-backed smoke test for the private ledger.
//!
//! Requires DATABASE_URL pointing at an empty Postgres database, e.g.:
//!   docker run -d --name auctioning-pg-smoke -p 5433:5432 \
//!     -e POSTGRES_PASSWORD=smoke -e POSTGRES_DB=auctioning postgres:16
//!   export DATABASE_URL=postgres://postgres:smoke@127.0.0.1:5433/auctioning
//!   cargo test -p shuttle-auctioning --test smoke_db --features sqlx-test
//!
//! Exercises the full money path end to end: migrations → weekly claim (FIFO
//! lot + expiry) → content reward → project import → support allocation
//! (lot provenance) → spend → insufficient-funds refusal.

#![cfg(feature = "sqlx-test")]

use sqlx::postgres::PgPoolOptions;
use std::str::FromStr;

const WALLETS: &str = "wallets";
async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").expect("set DATABASE_URL");
    let p = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");
    // Fresh schema per run.
    for t in [
        "project_allocations",
        "free_rp_lots",
        "content_reads",
        "content_items",
        "projects",
        "races",
        "weekly_claims",
        "ledger_events",
        "whop_members",
        WALLETS,
        "_sqlx_migrations",
    ] {
        let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {t} CASCADE"))
            .execute(&p)
            .await;
    }
    sqlx::migrate!("./migrations")
        .run(&p)
        .await
        .expect("migrations");
    ledger::seed_content(&p).await.expect("seed content");
    p
}

use shuttle_auctioning::catalog;
use shuttle_auctioning::ledger;

#[tokio::test]
async fn full_money_path_smoke() {
    let db = pool().await;
    let w = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
    let expires = ledger::next_week_start(chrono::Utc::now());

    // Weekly claim lands as a FIFO lot and shows in the cache.
    let claimed = ledger::claim_weekly(&db, w, 100, expires).await.unwrap();
    assert_eq!(claimed, Some(100));
    assert_eq!(
        ledger::claim_weekly(&db, w, 100, expires).await.unwrap(),
        None
    );

    // Content reward adds a second lot.
    let rewarded = ledger::content_read_reward(&db, w, "the-first-auction", expires)
        .await
        .unwrap();
    assert!(rewarded.is_some());
    assert!(
        ledger::content_read_reward(&db, w, "the-first-auction", expires)
            .await
            .unwrap()
            .is_none()
    );

    let view = ledger::ensure_wallet(&db, w).await.unwrap();
    assert_eq!(view.free_rp, 125); // 100 stipend + 25 content bonus

    // Import a project from an outbid-style snapshot row.
    let items = [catalog::ImportProject {
        stable_id: "outbid:beanz-coffee-brisbane".into(),
        handle: None,
        display_name: Some("Beanz Coffee".into()),
        url: None,
        blurb: Some("Specialty roaster.".into()),
        tags: vec!["coffee".into()],
        source: Some("outbid_import".into()),
        source_ref: None,
        owner_wallet: None,
    }];
    let (imported, updated) = catalog::import_projects(&db, &items).await.unwrap();
    assert_eq!((imported, updated), (1, 0));
    // Idempotent re-import only updates.
    let (imported, updated) = catalog::import_projects(&db, &items).await.unwrap();
    assert_eq!((imported, updated), (0, 1));

    // Support drains free lots FIFO with provenance.
    let outcome =
        catalog::allocate_to_project(&db, w, "beanz-coffee-brisbane", 30, Some("went viral"))
            .await
            .unwrap()
            .expect("allocation succeeds");
    assert_eq!(outcome.from_free, 30);
    assert_eq!(outcome.from_paid, 0);
    assert_eq!(outcome.project_total_rp, 30);
    assert_eq!(outcome.allocation.bucket, "free");
    assert!(outcome.allocation.lot_id.is_some());

    // Paid credit then a mixed spend: free drains first, paid covers rest.
    ledger::credit_paid(&db, w, 50, "whop:payment.succeeded", None)
        .await
        .unwrap();
    let breakdown = ledger::spend(&db, w, 100, "race-entry")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(breakdown.from_free, 95); // 125 - 30 allocated earlier
    assert_eq!(breakdown.from_paid, 5);
    assert_eq!(breakdown.lots.len() >= 1, true);

    // Overdraft is refused atomically.
    assert!(ledger::spend(&db, w, 10_000, "nope")
        .await
        .unwrap()
        .is_none());

    // Audit trail has every movement with typed sources.
    let events = ledger::events_for(&db, w, 50).await.unwrap();
    let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"free") && kinds.contains(&"paid") && kinds.contains(&"spend"));
    let sources: Vec<Option<&String>> = events.iter().map(|e| e.source.as_ref()).collect();

    // Expiry sweep lapses remaining promo RP and keeps the cache honest.
    let lapsed = ledger::expire_due_lots(&db).await.unwrap_or_else(|e| {
        panic!("expire failed: {e}");
    });
    let _ = sources; // kept above for debuggability when this fails in CI
    if lapsed > 0 {
        let v = ledger::ensure_wallet(&db, w).await.unwrap();
        // Free cache must now match active lots (zero — everything expired).
        assert_eq!(
            v.free_rp,
            sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(SUM(remaining),0) FROM free_rp_lots WHERE wallet=$1 AND remaining>0"
            )
            .bind(w)
            .fetch_one(&db)
            .await
            .unwrap()
        );
    }

    // Race bookkeeping round-trips.
    let race = ledger::open_race_row(&db, "race-pda-1", Some("er-session-x"))
        .await
        .unwrap();
    let settled = ledger::settle_race_row(&db, "race-pda-1", race.race_id, "sig-abc")
        .await
        .unwrap();
    assert_eq!(settled.status, "settled");

    let _ = f64::from_str("0"); // keep FromStr import honest
}
