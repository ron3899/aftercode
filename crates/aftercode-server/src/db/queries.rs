use super::Db;
use uuid::Uuid;

pub async fn user_by_token_hash(db: &Db, hash: &str) -> anyhow::Result<Option<Uuid>> {
    let row = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE token_hash = ?")
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
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO podcast_episodes (id, user_id, project_id, session_id, language)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user)
    .bind(project)
    .bind(session)
    .bind(lang)
    .execute(db)
    .await?;
    Ok(id)
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE podcast_episodes
         SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?",
    )
    .bind(status)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}
