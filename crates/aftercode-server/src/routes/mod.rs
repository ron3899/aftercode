pub mod cli;
pub mod cli_auth;
pub mod episodes;
pub mod health;
pub mod projects;
pub mod sessions;

use crate::state::AppState;
use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// CORS limited to the app's own origin + the local Vite dev server. The SPA is
/// served same-origin in prod (so it needs no CORS); only `vite dev` is cross-origin.
fn cors_layer(public_url: &str) -> CorsLayer {
    let mut origins: Vec<HeaderValue> = ["http://localhost:5173", "http://127.0.0.1:5173"]
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    if let Ok(v) = HeaderValue::from_str(public_url.trim_end_matches('/')) {
        origins.push(v);
    }
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

pub fn router(state: AppState) -> Router {
    let static_dir = state.cfg.localfs_dir.clone();
    // Web app: serve web/dist (or $WEB_DIST) as an SPA when built; otherwise
    // fall back to the minimal landing page so source checkouts still work.
    let web_dist = std::env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".into());
    let spa_built = std::path::Path::new(&web_dist).join("index.html").exists();

    let api = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/me", get(projects::me))
        .route("/cli/authorize", get(cli_auth::authorize))
        .route("/cli/approve", post(cli_auth::approve))
        .route("/projects", post(projects::create).get(projects::list))
        .route("/cli/register-project", post(projects::register))
        .route("/cli/upload-session", post(sessions::upload))
        .route("/cli/generate-episode", post(cli::generate))
        .route("/cli/episode-status/:id", get(cli::status))
        .route("/episodes", get(episodes::list))
        .route("/episodes/:id", get(episodes::detail))
        .route("/episodes/:id/retry", post(episodes::retry))
        .nest_service("/static", ServeDir::new(static_dir));

    let app = if spa_built {
        // SPA at / with client-side-routing fallback to index.html.
        let index = std::path::Path::new(&web_dist).join("index.html");
        api.fallback_service(ServeDir::new(&web_dist).fallback(ServeFile::new(index)))
    } else {
        api.route("/", get(health::landing))
    };

    let cors = cors_layer(&state.cfg.public_url);
    app.layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
