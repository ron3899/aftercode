use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use axum::extract::State;
use axum::Json;
use uuid::Uuid;

pub async fn upload(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(ctx): Json<SessionContext>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let project = Uuid::parse_str(&ctx.project_id)
        .map_err(|_| ServerError::BadRequest("project_id must be a uuid".into()))?;
    let ctx_json = serde_json::to_value(&ctx).map_err(|e| ServerError::Other(e.into()))?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO coding_sessions (id, project_id, user_id, source, context_json)
         VALUES (?, ?, ?, 'cli', ?)",
    )
    .bind(id)
    .bind(project)
    .bind(uid)
    .bind(ctx_json)
    .execute(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;
    Ok(Json(serde_json::json!({ "session_id": id })))
}
