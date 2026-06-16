mod auth;
mod config;
mod db;
mod error;
mod pipeline;
mod providers;
mod routes;
mod state;
mod storage;
#[cfg(test)]
mod testutil;
mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = config::Config::from_env()?;

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("seed-user") {
        let rotate = args.iter().any(|a| a == "--rotate");
        let email = args
            .iter()
            .skip(2)
            .find(|a| !a.starts_with("--"))
            .cloned()
            .unwrap_or_else(|| "dev@example.com".into());
        let db = db::connect(&cfg.database_url).await?;
        let existing: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
                .bind(&email)
                .fetch_optional(&db)
                .await?;
        // Don't clobber a working token on re-run — that silently logs out the
        // CLI. Only issue a token for a new user, or when --rotate is given.
        if existing.is_some() && !rotate {
            println!(
                "user {email} already exists; existing token kept. \
                 Pass --rotate to issue a new token (invalidates the old one)."
            );
            return Ok(());
        }
        let token = format!("ak_{}", uuid::Uuid::new_v4().simple());
        let hash = auth::hash_token(&token);
        sqlx::query(
            "INSERT INTO users (id, email, token_hash) VALUES (?, ?, ?)
             ON CONFLICT (email) DO UPDATE SET token_hash=excluded.token_hash",
        )
        .bind(uuid::Uuid::new_v4())
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
