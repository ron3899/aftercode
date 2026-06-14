use crate::db::queries;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

pub fn hash_token(token: &str) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(digest)
}

pub struct AuthUser(pub Uuid);

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ServerError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ServerError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(ServerError::Unauthorized)?;
        let hash = hash_token(token);
        let uid = queries::user_by_token_hash(&state.db, &hash)
            .await
            .map_err(ServerError::Other)?
            .ok_or(ServerError::Unauthorized)?;
        Ok(AuthUser(uid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_is_stable_and_nonempty() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert!(!hash_token("abc").is_empty());
    }
}
