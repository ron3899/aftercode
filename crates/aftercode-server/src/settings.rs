//! Provider settings stored in the DB and configured from the web UI, so users
//! never have to edit `.env`. A single row (id = 1). Any NULL/empty field falls
//! back to the env-derived [`Config`] default.

use crate::config::Config;
use crate::db::Db;
use serde::{Deserialize, Serialize};

/// Raw settings row. Every field is optional; missing means "use the env default".
#[derive(Debug, Clone, Default, sqlx::FromRow, Deserialize)]
pub struct Settings {
    pub llm_provider: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub tts_provider: Option<String>,
    pub elevenlabs_api_key: Option<String>,
    pub elevenlabs_host_voice_id: Option<String>,
    pub elevenlabs_expert_voice_id: Option<String>,
    pub openai_tts_model: Option<String>,
    pub openai_tts_voice_host: Option<String>,
    pub openai_tts_voice_expert: Option<String>,
}

fn norm(o: &Option<String>) -> Option<String> {
    o.clone().filter(|v| !v.trim().is_empty())
}

/// Keep the patch value when it's a non-empty string, otherwise the current one.
/// This is how an untouched key field (sent empty by the UI) preserves the
/// stored secret instead of wiping it.
fn pick(patch: Option<String>, cur: Option<String>) -> Option<String> {
    match norm(&patch) {
        Some(v) => Some(v),
        None => cur,
    }
}

impl Settings {
    /// Override a [`Config`] with any set fields. Unset fields leave the env
    /// default in place — so a partial setup (e.g. only TTS) keeps the other
    /// side at its default (mock in the shipped image).
    pub fn apply_to(&self, mut cfg: Config) -> Config {
        if let Some(v) = norm(&self.llm_provider) {
            cfg.llm_provider = v;
        }
        if let Some(v) = norm(&self.anthropic_api_key) {
            cfg.anthropic_api_key = Some(v);
        }
        if let Some(v) = norm(&self.openai_api_key) {
            cfg.openai_api_key = Some(v);
        }
        if let Some(v) = norm(&self.tts_provider) {
            cfg.tts_provider = v;
        }
        if let Some(v) = norm(&self.elevenlabs_api_key) {
            cfg.elevenlabs_api_key = Some(v);
        }
        if let Some(v) = norm(&self.elevenlabs_host_voice_id) {
            cfg.host_voice_id = Some(v);
        }
        if let Some(v) = norm(&self.elevenlabs_expert_voice_id) {
            cfg.expert_voice_id = Some(v);
        }
        if let Some(v) = norm(&self.openai_tts_model) {
            cfg.openai_tts_model = v;
        }
        if let Some(v) = norm(&self.openai_tts_voice_host) {
            cfg.openai_tts_voice_host = v;
        }
        if let Some(v) = norm(&self.openai_tts_voice_expert) {
            cfg.openai_tts_voice_expert = v;
        }
        cfg
    }

    /// True once at least one real provider key has been set.
    pub fn is_configured(&self) -> bool {
        [
            &self.anthropic_api_key,
            &self.openai_api_key,
            &self.elevenlabs_api_key,
        ]
        .iter()
        .any(|o| norm(o).is_some())
    }

    /// Merge a patch (from PUT) onto the current row: non-empty fields win,
    /// empty/missing fields keep the stored value.
    fn merged_with(self, patch: Settings) -> Settings {
        Settings {
            llm_provider: pick(patch.llm_provider, self.llm_provider),
            anthropic_api_key: pick(patch.anthropic_api_key, self.anthropic_api_key),
            openai_api_key: pick(patch.openai_api_key, self.openai_api_key),
            tts_provider: pick(patch.tts_provider, self.tts_provider),
            elevenlabs_api_key: pick(patch.elevenlabs_api_key, self.elevenlabs_api_key),
            elevenlabs_host_voice_id: pick(
                patch.elevenlabs_host_voice_id,
                self.elevenlabs_host_voice_id,
            ),
            elevenlabs_expert_voice_id: pick(
                patch.elevenlabs_expert_voice_id,
                self.elevenlabs_expert_voice_id,
            ),
            openai_tts_model: pick(patch.openai_tts_model, self.openai_tts_model),
            openai_tts_voice_host: pick(patch.openai_tts_voice_host, self.openai_tts_voice_host),
            openai_tts_voice_expert: pick(
                patch.openai_tts_voice_expert,
                self.openai_tts_voice_expert,
            ),
        }
    }

    /// Masked view for the client — provider/voice/model values, but keys only
    /// as booleans. Raw secrets are never sent back.
    pub fn to_view(&self, cfg: &Config) -> SettingsView {
        SettingsView {
            llm_provider: norm(&self.llm_provider).unwrap_or_else(|| cfg.llm_provider.clone()),
            tts_provider: norm(&self.tts_provider).unwrap_or_else(|| cfg.tts_provider.clone()),
            anthropic_key_set: norm(&self.anthropic_api_key).is_some()
                || cfg.anthropic_api_key.is_some(),
            openai_key_set: norm(&self.openai_api_key).is_some() || cfg.openai_api_key.is_some(),
            elevenlabs_key_set: norm(&self.elevenlabs_api_key).is_some()
                || cfg.elevenlabs_api_key.is_some(),
            elevenlabs_host_voice_id: norm(&self.elevenlabs_host_voice_id)
                .or_else(|| cfg.host_voice_id.clone()),
            elevenlabs_expert_voice_id: norm(&self.elevenlabs_expert_voice_id)
                .or_else(|| cfg.expert_voice_id.clone()),
            openai_tts_model: norm(&self.openai_tts_model)
                .or_else(|| Some(cfg.openai_tts_model.clone())),
            openai_tts_voice_host: norm(&self.openai_tts_voice_host)
                .or_else(|| Some(cfg.openai_tts_voice_host.clone())),
            openai_tts_voice_expert: norm(&self.openai_tts_voice_expert)
                .or_else(|| Some(cfg.openai_tts_voice_expert.clone())),
        }
    }
}

/// Masked settings sent to the client (no raw secrets).
#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub llm_provider: String,
    pub tts_provider: String,
    pub anthropic_key_set: bool,
    pub openai_key_set: bool,
    pub elevenlabs_key_set: bool,
    pub elevenlabs_host_voice_id: Option<String>,
    pub elevenlabs_expert_voice_id: Option<String>,
    pub openai_tts_model: Option<String>,
    pub openai_tts_voice_host: Option<String>,
    pub openai_tts_voice_expert: Option<String>,
}

const COLS: &str = "llm_provider, anthropic_api_key, openai_api_key, tts_provider, \
    elevenlabs_api_key, elevenlabs_host_voice_id, elevenlabs_expert_voice_id, \
    openai_tts_model, openai_tts_voice_host, openai_tts_voice_expert";

pub async fn load(db: &Db) -> anyhow::Result<Option<Settings>> {
    let q = format!("SELECT {COLS} FROM settings WHERE id = 1");
    Ok(sqlx::query_as::<_, Settings>(&q).fetch_optional(db).await?)
}

/// Merge `patch` onto the stored row and upsert. Returns the merged result.
pub async fn save(db: &Db, patch: Settings) -> anyhow::Result<Settings> {
    let cur = load(db).await?.unwrap_or_default();
    let m = cur.merged_with(patch);
    sqlx::query(
        "INSERT INTO settings (id, llm_provider, anthropic_api_key, openai_api_key, tts_provider,
            elevenlabs_api_key, elevenlabs_host_voice_id, elevenlabs_expert_voice_id,
            openai_tts_model, openai_tts_voice_host, openai_tts_voice_expert, updated_at)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         ON CONFLICT(id) DO UPDATE SET
            llm_provider=excluded.llm_provider,
            anthropic_api_key=excluded.anthropic_api_key,
            openai_api_key=excluded.openai_api_key,
            tts_provider=excluded.tts_provider,
            elevenlabs_api_key=excluded.elevenlabs_api_key,
            elevenlabs_host_voice_id=excluded.elevenlabs_host_voice_id,
            elevenlabs_expert_voice_id=excluded.elevenlabs_expert_voice_id,
            openai_tts_model=excluded.openai_tts_model,
            openai_tts_voice_host=excluded.openai_tts_voice_host,
            openai_tts_voice_expert=excluded.openai_tts_voice_expert,
            updated_at=excluded.updated_at",
    )
    .bind(&m.llm_provider)
    .bind(&m.anthropic_api_key)
    .bind(&m.openai_api_key)
    .bind(&m.tts_provider)
    .bind(&m.elevenlabs_api_key)
    .bind(&m.elevenlabs_host_voice_id)
    .bind(&m.elevenlabs_expert_voice_id)
    .bind(&m.openai_tts_model)
    .bind(&m.openai_tts_voice_host)
    .bind(&m.openai_tts_voice_expert)
    .execute(db)
    .await?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_key_field_preserves_secret() {
        let cur = Settings {
            anthropic_api_key: Some("sk-old".into()),
            ..Default::default()
        };
        // Patch changes provider but leaves the key empty (untouched in the UI).
        let patch = Settings {
            llm_provider: Some("anthropic".into()),
            anthropic_api_key: Some("".into()),
            ..Default::default()
        };
        let m = cur.merged_with(patch);
        assert_eq!(m.anthropic_api_key.as_deref(), Some("sk-old"));
        assert_eq!(m.llm_provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn apply_overrides_only_set_fields() {
        let mut cfg = crate::config::Config::from_env().unwrap();
        cfg.llm_provider = "mock".into();
        cfg.tts_provider = "mock".into();
        let s = Settings {
            tts_provider: Some("elevenlabs".into()),
            elevenlabs_api_key: Some("el-key".into()),
            ..Default::default()
        };
        let eff = s.apply_to(cfg);
        assert_eq!(eff.tts_provider, "elevenlabs");
        assert_eq!(eff.elevenlabs_api_key.as_deref(), Some("el-key"));
        // LLM untouched -> stays mock.
        assert_eq!(eff.llm_provider, "mock");
    }

    #[test]
    fn view_masks_keys() {
        let cfg = crate::config::Config::from_env().unwrap();
        let s = Settings {
            openai_api_key: Some("sk-secret".into()),
            ..Default::default()
        };
        let v = s.to_view(&cfg);
        assert!(v.openai_key_set);
        // SettingsView has no raw-key field at all; serialize and confirm.
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("sk-secret"));
    }
}
