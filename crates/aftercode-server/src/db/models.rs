use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub token_hash: String,
}
