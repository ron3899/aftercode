use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            ServerError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ServerError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ServerError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServerError::Other(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (code, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
