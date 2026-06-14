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
    pub blob_store: String, // "localfs" | "s3" | "mock"
    pub localfs_dir: String,
    pub s3_bucket: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        fn req(k: &str) -> anyhow::Result<String> {
            std::env::var(k).map_err(|_| anyhow::anyhow!("missing env {k}"))
        }
        Ok(Config {
            database_url: req("DATABASE_URL")?,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            public_url: std::env::var("APP_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            llm_provider: std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            elevenlabs_api_key: std::env::var("ELEVENLABS_API_KEY").ok(),
            host_voice_id: std::env::var("ELEVENLABS_HOST_VOICE_ID").ok(),
            expert_voice_id: std::env::var("ELEVENLABS_EXPERT_VOICE_ID").ok(),
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
    fn from_env_requires_database_url() {
        let saved = std::env::var("DATABASE_URL").ok();
        std::env::remove_var("DATABASE_URL");
        let result = Config::from_env();
        if let Some(v) = saved {
            std::env::set_var("DATABASE_URL", v);
        }
        assert!(result.is_err());
    }
}
