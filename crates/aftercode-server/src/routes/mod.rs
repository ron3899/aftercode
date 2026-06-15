pub mod cli;
pub mod cli_auth;
pub mod episodes;
pub mod health;
pub mod projects;
pub mod sessions;

use crate::state::AppState;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

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

    app.layer(CorsLayer::very_permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
