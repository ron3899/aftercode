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
        local_tts_command: None,
        local_tts_args: None,
        local_tts_sample_rate: 24_000,
        local_tts_host_reference: None,
        local_tts_expert_reference: None,
        local_tts_host_reference_text: None,
        local_tts_expert_reference_text: None,
        blob_store: "mock".into(),
        localfs_dir: "./data".into(),
        s3_bucket: None,
    }
}
