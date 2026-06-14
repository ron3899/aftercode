use super::llm::{LlmProvider, NormalizedContext, ScriptOpts};
use crate::config::Config;
use aftercode_core::episode::{EpisodeScript, LearningTopic};
use async_trait::async_trait;

pub struct OpenAiProvider {
    key: String,
    http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let key = cfg
            .openai_api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        Ok(Self {
            key,
            http: reqwest::Client::new(),
        })
    }
    async fn call_json(&self, system: &str, user: &str) -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "response_format": { "type": "json_object" },
            "messages": [{ "role":"system","content":system },{ "role":"user","content":user }]
        });
        let resp = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no content"))?;
        Ok(serde_json::from_str(text)?)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        let user = format!(
            "Return JSON {{\"topics\":[...]}} with fields title, summary, \
            evidence[], knowledge_gap, difficulty, priority. Session:\n{}",
            ctx.text
        );
        let v = self
            .call_json("Extract evidence-based technical topics. JSON only.", &user)
            .await?;
        Ok(serde_json::from_value(v["topics"].clone())?)
    }
    async fn write_script(
        &self,
        topics: &[LearningTopic],
        opts: &ScriptOpts,
    ) -> anyhow::Result<EpisodeScript> {
        let user = format!(
            "Return JSON with title, language, segments[(speaker host|expert, text)], \
            summary_points[], quiz{{question,answer}}. {} minutes. Topics:\n{}",
            opts.minutes,
            serde_json::to_string(topics)?
        );
        let v = self
            .call_json("Two-speaker technical podcast. JSON only.", &user)
            .await?;
        Ok(serde_json::from_value(v)?)
    }
}
