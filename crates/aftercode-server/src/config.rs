#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub public_url: String,
    pub llm_provider: String, // "anthropic" | "openai" | "mock"
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub elevenlabs_api_key: Option<String>,
    pub host_voice_id: Option<String>,
    pub expert_voice_id: Option<String>,
    pub tts_provider: String, // "elevenlabs" | "openai" | "mock"
    pub openai_tts_model: String,
    pub openai_tts_voice_host: String,
    pub openai_tts_voice_expert: String,
    pub blob_store: String, // "localfs" | "s3" | "mock"
    pub localfs_dir: String,
    pub s3_bucket: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://aftercode.db?mode=rwc".into()),
            // Loopback by default: never exposed beyond this machine unless you
            // opt in (Docker sets 0.0.0.0 inside its own network boundary).
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            public_url: std::env::var("APP_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            llm_provider: std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            elevenlabs_api_key: std::env::var("ELEVENLABS_API_KEY").ok(),
            host_voice_id: std::env::var("ELEVENLABS_HOST_VOICE_ID").ok(),
            expert_voice_id: std::env::var("ELEVENLABS_EXPERT_VOICE_ID").ok(),
            tts_provider: std::env::var("TTS_PROVIDER").unwrap_or_else(|_| "elevenlabs".into()),
            openai_tts_model: std::env::var("OPENAI_TTS_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini-tts".into()),
            openai_tts_voice_host: std::env::var("OPENAI_TTS_VOICE_HOST")
                .unwrap_or_else(|_| "alloy".into()),
            openai_tts_voice_expert: std::env::var("OPENAI_TTS_VOICE_EXPERT")
                .unwrap_or_else(|_| "onyx".into()),
            blob_store: std::env::var("BLOB_STORE").unwrap_or_else(|_| "localfs".into()),
            localfs_dir: std::env::var("LOCALFS_DIR").unwrap_or_else(|_| "./data/audio".into()),
            s3_bucket: std::env::var("S3_BUCKET").ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[serial_test::serial(env)]
    fn database_url_defaults_to_sqlite() {
        let saved = std::env::var("DATABASE_URL").ok();
        std::env::remove_var("DATABASE_URL");
        let cfg = Config::from_env().unwrap();
        if let Some(v) = saved {
            std::env::set_var("DATABASE_URL", v);
        }
        assert!(cfg.database_url.starts_with("sqlite:"));
    }
}
