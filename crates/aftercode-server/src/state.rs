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

/// Build the LLM provider from an (effective) config.
pub fn build_llm(cfg: &Config) -> anyhow::Result<Arc<dyn LlmProvider>> {
    Ok(match cfg.llm_provider.as_str() {
        "mock" => Arc::new(MockLlm),
        "openai" => Arc::new(crate::providers::openai::OpenAiProvider::from_cfg(cfg)?),
        _ => Arc::new(crate::providers::anthropic::AnthropicProvider::from_cfg(
            cfg,
        )?),
    })
}

/// Build the TTS provider from an (effective) config.
pub fn build_tts(cfg: &Config) -> anyhow::Result<Arc<dyn TtsProvider>> {
    Ok(match cfg.tts_provider.as_str() {
        "openai" => Arc::new(crate::providers::tts::openai_tts_from_cfg(cfg)?),
        "local" => Arc::new(crate::providers::tts::local_tts_from_cfg(cfg)?),
        "mock" => Arc::new(MockTts),
        "elevenlabs" => Arc::new(crate::providers::tts::eleven_from_cfg(cfg)?),
        // Back-compat: fall back to ElevenLabs if its key is present, else mock.
        _ if cfg.elevenlabs_api_key.is_some() => {
            Arc::new(crate::providers::tts::eleven_from_cfg(cfg)?)
        }
        _ => Arc::new(MockTts),
    })
}

impl AppState {
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        let db = crate::db::connect(&cfg.database_url).await?;
        let llm = build_llm(&cfg)?;
        let tts = build_tts(&cfg)?;
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

    /// Resolve the providers for a generation job: DB settings (from the web UI)
    /// override env, built fresh per job so changes take effect without a
    /// restart. With no settings saved, use the startup providers (which are the
    /// injected mocks in tests).
    pub async fn resolve_providers(
        &self,
    ) -> anyhow::Result<(Arc<dyn LlmProvider>, Arc<dyn TtsProvider>)> {
        match crate::settings::load(&self.db).await? {
            Some(s) if s.is_configured() => {
                let eff = s.apply_to(self.cfg.clone());
                Ok((build_llm(&eff)?, build_tts(&eff)?))
            }
            _ => Ok((self.llm.clone(), self.tts.clone())),
        }
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
