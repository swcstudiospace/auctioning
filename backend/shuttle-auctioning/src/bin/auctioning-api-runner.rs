//! Local Axum runner (no Shuttle). Bind DATABASE_URL + BIND_ADDR.
//! Used so marketing can talk to a rebuilt API on :8000.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".into());
    let pool = sqlx::PgPool::connect(&url)
        .await
        .expect("postgres connect");
    let cfg = shuttle_auctioning::config::AppConfig::from_env();
    let router = shuttle_auctioning::build_app(pool, cfg).await;
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("bind {bind}: {e}"));
    tracing::info!(%bind, "auctioning-api-runner listening");
    axum::serve(listener, router).await.expect("serve");
}
