mod auth;
mod config;
mod db;
mod error;
mod pipeline;
mod providers;
mod routes;
mod state;
mod storage;
mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = config::Config::from_env()?;

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("seed-user") {
        let email = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "dev@example.com".into());
        let token = format!("ak_{}", uuid::Uuid::new_v4().simple());
        let hash = auth::hash_token(&token);
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect(&cfg.database_url)
            .await?;
        sqlx::query(
            "INSERT INTO users (email, token_hash) VALUES ($1,$2)
             ON CONFLICT (email) DO UPDATE SET token_hash=EXCLUDED.token_hash",
        )
        .bind(&email)
        .bind(&hash)
        .execute(&db)
        .await?;
        println!("user {email} token: {token}");
        return Ok(());
    }

    let state = state::AppState::new(cfg.clone()).await?;
    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
