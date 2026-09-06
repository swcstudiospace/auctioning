//! outbid.lol mirror against a real Postgres. Self-skips when DATABASE_URL is
//! unset; resets the public schema, so point it at a disposable database.
//!
//!   DATABASE_URL=postgres://... cargo test -p shuttle-auctioning --test outbid_sync

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use shuttle_auctioning::catalog::{allocations_for, get_project, submit_site, SubmitSite};
use shuttle_auctioning::config::AppConfig;
use shuttle_auctioning::outbid::{apply_sync, status, OutbidEntry, SyncRequest, MIRROR_WALLET};
use shuttle_auctioning::race_engine::lifetime_grid_uncached;
use shuttle_auctioning::{ratelimit, router, AppState};
use std::sync::Arc;
use tower::ServiceExt;

const INGEST: &str = "ingest-secret-0123456789";

async fn fresh_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect");
    sqlx::query("DROP SCHEMA IF EXISTS public CASCADE;")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    Some(pool)
}

fn entry(id: &str, host: &str, cents: i64) -> OutbidEntry {
    OutbidEntry {
        id: id.into(),
        identity_key: Some(format!("website:{host}")),
        display_name: Some(format!("{host} · demo")),
        description: Some("A listing".into()),
        source_url: Some(format!("https://{host}/?utm_source=outbid")),
        product_url: Some(format!("https://outbid.lol/product/{host}")),
        image_url: None,
        category_slug: Some("ai-agents-infrastructure".into()),
        category_name: Some("AI Agents & Infrastructure".into()),
        amount_cents: cents,
        click_count: Some(10),
        created_at: None,
        rank: Some(1),
        category_rank: Some(1),
    }
}

/// One serial test: the schema reset makes parallel tests race.
#[tokio::test]
async fn mirror_credits_increase_only_and_adopts_manual_rows() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };

    // A site someone already listed by hand must be adopted, not duplicated.
    let manual = submit_site(
        &pool,
        SubmitSite {
            url: "https://tutti.so".into(),
            display_name: Some("Tutti".into()),
            blurb: None,
            owner_wallet: None,
            tags: vec![],
        },
    )
    .await
    .unwrap();
    assert!(manual.created);
    assert_eq!(manual.project.handle, "tutti-so");

    // First push: two entries, one duplicated across boards with a lower amount.
    let req = SyncRequest {
        collector: Some("test".into()),
        collected_at: None,
        entries: vec![
            entry("e-see", "see.io", 1_700_000),
            entry("e-see-cat", "See.io", 1_600_000), // same host, older amount
            entry("e-tutti", "tutti.so", 1_600_050),
        ],
    };
    let out = apply_sync(&pool, &req).await.unwrap();
    assert_eq!(out.entries_seen, 3);
    assert_eq!(out.created, 1, "see.io is new");
    assert_eq!(out.updated, 1, "tutti.so adopted the manual row");
    assert_eq!(out.rp_credited, 17_000 + 16_000, "1 RP = $1, floored");
    assert_eq!(out.skipped, 0);

    let see = get_project(&pool, "see-io").await.unwrap().expect("see-io");
    assert_eq!(see.total_rp, 17_000);
    assert_eq!(see.mirrored_rp, 17_000);
    assert_eq!(see.outbid_amount_cents, 1_700_000);
    assert_eq!(see.stable_id.as_deref(), Some("outbid:see.io"));
    assert_eq!(see.source, "outbid_import");
    assert_eq!(see.tags, vec!["ai-agents-infrastructure"]);

    let tutti = get_project(&pool, "tutti-so")
        .await
        .unwrap()
        .expect("tutti");
    assert_eq!(tutti.total_rp, 16_000);
    assert_eq!(tutti.stable_id.as_deref(), Some("manual:tutti.so"));
    assert_eq!(tutti.outbid_rank, Some(1));

    // Allocations carry the mirror provenance and count as paid RP.
    let allocs = allocations_for(&pool, "see-io", 10).await.unwrap();
    assert_eq!(allocs.len(), 1);
    assert_eq!(allocs[0].supporter_wallet, MIRROR_WALLET);
    assert_eq!(allocs[0].source, "outbid_mirror");
    assert_eq!(allocs[0].bucket, "paid");
    let grid = lifetime_grid_uncached(&pool).await.unwrap();
    let top = grid.iter().find(|s| s.handle == "see-io").expect("on grid");
    assert_eq!(top.race_rp, 17_000);
    assert_eq!(top.paid_rp, 17_000);
    assert_eq!(top.rank, 1);

    // Second push: same amounts → nothing credited; a raise → only the delta;
    // a drop → ignored (append-only ledger).
    let req2 = SyncRequest {
        collector: Some("test".into()),
        collected_at: None,
        entries: vec![
            entry("e-see", "see.io", 1_700_500),   // +$5.00 → +5 RP
            entry("e-tutti", "tutti.so", 900_000), // decrease → ignored
        ],
    };
    let out2 = apply_sync(&pool, &req2).await.unwrap();
    assert_eq!(out2.created, 0);
    assert_eq!(out2.updated, 2);
    assert_eq!(out2.rp_credited, 5);
    let see = get_project(&pool, "see-io").await.unwrap().unwrap();
    assert_eq!(see.total_rp, 17_005);
    assert_eq!(see.mirrored_rp, 17_005);
    let tutti = get_project(&pool, "tutti-so").await.unwrap().unwrap();
    assert_eq!(tutti.total_rp, 16_000, "never clawed back");
    assert_eq!(
        tutti.outbid_amount_cents, 1_600_050,
        "amount cache keeps the max"
    );

    // Status read model reflects both runs.
    let st = status(&pool).await.unwrap();
    assert_eq!(st.totals.projects, 2);
    assert_eq!(st.totals.mirrored_rp, 17_005 + 16_000);
    assert_eq!(st.recent_runs.len(), 2);
    assert_eq!(st.last_run.as_ref().unwrap().status, "ok");

    // Entries with no usable host are skipped, not fatal.
    let mut junk = entry("e-junk", "see.io", 1);
    junk.identity_key = Some("website:not a host".into());
    junk.source_url = Some("nope".into());
    junk.product_url = None;
    let out3 = apply_sync(
        &pool,
        &SyncRequest {
            collector: None,
            collected_at: None,
            entries: vec![junk],
        },
    )
    .await
    .unwrap();
    assert_eq!(out3.skipped, 1);
    assert_eq!(out3.created + out3.updated, 0);

    // --- HTTP surface: ingest gate + public status --------------------------
    std::env::set_var("INGEST_SECRET", INGEST);
    let cfg = AppConfig::from_env();
    cfg.validate().expect("dev config valid");
    let app = router(AppState {
        db: pool.clone(),
        cfg: Arc::new(cfg),
        limiter: ratelimit::RateLimiter::new(),
    });

    let body = json!({
        "collector": "http-test",
        "entries": [{"id": "e-joni", "identity_key": "website:joni.ai", "amount_cents": 1_401_500}]
    });
    let unauth = app
        .clone()
        .oneshot(
            Request::post("/v1/outbid/sync")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let ok = app
        .clone()
        .oneshot(
            Request::post("/v1/outbid/sync")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-auctioning-ingest", INGEST)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let v: Value =
        serde_json::from_slice(&ok.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["created"], 1);
    assert_eq!(v["rp_credited"], 14_015);

    let st = app
        .clone()
        .oneshot(
            Request::get("/v1/outbid/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(st.status(), StatusCode::OK);
    let v: Value =
        serde_json::from_slice(&st.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["totals"]["projects"], 3);

    let hosts = app
        .oneshot(
            Request::get("/v1/outbid/hosts")
                .header("x-auctioning-ingest", INGEST)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hosts.status(), StatusCode::OK);
    let v: Value =
        serde_json::from_slice(&hosts.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let hosts: Vec<String> = serde_json::from_value(v["hosts"].clone()).unwrap();
    assert!(hosts.contains(&"see.io".to_string()));
    assert!(hosts.contains(&"joni.ai".to_string()));
    assert!(
        !hosts.contains(&"tutti.so".to_string()),
        "manual rows keep their manual: key"
    );
}
