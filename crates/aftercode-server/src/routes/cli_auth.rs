use crate::auth::hash_token;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::{Form, Query, State};
use axum::response::{Html, IntoResponse, Redirect};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthorizeParams {
    /// CLI loopback port (CLI flow).
    pub port: Option<u16>,
    /// Web app URL to bounce back to (web flow).
    pub redirect: Option<String>,
    pub state: String,
}

/// Page the browser opens. Approve POSTs to /cli/approve, carrying the same
/// port/redirect/state so the backend knows where to send the token.
pub async fn authorize(Query(p): Query<AuthorizeParams>) -> impl IntoResponse {
    let state = sanitize(&p.state);
    let hidden = if let Some(port) = p.port {
        format!("<input type=hidden name=port value=\"{port}\">")
    } else if let Some(r) = &p.redirect {
        format!(
            "<input type=hidden name=redirect value=\"{}\">",
            html_attr(r)
        )
    } else {
        String::new()
    };
    let html = format!(
        "<!doctype html><html><head><meta charset=utf-8><title>Authorize Aftercode</title>\
         <style>body{{font-family:system-ui;max-width:32rem;margin:5rem auto;padding:0 1rem;\
         text-align:center;line-height:1.6;background:#14110f;color:#f3ece2}}\
         button{{font-size:1.1rem;padding:.7rem 2rem;background:#e0794b;color:#14110f;border:0;\
         border-radius:10px;cursor:pointer;font-weight:600}}</style></head>\
         <body><h1>Authorize Aftercode</h1>\
         <p>Sign this device in to the Aftercode backend.</p>\
         <form method=post action=\"/cli/approve\">{hidden}\
         <input type=hidden name=state value=\"{state}\">\
         <button type=submit>Approve</button></form>\
         <p style=\"color:#8a8178;font-size:.85rem;margin-top:2rem\">Local approval — no identity \
         check. Use only on a backend you trust on your own machine.</p></body></html>"
    );
    Html(html)
}

#[derive(Deserialize)]
pub struct ApproveParams {
    pub port: Option<u16>,
    pub redirect: Option<String>,
    pub state: String,
}

/// Mint a token and bounce it back — to the CLI loopback (`port`) or to the web
/// app (`redirect`, token in the URL fragment so it isn't logged server-side).
pub async fn approve(
    State(st): State<AppState>,
    Form(p): Form<ApproveParams>,
) -> Result<Redirect, ServerError> {
    let state = sanitize(&p.state);
    // Decide the destination before minting.
    let dest = match (p.port, p.redirect.as_deref()) {
        (Some(port), None) => {
            format!("http://127.0.0.1:{port}/callback?token=__TOKEN__&state={state}")
        }
        (None, Some(redirect)) => {
            if !redirect_allowed(redirect, &st.cfg.public_url) {
                return Err(ServerError::BadRequest("redirect not allowed".into()));
            }
            // token in the fragment (#) — never sent to a server / not logged.
            format!("{redirect}#token=__TOKEN__&state={state}")
        }
        _ => {
            return Err(ServerError::BadRequest(
                "provide exactly one of port|redirect".into(),
            ))
        }
    };

    let token = format!("ak_{}", Uuid::new_v4().simple());
    let hash = hash_token(&token);
    sqlx::query(
        "INSERT INTO users (email, token_hash) VALUES ('cli@local', $1)
         ON CONFLICT (email) DO UPDATE SET token_hash = EXCLUDED.token_hash",
    )
    .bind(&hash)
    .execute(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;

    Ok(Redirect::to(&dest.replace("__TOKEN__", &token)))
}

/// Allow only same-origin (APP_PUBLIC_URL) or localhost/127.0.0.1 (any port).
/// Prevents an open redirect from leaking the token to an arbitrary host.
fn redirect_allowed(redirect: &str, public_url: &str) -> bool {
    fn origin(u: &str) -> Option<(String, String)> {
        let rest = u
            .strip_prefix("http://")
            .or_else(|| u.strip_prefix("https://"))?;
        let scheme = if u.starts_with("https://") {
            "https"
        } else {
            "http"
        };
        let host = rest.split(['/', '#', '?']).next().unwrap_or("");
        Some((scheme.to_string(), host.to_string()))
    }
    let Some((_, host)) = origin(redirect) else {
        return false;
    };
    let host_only = host.split(':').next().unwrap_or("");
    if host_only == "localhost" || host_only == "127.0.0.1" {
        return true;
    }
    origin(public_url).map(|(_, h)| h) == Some(host)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(128)
        .collect()
}

fn html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::router;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn sanitize_strips_unsafe() {
        assert_eq!(sanitize("abc123-_"), "abc123-_");
        assert_eq!(sanitize("a\"<b> c"), "abc");
    }

    #[test]
    fn redirect_allows_localhost_and_same_origin() {
        assert!(redirect_allowed(
            "http://localhost:5173",
            "http://localhost:8090"
        ));
        assert!(redirect_allowed(
            "http://127.0.0.1:3000/app",
            "http://localhost:8090"
        ));
        assert!(redirect_allowed(
            "https://app.aftercode.ai",
            "https://app.aftercode.ai"
        ));
        assert!(!redirect_allowed(
            "http://evil.com",
            "http://localhost:8090"
        ));
        assert!(!redirect_allowed(
            "https://evil.com",
            "https://app.aftercode.ai"
        ));
    }

    async fn test_state() -> AppState {
        let cfg = crate::config::Config {
            database_url: std::env::var("DATABASE_URL").unwrap(),
            bind_addr: "127.0.0.1:0".into(),
            public_url: "http://localhost:8090".into(),
            llm_provider: "mock".into(),
            anthropic_api_key: None,
            openai_api_key: None,
            elevenlabs_api_key: None,
            host_voice_id: None,
            expert_voice_id: None,
            tts_provider: "mock".into(),
            openai_tts_model: "m".into(),
            openai_tts_voice_host: "alloy".into(),
            openai_tts_voice_expert: "onyx".into(),
            blob_store: "mock".into(),
            localfs_dir: "./data".into(),
            s3_bucket: None,
        };
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect(&cfg.database_url)
            .await
            .unwrap();
        AppState::for_test(db, cfg)
    }

    #[tokio::test]
    #[serial_test::serial(env)]
    async fn web_approve_redirects_with_token_in_fragment() {
        let app = router(test_state().await);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cli/approve")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("redirect=http://localhost:5173&state=s1"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let loc = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(loc.starts_with("http://localhost:5173#token=ak_"));
        assert!(loc.contains("&state=s1"));
    }

    #[tokio::test]
    #[serial_test::serial(env)]
    async fn web_approve_rejects_foreign_redirect() {
        let app = router(test_state().await);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cli/approve")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("redirect=http://evil.com&state=s1"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial_test::serial(env)]
    async fn loopback_approve_still_works() {
        let app = router(test_state().await);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cli/approve")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("port=4567&state=xyz"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let loc = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(loc.starts_with("http://127.0.0.1:4567/callback?token=ak_"));
    }
}
