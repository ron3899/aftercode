use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

#[allow(clippy::type_complexity)]
pub async fn list(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
) -> Result<Json<serde_json::Value>, ServerError> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            Option<i32>,
            Option<serde_json::Value>,
            chrono::DateTime<chrono::Utc>,
            String,
        ),
    >(
        "SELECT e.id, e.title, e.language, e.status::text, e.duration_seconds, e.topics_json,
                e.created_at, p.name
         FROM podcast_episodes e JOIN projects p ON p.id = e.project_id
         WHERE e.user_id=$1 ORDER BY e.created_at DESC",
    )
    .bind(uid)
    .fetch_all(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;
    let items: Vec<_> = rows
        .into_iter()
        .map(|(id, title, lang, status, dur, topics, created, proj)| {
            let topic_titles: Vec<String> = topics
                .as_ref()
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x["title"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            serde_json::json!({ "id":id,"title":title,"language":lang,"status":status,
                "duration_seconds":dur,"topics":topic_titles,"project_name":proj,
                "created_at":created.to_rfc3339() })
        })
        .collect();
    Ok(Json(serde_json::json!({ "episodes": items })))
}

#[allow(clippy::type_complexity)]
pub async fn detail(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            Option<String>,
            Option<i32>,
            Option<String>,
            Option<String>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, title, language, status::text, audio_url, duration_seconds, summary,
                transcript_text, topics_json, script_json, error, created_at
         FROM podcast_episodes WHERE id=$1 AND user_id=$2",
    )
    .bind(id)
    .bind(uid)
    .fetch_optional(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?
    .ok_or(ServerError::NotFound)?;
    Ok(Json(serde_json::json!({
        "id":row.0,"title":row.1,"language":row.2,"status":row.3,"audio_url":row.4,
        "duration_seconds":row.5,"summary":row.6,"transcript_text":row.7,
        "topics":row.8,"script":row.9,"error":row.10,"created_at":row.11.to_rfc3339() })))
}

pub async fn retry(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT s.context_json FROM podcast_episodes e
         JOIN coding_sessions s ON s.id = e.session_id
         WHERE e.id=$1 AND e.user_id=$2",
    )
    .bind(id)
    .bind(uid)
    .fetch_optional(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?
    .ok_or(ServerError::NotFound)?;
    let ctx: aftercode_core::session::SessionContext =
        serde_json::from_value(row).map_err(|e| ServerError::Other(e.into()))?;
    crate::db::queries::set_status(&st.db, id, "queued")
        .await
        .map_err(ServerError::Other)?;
    crate::worker::spawn(st.clone(), id, ctx);
    Ok(Json(
        serde_json::json!({ "episode_id": id, "status": "queued" }),
    ))
}

#[cfg(test)]
mod tests {
    use crate::auth::hash_token;
    use crate::routes::router;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn setup() -> (AppState, String, Uuid) {
        let cfg = crate::config::Config {
            database_url: std::env::var("DATABASE_URL").unwrap(),
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
            .connect(&cfg.database_url)
            .await
            .unwrap();
        let token = "ak_test_token";
        let uid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, token_hash) VALUES ($1,$2)
             ON CONFLICT (email) DO UPDATE SET token_hash=EXCLUDED.token_hash RETURNING id",
        )
        .bind(format!("t-{}@e.com", Uuid::new_v4()))
        .bind(hash_token(token))
        .fetch_one(&db)
        .await
        .unwrap();
        let pid: Uuid =
            sqlx::query_scalar("INSERT INTO projects (user_id, name) VALUES ($1,'p') RETURNING id")
                .bind(uid)
                .fetch_one(&db)
                .await
                .unwrap();
        (AppState::for_test(db, cfg), token.to_string(), pid)
    }

    #[tokio::test]
    #[serial_test::serial(env)]
    async fn generate_then_status_reaches_ready() {
        let (state, token, pid) = setup().await;
        let db = state.db.clone();
        let app = router(state);
        let body = serde_json::json!({
            "project_id": pid,
            "language": "en",
            "session_context": {
                "project_id": pid, "language": "en", "episode_length_minutes": 10,
                "collected_at": "2026-06-14T19:00:00Z",
                "changed_files": ["m.py"], "git_diff_summary": "x" }
        });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cli/generate-episode")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let eid = v["episode_id"].as_str().unwrap().to_string();

        // Worker runs async; poll the DB up to 5s.
        let id = Uuid::parse_str(&eid).unwrap();
        let mut status = String::new();
        for _ in 0..50 {
            status = sqlx::query_scalar::<_, String>(
                "SELECT status::text FROM podcast_episodes WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
            if status == "ready" || status == "failed" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert_eq!(status, "ready");
    }
}
