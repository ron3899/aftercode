use crate::config::Config;
use aftercode_core::audio::{PcmAudio, VoiceRole, SAMPLE_RATE};
use aftercode_core::session::Language;
use async_trait::async_trait;

#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceRole,
        lang: Language,
    ) -> anyhow::Result<PcmAudio>;
}

pub struct MockTts;

#[async_trait]
impl TtsProvider for MockTts {
    async fn synthesize(
        &self,
        text: &str,
        _voice: VoiceRole,
        _lang: Language,
    ) -> anyhow::Result<PcmAudio> {
        // 50ms of audio per character, simple ramp so it's non-silent.
        let n = (SAMPLE_RATE as usize / 20) * text.len().max(1);
        let samples = (0..n).map(|i| ((i % 100) as i16 - 50) * 100).collect();
        Ok(PcmAudio { samples })
    }
}

pub struct ElevenLabsProvider {
    key: String,
    host_voice: String,
    expert_voice: String,
    http: reqwest::Client,
}

pub fn eleven_from_cfg(cfg: &Config) -> anyhow::Result<ElevenLabsProvider> {
    Ok(ElevenLabsProvider {
        key: cfg
            .elevenlabs_api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no eleven key"))?,
        host_voice: cfg
            .host_voice_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no host voice"))?,
        expert_voice: cfg
            .expert_voice_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no expert voice"))?,
        http: reqwest::Client::new(),
    })
}

#[async_trait]
impl TtsProvider for ElevenLabsProvider {
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceRole,
        _lang: Language,
    ) -> anyhow::Result<PcmAudio> {
        let vid = match voice {
            VoiceRole::Host => &self.host_voice,
            VoiceRole::Expert => &self.expert_voice,
        };
        // Request raw PCM 44.1kHz so we can concat without decoding.
        let url =
            format!("https://api.elevenlabs.io/v1/text-to-speech/{vid}?output_format=pcm_44100");
        let resp = self
            .http
            .post(&url)
            .header("xi-api-key", &self.key)
            .json(&serde_json::json!({ "text": text, "model_id": "eleven_multilingual_v2" }))
            .send()
            .await?
            .error_for_status()?;
        let bytes = resp.bytes().await?;
        // pcm_44100 is little-endian i16 mono.
        let samples = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(PcmAudio { samples })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_tts_non_empty() {
        let p = MockTts
            .synthesize("hi", VoiceRole::Host, Language::En)
            .await
            .unwrap();
        assert!(!p.samples.is_empty());
    }
}
