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
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO coding_sessions (project_id, user_id, source, context_json)
         VALUES ($1,$2,'cli',$3) RETURNING id",
    )
    .bind(project)
    .bind(uid)
    .bind(ctx_json)
    .fetch_one(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;
    Ok(Json(serde_json::json!({ "session_id": id })))
}
