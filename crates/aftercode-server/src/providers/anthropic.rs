use super::llm::{LlmProvider, NormalizedContext, ScriptOpts};
use crate::config::Config;
use aftercode_core::episode::{EpisodeScript, LearningTopic};
use aftercode_core::session::Language;
use async_trait::async_trait;

pub struct AnthropicProvider {
    key: String,
    http: reqwest::Client,
}

impl AnthropicProvider {
    pub fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let key = cfg
            .anthropic_api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
        Ok(Self {
            key,
            http: reqwest::Client::new(),
        })
    }

    async fn call_json(
        &self,
        system: &str,
        user: &str,
        schema: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({
            "model": "claude-opus-4-8",
            "max_tokens": 8000,
            "thinking": { "type": "adaptive" },
            "output_config": { "format": { "type": "json_schema", "schema": schema } },
            "system": system,
            "messages": [{ "role": "user", "content": user }]
        });
        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        let text = v["content"]
            .as_array()
            .and_then(|a| a.iter().find(|b| b["type"] == "text"))
            .and_then(|b| b["text"].as_str())
            .ok_or_else(|| anyhow::anyhow!("no text block in response"))?;
        Ok(serde_json::from_str(text)?)
    }
}

fn topics_schema() -> serde_json::Value {
    serde_json::json!({
        "type":"object","additionalProperties":false,
        "properties":{"topics":{"type":"array","items":{
            "type":"object","additionalProperties":false,
            "properties":{"title":{"type":"string"},"summary":{"type":"string"},
                "evidence":{"type":"array","items":{"type":"string"}},
                "knowledge_gap":{"type":"string"},"difficulty":{"type":"string"},
                "priority":{"type":"string"}},
            "required":["title","summary","evidence","knowledge_gap","difficulty","priority"]}}},
        "required":["topics"]})
}

fn script_schema() -> serde_json::Value {
    serde_json::json!({
        "type":"object","additionalProperties":false,
        "properties":{"title":{"type":"string"},"language":{"type":"string"},
            "segments":{"type":"array","items":{"type":"object","additionalProperties":false,
                "properties":{"speaker":{"type":"string","enum":["host","expert"]},
                    "text":{"type":"string"}},"required":["speaker","text"]}},
            "summary_points":{"type":"array","items":{"type":"string"}},
            "quiz":{"type":"object","additionalProperties":false,
                "properties":{"question":{"type":"string"},"answer":{"type":"string"}},
                "required":["question","answer"]}},
        "required":["title","language","segments","summary_points"]})
}

fn lang_name(l: Language) -> &'static str {
    match l {
        Language::He => "Hebrew",
        Language::En => "English",
    }
}

fn script_system(l: Language) -> String {
    match l {
        Language::He => "אתה כותב פודקאסט טכני בעברית בין מנחה (host) למומחה (expert). \
            דבר טבעי כמו מפתחים ישראלים, השאר מונחים טכניים באנגלית כשטבעי, הימנע מעברית פורמלית מדי. \
            החזר JSON בלבד."
            .to_string(),
        Language::En => "You write a technical two-speaker podcast (host + expert). \
            Calm mentor tone, practical, not cheesy. Return JSON only."
            .to_string(),
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        let system = "Extract deep technical learning topics from a coding session. \
            Every topic must cite evidence from the provided context. Return JSON only.";
        let user = format!(
            "Coding session ({} min target, {}):\n\n{}",
            ctx.minutes,
            lang_name(ctx.language),
            ctx.text
        );
        let v = self.call_json(system, &user, topics_schema()).await?;
        Ok(serde_json::from_value(v["topics"].clone())?)
    }
    async fn write_script(
        &self,
        topics: &[LearningTopic],
        opts: &ScriptOpts,
    ) -> anyhow::Result<EpisodeScript> {
        let user = format!(
            "Write a {}-minute episode in {} about these topics:\n{}",
            opts.minutes,
            lang_name(opts.language),
            serde_json::to_string_pretty(topics)?
        );
        let v = self
            .call_json(&script_system(opts.language), &user, script_schema())
            .await?;
        Ok(serde_json::from_value(v)?)
    }
}
