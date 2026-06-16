//! Test-only helpers: a fresh migrated SQLite pool + a mock Config.
#![cfg(test)]

use crate::config::Config;
use crate::db::Db;

/// A throwaway SQLite database (temp file) with migrations applied.
pub async fn pool() -> Db {
    let f = std::env::temp_dir().join(format!("aftercode-test-{}.db", uuid::Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", f.display());
    crate::db::connect(&url).await.unwrap()
}

pub fn cfg() -> Config {
    Config {
        database_url: "sqlite::memory:".into(),
        bind_addr: "127.0.0.1:0".into(),
        public_url: "http://localhost:8090".into(),
        llm_provider: "mock".into(),
        anthropic_api_key: None,
        openai_api_key: None,
        elevenlabs_api_key: None,
        host_voice_id: None,
        expert_voice_id: None,
        tts_provider: "mock".into(),
        openai_tts_model: "m".into(),
        openai_tts_voice_host: "alloy".into(),
        openai_tts_voice_expert: "onyx".into(),
        blob_store: "mock".into(),
        localfs_dir: "./data".into(),
        s3_bucket: None,
    }
}
