use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::settings::{self, Settings};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

/// Current provider settings — masked (keys returned only as `*_key_set` flags).
pub async fn get(
    State(st): State<AppState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ServerError> {
    let s = settings::load(&st.db)
        .await
        .map_err(ServerError::Other)?
        .unwrap_or_default();
    Ok(Json(serde_json::to_value(s.to_view(&st.cfg)).unwrap()))
}

/// Save provider settings. Empty key fields keep the stored secret.
pub async fn put(
    State(st): State<AppState>,
    _user: AuthUser,
    Json(patch): Json<Settings>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let merged = settings::save(&st.db, patch)
        .await
        .map_err(ServerError::Other)?;
    Ok(Json(serde_json::to_value(merged.to_view(&st.cfg)).unwrap()))
}

/// Ping each configured provider with a cheap request to validate the keys.
pub async fn verify(
    State(st): State<AppState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ServerError> {
    let s = settings::load(&st.db)
        .await
        .map_err(ServerError::Other)?
        .unwrap_or_default();
    let eff = s.apply_to(st.cfg.clone());
    let http = reqwest::Client::new();
    let llm = verify_llm(&eff, &http).await;
    let tts = verify_tts(&eff, &http).await;
    Ok(Json(serde_json::json!({ "llm": llm, "tts": tts })))
}

fn result_json(provider: &str, r: Result<(), String>) -> serde_json::Value {
    match r {
        Ok(()) => serde_json::json!({ "provider": provider, "ok": true }),
        Err(e) => serde_json::json!({ "provider": provider, "ok": false, "error": e }),
    }
}

async fn ping(req: reqwest::RequestBuilder) -> Result<(), String> {
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED
        || resp.status() == reqwest::StatusCode::FORBIDDEN
    {
        Err("invalid API key".into())
    } else {
        Err(format!("provider returned HTTP {}", resp.status().as_u16()))
    }
}

async fn verify_llm(cfg: &crate::config::Config, http: &reqwest::Client) -> serde_json::Value {
    match cfg.llm_provider.as_str() {
        "mock" => serde_json::json!({ "provider": "mock", "ok": true }),
        "openai" => result_json("openai", verify_openai_key(&cfg.openai_api_key, http).await),
        _ => result_json(
            "anthropic",
            match cfg.anthropic_api_key.as_deref().filter(|k| !k.is_empty()) {
                None => Err("no API key set".into()),
                Some(k) => {
                    ping(
                        http.get("https://api.anthropic.com/v1/models")
                            .header("x-api-key", k)
                            .header("anthropic-version", "2023-06-01"),
                    )
                    .await
                }
            },
        ),
    }
}

async fn verify_tts(cfg: &crate::config::Config, http: &reqwest::Client) -> serde_json::Value {
    match cfg.tts_provider.as_str() {
        "mock" => serde_json::json!({ "provider": "mock", "ok": true }),
        "openai" => result_json("openai", verify_openai_key(&cfg.openai_api_key, http).await),
        _ => result_json(
            "elevenlabs",
            match cfg.elevenlabs_api_key.as_deref().filter(|k| !k.is_empty()) {
                None => Err("no API key set".into()),
                Some(k) => {
                    ping(
                        http.get("https://api.elevenlabs.io/v1/voices")
                            .header("xi-api-key", k),
                    )
                    .await
                }
            },
        ),
    }
}

async fn verify_openai_key(key: &Option<String>, http: &reqwest::Client) -> Result<(), String> {
    match key.as_deref().filter(|k| !k.is_empty()) {
        None => Err("no API key set".into()),
        Some(k) => ping(http.get("https://api.openai.com/v1/models").bearer_auth(k)).await,
    }
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

    async fn setup() -> (AppState, String) {
        let db = crate::testutil::pool().await;
        let token = "ak_settings_test";
        let uid = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, token_hash) VALUES (?, ?, ?)")
            .bind(uid)
            .bind(format!("t-{}@e.com", Uuid::new_v4()))
            .bind(hash_token(token))
            .execute(&db)
            .await
            .unwrap();
        (AppState::for_test(db, crate::testutil::cfg()), token.into())
    }

    #[tokio::test]
    async fn get_settings_requires_auth() {
        let (state, _token) = setup().await;
        let resp = router(state)
            .oneshot(
                Request::builder()
                    .uri("/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn put_then_get_roundtrips_and_masks_key() {
        let (state, token) = setup().await;
        let app = router(state);
        let body = serde_json::json!({
            "llm_provider": "openai",
            "openai_api_key": "sk-supersecret",
            "tts_provider": "mock"
        });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/settings")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/settings")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        // The raw secret must never be returned.
        assert!(!text.contains("sk-supersecret"), "leaked key: {text}");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["llm_provider"], "openai");
        assert_eq!(v["openai_key_set"], true);
        assert_eq!(v["anthropic_key_set"], false);
    }
}
