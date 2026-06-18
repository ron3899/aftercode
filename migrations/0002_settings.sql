-- Provider settings configured from the web UI, so users don't edit .env.
-- Single row (id = 1). NULL means "fall back to the env/Config default".
CREATE TABLE IF NOT EXISTS settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  llm_provider TEXT,
  anthropic_api_key TEXT,
  openai_api_key TEXT,
  tts_provider TEXT,
  elevenlabs_api_key TEXT,
  elevenlabs_host_voice_id TEXT,
  elevenlabs_expert_voice_id TEXT,
  openai_tts_model TEXT,
  openai_tts_voice_host TEXT,
  openai_tts_voice_expert TEXT,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
