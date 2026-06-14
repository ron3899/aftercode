pub mod cli;
pub mod episodes;
pub mod health;
pub mod projects;
pub mod sessions;

use crate::state::AppState;
use axum::routing::{get, post};
use axum::Router;

pub fn router(state: AppState) -> Router {
    let static_dir = state.cfg.localfs_dir.clone();
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/me", get(projects::me))
        .route("/projects", post(projects::create).get(projects::list))
        .route("/cli/register-project", post(projects::register))
        .route("/cli/upload-session", post(sessions::upload))
        .route("/cli/generate-episode", post(cli::generate))
        .route("/cli/episode-status/:id", get(cli::status))
        .route("/episodes", get(episodes::list))
        .route("/episodes/:id", get(episodes::detail))
        .route("/episodes/:id/retry", post(episodes::retry))
        .nest_service("/static", tower_http::services::ServeDir::new(static_dir))
        .with_state(state)
}
