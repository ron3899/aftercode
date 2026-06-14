use axum::Json;

pub async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[cfg(test)]
mod tests {
    use crate::routes::router;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let cfg = crate::config::Config {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| std::env::var("DATABASE_URL").unwrap()),
            bind_addr: "127.0.0.1:0".into(),
            public_url: "http://t".into(),
            llm_provider: "mock".into(),
            anthropic_api_key: None,
            openai_api_key: None,
            elevenlabs_api_key: None,
            host_voice_id: None,
            expert_voice_id: None,
            blob_store: "mock".into(),
            localfs_dir: "./data".into(),
            s3_bucket: None,
        };
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&cfg.database_url)
            .await
            .unwrap();
        AppState::for_test(db, cfg)
    }

    #[tokio::test]
    #[serial_test::serial(env)]
    async fn healthz_ok() {
        let app = router(test_state().await);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
