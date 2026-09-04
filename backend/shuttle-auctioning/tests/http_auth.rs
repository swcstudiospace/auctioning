//! HTTP-level authorization tests over the real router and a real Postgres.
//!
//! Self-skips when DATABASE_URL is unset. Resets the public schema, so point
//! it at a disposable database only.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use shuttle_auctioning::config::AppConfig;
use shuttle_auctioning::{ratelimit, router, AppState};
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;
use tower::ServiceExt;

const INGEST: &str = "ingest-secret-0123456789";
const OPERATOR: &str = "operator-token-0123456789";

/// Tests share one disposable database and reset it, so they run serially.
fn serial() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn app() -> Option<axum::Router> {
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
    sqlx::query(
        "INSERT INTO projects (handle, stable_id, display_name) VALUES ('car-a', 'car-a', 'Car A')",
    )
    .execute(&pool)
    .await
    .unwrap();

    std::env::set_var("APP_ENV", "staging");
    std::env::set_var("INGEST_SECRET", INGEST);
    std::env::set_var("OPERATOR_TOKEN", OPERATOR);
    std::env::set_var("WHOP_WEBHOOK_SECRET", "whsec_test");
    std::env::set_var("ALLOWED_ORIGINS", "https://auctioning.lol");
    std::env::set_var("WEEKLY_FREE_RP", "50");
    let cfg = AppConfig::from_env();
    cfg.validate().expect("staging config valid");
    Some(router(AppState {
        db: pool,
        cfg: Arc::new(cfg),
        limiter: ratelimit::RateLimiter::new(),
    }))
}

async fn call(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()))
    };
    (status, body)
}

fn json_req(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn sign_in(app: &axum::Router, kp: &Keypair) -> String {
    let wallet = kp.pubkey().to_string();
    let (status, body) = call(
        app,
        Request::get(format!("/v1/auth/nonce?wallet={wallet}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let message = body["message"].as_str().unwrap().to_string();
    let nonce = body["nonce"].as_str().unwrap().to_string();
    let signature = kp.sign_message(message.as_bytes()).to_string();
    let (status, body) = call(
        app,
        json_req(
            "POST",
            "/v1/auth/verify",
            json!({ "wallet": wallet, "nonce": nonce, "signature": signature }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn sign_in_claim_and_support_are_wallet_bound() {
    let _serial = serial().lock().await;
    let Some(app) = app().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let alice = Keypair::new();
    let mallory = Keypair::new();

    // No session → 401 on every wallet-bound write.
    let (status, _) = call(&app, json_req("POST", "/v1/rp/claim-weekly", json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let token = sign_in(&app, &alice).await;
    let bearer = format!("Bearer {token}");

    // /me reflects the session wallet.
    let (status, body) = call(
        &app,
        Request::get("/v1/auth/me")
            .header(header::AUTHORIZATION, &bearer)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["wallet"], alice.pubkey().to_string());

    // Claim weekly with the session; a body wallet that disagrees is 403.
    let mut req = json_req("POST", "/v1/rp/claim-weekly", json!({}));
    req.headers_mut()
        .insert(header::AUTHORIZATION, bearer.parse().unwrap());
    let (status, body) = call(&app, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["amount"], 50);

    let mut req = json_req(
        "POST",
        "/v1/rp/spend",
        json!({ "wallet": mallory.pubkey().to_string(), "amount": 1, "reason": "steal" }),
    );
    req.headers_mut()
        .insert(header::AUTHORIZATION, bearer.parse().unwrap());
    let (status, _) = call(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Support a car with the session wallet; the allocation is attributed to alice.
    let mut req = json_req(
        "POST",
        "/v1/projects/car-a/support",
        json!({ "amount": 10, "reason": "go" }),
    );
    req.headers_mut()
        .insert(header::AUTHORIZATION, bearer.parse().unwrap());
    let (status, body) = call(&app, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["project_total_rp"], 10);

    let (status, body) = call(
        &app,
        Request::get("/v1/projects/car-a/stats")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["lifetime"]["race_rp"], 10);
    assert_eq!(body["lifetime"]["supporters"], 1);

    // Own history requires the session and only shows alice.
    let (status, body) = call(
        &app,
        Request::get("/v1/wallets/me/history")
            .header(header::AUTHORIZATION, &bearer)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["allocations"].as_array().unwrap().len(), 1);

    // Logout revokes the token.
    let (status, _) = call(
        &app,
        Request::post("/v1/auth/logout")
            .header(header::AUTHORIZATION, &bearer)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &app,
        Request::get("/v1/auth/me")
            .header(header::AUTHORIZATION, &bearer)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn nonce_is_single_use_and_signature_must_match() {
    let _serial = serial().lock().await;
    let Some(app) = app().await else {
        return;
    };
    let kp = Keypair::new();
    let wallet = kp.pubkey().to_string();
    let (_, body) = call(
        &app,
        Request::get(format!("/v1/auth/nonce?wallet={wallet}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let message = body["message"].as_str().unwrap().to_string();
    let nonce = body["nonce"].as_str().unwrap().to_string();

    // Wrong signer.
    let bad = Keypair::new().sign_message(message.as_bytes()).to_string();
    let (status, _) = call(
        &app,
        json_req(
            "POST",
            "/v1/auth/verify",
            json!({ "wallet": wallet, "nonce": nonce, "signature": bad }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Right signer, then replay.
    let good = kp.sign_message(message.as_bytes()).to_string();
    let verify = || {
        json_req(
            "POST",
            "/v1/auth/verify",
            json!({ "wallet": wallet, "nonce": nonce, "signature": good }),
        )
    };
    let (status, _) = call(&app, verify()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, verify()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "nonce must be single-use");
}

#[tokio::test]
async fn operator_and_ingest_gates_hold() {
    let _serial = serial().lock().await;
    let Some(app) = app().await else {
        return;
    };
    // Operator-only queue.
    let (status, _) = call(
        &app,
        Request::get("/v1/narrative/queue")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &app,
        Request::get("/v1/narrative/queue")
            .header("x-auctioning-operator", OPERATOR)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &app,
        Request::get("/v1/stats/revenue")
            .header("x-auctioning-operator", "wrong")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Ingest-only minting.
    let wallet = Keypair::new().pubkey().to_string();
    let mint = json!({ "wallet": wallet, "amount": 5, "reason": "test" });
    let (status, _) = call(&app, json_req("POST", "/v1/rp/earn", mint.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let mut req = json_req("POST", "/v1/rp/earn", mint);
    req.headers_mut()
        .insert("x-auctioning-ingest", INGEST.parse().unwrap());
    let (status, body) = call(&app, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["free_rp"], 5);

    // Public reads still open, readiness reports the migration version.
    let (status, body) = call(&app, Request::get("/readyz").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["migration_version"].as_i64().unwrap() >= 9);
    let (status, body) = call(
        &app,
        Request::get("/v1/stats/overview")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["overview"]["projects"], 1);
}

#[tokio::test]
async fn whop_webhook_is_verified_logged_and_idempotent() {
    let _serial = serial().lock().await;
    let Some(app) = app().await else {
        return;
    };
    use hmac::{Hmac, Mac};
    let wallet = Keypair::new().pubkey().to_string();
    let payload = json!({
        "type": "payment.succeeded",
        "data": { "id": "pay_1", "wallet_address": wallet, "final_amount": 25.0, "product_id": "plan_x" }
    })
    .to_string();
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(b"whsec_test").unwrap();
    mac.update(payload.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    let deliver = |sig: &str| {
        Request::post("/v1/whop/webhook")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-whop-signature", sig)
            .body(Body::from(payload.clone()))
            .unwrap()
    };
    let (status, _) = call(&app, deliver("deadbeef")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = call(&app, deliver(&sig)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["outcome"], "recorded");
    let (status, body) = call(&app, deliver(&sig)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["outcome"], "duplicate");

    let (_, body) = call(
        &app,
        Request::get(format!("/v1/rp/{wallet}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(body["paid_rp"], 25, "credited once at $1 = 1 RP");
}
