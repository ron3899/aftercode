use super::Db;
use uuid::Uuid;

pub async fn user_by_token_hash(db: &Db, hash: &str) -> anyhow::Result<Option<Uuid>> {
    let row = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE token_hash = $1")
        .bind(hash)
        .fetch_optional(db)
        .await?;
    Ok(row)
}

pub async fn insert_episode(
    db: &Db,
    user: Uuid,
    project: Uuid,
    session: Uuid,
    lang: &str,
) -> anyhow::Result<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO podcast_episodes (user_id, project_id, session_id, language)
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(user)
    .bind(project)
    .bind(session)
    .bind(lang)
    .fetch_one(db)
    .await?;
    Ok(id)
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE podcast_episodes SET status = $1::episode_status, updated_at = now() WHERE id = $2",
    )
    .bind(status)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}
