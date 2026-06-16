use crate::config::Config;
use crate::db::Db;
use crate::providers::llm::{LlmProvider, MockLlm};
use crate::providers::tts::{MockTts, TtsProvider};
use crate::storage::blob::{BlobStore, MockBlob};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub cfg: Config,
    pub llm: Arc<dyn LlmProvider>,
    pub tts: Arc<dyn TtsProvider>,
    pub blob: Arc<dyn BlobStore>,
}

impl AppState {
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        let db = crate::db::connect(&cfg.database_url).await?;
        let llm: Arc<dyn LlmProvider> = match cfg.llm_provider.as_str() {
            "mock" => Arc::new(MockLlm),
            "openai" => Arc::new(crate::providers::openai::OpenAiProvider::from_cfg(&cfg)?),
            _ => Arc::new(crate::providers::anthropic::AnthropicProvider::from_cfg(
                &cfg,
            )?),
        };
        let tts: Arc<dyn TtsProvider> = match cfg.tts_provider.as_str() {
            "openai" => Arc::new(crate::providers::tts::openai_tts_from_cfg(&cfg)?),
            "mock" => Arc::new(MockTts),
            "elevenlabs" => Arc::new(crate::providers::tts::eleven_from_cfg(&cfg)?),
            // Back-compat: fall back to ElevenLabs if its key is present, else mock.
            _ if cfg.elevenlabs_api_key.is_some() => {
                Arc::new(crate::providers::tts::eleven_from_cfg(&cfg)?)
            }
            _ => Arc::new(MockTts),
        };
        let blob: Arc<dyn BlobStore> = match cfg.blob_store.as_str() {
            "s3" => Arc::new(crate::storage::s3::S3Store::from_cfg(&cfg).await?),
            "mock" => Arc::new(MockBlob::default()),
            _ => Arc::new(crate::storage::localfs::LocalFs::from_cfg(&cfg)),
        };
        Ok(AppState {
            db,
            cfg,
            llm,
            tts,
            blob,
        })
    }

    /// Test constructor with injected mocks (no network).
    #[allow(dead_code)]
    pub fn for_test(db: Db, cfg: Config) -> Self {
        AppState {
            db,
            cfg,
            llm: Arc::new(MockLlm),
            tts: Arc::new(MockTts),
            blob: Arc::new(MockBlob::default()),
        }
    }
}
