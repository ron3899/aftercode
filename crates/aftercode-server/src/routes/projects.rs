use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

pub async fn me(AuthUser(uid): AuthUser) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "user_id": uid }))
}

#[derive(Deserialize)]
pub struct NewProject {
    pub name: String,
    #[serde(default)]
    pub default_language: Option<String>,
}

pub async fn create(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(p): Json<NewProject>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let lang = p.default_language.unwrap_or_else(|| "en".into());
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (user_id, name, default_language) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind(uid)
    .bind(&p.name)
    .bind(&lang)
    .fetch_one(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;
    Ok(Json(
        serde_json::json!({ "project_id": id, "project_name": p.name }),
    ))
}

pub async fn register(
    st: State<AppState>,
    user: AuthUser,
    body: Json<NewProject>,
) -> Result<Json<serde_json::Value>, ServerError> {
    create(st, user, body).await
}

pub async fn list(
    State(st): State<AppState>,
    AuthUser(uid): AuthUser,
) -> Result<Json<serde_json::Value>, ServerError> {
    let rows = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, name, default_language FROM projects WHERE user_id=$1 ORDER BY created_at DESC",
    )
    .bind(uid)
    .fetch_all(&st.db)
    .await
    .map_err(|e| ServerError::Other(e.into()))?;
    let items: Vec<_> = rows
        .into_iter()
        .map(|(id, name, lang)| serde_json::json!({ "id":id,"name":name,"default_language":lang }))
        .collect();
    Ok(Json(serde_json::json!({ "projects": items })))
}
