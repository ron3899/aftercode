use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GenerateReq {
    pub project_id: String,
    pub session_id: Option<String>,
    pub session_context: Option<SessionContext>,
    pub language: Option<String>,
}

pub async fn generate(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<GenerateReq>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let project = Uuid::parse_str(&req.project_id)
        .map_err(|_| ServerError::BadRequest("project_id must be uuid".into()))?;

    // Resolve a session: use provided session_id or persist the inline context.
    let (session_id, ctx) = if let Some(sid) = req.session_id {
        let sid =
            Uuid::parse_str(&sid).map_err(|_| ServerError::BadRequest("bad session_id".into()))?;
        let json: serde_json::Value =
            sqlx::query_scalar("SELECT context_json FROM coding_sessions WHERE id=$1")
                .bind(sid)
                .fetch_optional(&st.db)
                .await
                .map_err(|e| ServerError::Other(e.into()))?
                .ok_or(ServerError::NotFound)?;
        (
            sid,
            serde_json::from_value::<SessionContext>(json)
                .map_err(|e| ServerError::Other(e.into()))?,
        )
    } else {
        let ctx = req.session_context.ok_or(ServerError::BadRequest(
            "need session_id or session_context".into(),
        ))?;
        let json = serde_json::to_value(&ctx).map_err(|e| ServerError::Other(e.into()))?;
        let sid = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO coding_sessions (project_id,user_id,source,context_json)
             VALUES ($1,$2,'cli',$3) RETURNING id",
        )
        .bind(project)
        .bind(uid)
        .bind(json)
        .fetch_one(&st.db)
        .await
        .map_err(|e| ServerError::Other(e.into()))?;
        (sid, ctx)
    };

    let lang = req.language.unwrap_or_else(|| match ctx.language {
        aftercode_core::session::Language::He => "he".into(),
        aftercode_core::session::Language::En => "en".into(),
    });
    let episode_id = crate::db::queries::insert_episode(&st.db, uid, project, session_id, &lang)
        .await
        .map_err(ServerError::Other)?;
    crate::worker::spawn(st.clone(), episode_id, ctx);
    Ok(Json(
        serde_json::json!({ "episode_id": episode_id, "status": "queued" }),
    ))
}

pub async fn status(
    State(st): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT status::text, title, error FROM podcast_episodes WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?
    .ok_or(ServerError::NotFound)?;
    Ok(Json(
        serde_json::json!({ "episode_id": id, "status": row.0, "title": row.1, "error": row.2 }),
    ))
}
