use crate::config::Config;
use aftercode_core::audio::{PcmAudio, VoiceRole, OPENAI_SAMPLE_RATE, SAMPLE_RATE};
use aftercode_core::session::Language;
use async_trait::async_trait;

#[async_trait]
pub trait TtsProvider: Send + Sync {
    /// Sample rate (Hz) of the PCM this provider returns. One provider is used
    /// per episode, so the assembler uses this rate for silence gaps + encoding.
    fn sample_rate(&self) -> u32;
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
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
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
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
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

pub struct OpenAiTts {
    key: String,
    model: String,
    host_voice: String,
    expert_voice: String,
    http: reqwest::Client,
}

pub fn openai_tts_from_cfg(cfg: &Config) -> anyhow::Result<OpenAiTts> {
    Ok(OpenAiTts {
        key: cfg
            .openai_api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY not set for TTS"))?,
        model: cfg.openai_tts_model.clone(),
        host_voice: cfg.openai_tts_voice_host.clone(),
        expert_voice: cfg.openai_tts_voice_expert.clone(),
        http: reqwest::Client::new(),
    })
}

#[async_trait]
impl TtsProvider for OpenAiTts {
    fn sample_rate(&self) -> u32 {
        // /v1/audio/speech with response_format=pcm returns 24kHz 16-bit mono LE.
        OPENAI_SAMPLE_RATE
    }
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceRole,
        _lang: Language,
    ) -> anyhow::Result<PcmAudio> {
        let v = match voice {
            VoiceRole::Host => &self.host_voice,
            VoiceRole::Expert => &self.expert_voice,
        };
        let resp = self
            .http
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.key)
            .json(&serde_json::json!({
                "model": self.model,
                "voice": v,
                "input": text,
                "response_format": "pcm"
            }))
            .send()
            .await?
            .error_for_status()?;
        let bytes = resp.bytes().await?;
        // pcm response is little-endian i16 mono at 24kHz.
        let samples = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(PcmAudio { samples })
    }
}

/// Local, offline TTS via a user-supplied command (e.g. F5-TTS, XTTS, Piper,
/// or any local voice-cloning runner). This is the BYO-engine escape hatch:
/// AfterCode generates the script + reasoning, and a fully local model speaks
/// it — including cloned voices — with no cloud API and no audio leaving the box.
///
/// The command is templated. These placeholders are substituted per segment:
///   `{text}`            — the segment text to speak
///   `{output}`          — path AfterCode expects a mono 16-bit PCM WAV written to
///   `{reference}`       — reference clip for the active voice role (clone source)
///   `{reference_text}`  — transcript of that reference clip (some engines need it)
///   `{role}`            — "host" or "expert"
///
/// Per-role reference clips let Host and Expert use two different cloned voices
/// from one provider. The runner must write a mono 16-bit PCM WAV at `{output}`
/// whose sample rate equals `LOCAL_TTS_SAMPLE_RATE` (default 24000).
pub struct LocalTts {
    command: String,
    args: Vec<String>,
    sample_rate: u32,
    host_reference: Option<String>,
    expert_reference: Option<String>,
    host_reference_text: Option<String>,
    expert_reference_text: Option<String>,
}

pub fn local_tts_from_cfg(cfg: &Config) -> anyhow::Result<LocalTts> {
    let command = cfg
        .local_tts_command
        .clone()
        .ok_or_else(|| anyhow::anyhow!("LOCAL_TTS_COMMAND not set for local TTS"))?;
    // Args are whitespace-split from LOCAL_TTS_ARGS; quote-free by design — keep
    // each placeholder as its own arg (e.g. `--text {text} --out {output}`).
    let args = cfg
        .local_tts_args
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    Ok(LocalTts {
        command,
        args,
        sample_rate: cfg.local_tts_sample_rate,
        host_reference: cfg.local_tts_host_reference.clone(),
        expert_reference: cfg.local_tts_expert_reference.clone(),
        host_reference_text: cfg.local_tts_host_reference_text.clone(),
        expert_reference_text: cfg.local_tts_expert_reference_text.clone(),
    })
}

fn expand(arg: &str, subs: &[(&str, &str)]) -> String {
    let mut out = arg.to_string();
    for (k, v) in subs {
        out = out.replace(k, v);
    }
    out
}

/// Decode a mono 16-bit PCM WAV file into `PcmAudio`. Minimal RIFF/WAVE reader
/// (PCM format 1, 16-bit) so we take no audio-decoding dependency.
fn read_wav_i16_mono(path: &std::path::Path) -> anyhow::Result<Vec<i16>> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        anyhow::bail!(
            "local TTS output is not a RIFF/WAVE file: {}",
            path.display()
        );
    }
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(bytes.len());
        if id == b"data" {
            return Ok(bytes[body_start..body_end]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect());
        }
        // Chunks are word-aligned (pad byte if size is odd).
        pos = body_start + size + (size & 1);
    }
    anyhow::bail!("no `data` chunk in local TTS WAV: {}", path.display())
}

#[async_trait]
impl TtsProvider for LocalTts {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    async fn synthesize(
        &self,
        text: &str,
        voice: VoiceRole,
        _lang: Language,
    ) -> anyhow::Result<PcmAudio> {
        let (role, reference, reference_text) = match voice {
            VoiceRole::Host => ("host", &self.host_reference, &self.host_reference_text),
            VoiceRole::Expert => (
                "expert",
                &self.expert_reference,
                &self.expert_reference_text,
            ),
        };
        let reference = reference.clone().unwrap_or_default();
        let reference_text = reference_text.clone().unwrap_or_default();

        // Unique output path per call so concurrent segments never collide.
        let out = std::env::temp_dir().join(format!(
            "aftercode-local-tts-{}-{}.wav",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let out_str = out.to_string_lossy().to_string();
        let subs: &[(&str, &str)] = &[
            ("{text}", text),
            ("{output}", &out_str),
            ("{reference}", &reference),
            ("{reference_text}", &reference_text),
            ("{role}", role),
        ];
        let args: Vec<String> = self.args.iter().map(|a| expand(a, subs)).collect();

        let status = tokio::process::Command::new(&self.command)
            .args(&args)
            .status()
            .await?;
        if !status.success() {
            let _ = std::fs::remove_file(&out);
            anyhow::bail!(
                "local TTS command `{}` exited with {}",
                self.command,
                status
            );
        }

        let samples = read_wav_i16_mono(&out)?;
        let _ = std::fs::remove_file(&out);
        if samples.is_empty() {
            anyhow::bail!("local TTS produced empty audio for role {role}");
        }
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
    #[test]
    fn provider_sample_rates() {
        assert_eq!(MockTts.sample_rate(), SAMPLE_RATE);
        let cfg = base_cfg();
        assert_eq!(
            openai_tts_from_cfg(&cfg).unwrap().sample_rate(),
            OPENAI_SAMPLE_RATE
        );
    }

    fn write_wav_i16_mono(path: &std::path::Path, sample_rate: u32, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(36 + data_len).to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // mono
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        buf.extend_from_slice(&2u16.to_le_bytes()); // block align
        buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            buf.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(path, buf).unwrap();
    }

    #[test]
    fn read_wav_round_trips() {
        let p =
            std::env::temp_dir().join(format!("aftercode-wav-test-{}.wav", uuid::Uuid::new_v4()));
        let samples = [0i16, 1000, -1000, 32767, -32768];
        write_wav_i16_mono(&p, 24000, &samples);
        let got = read_wav_i16_mono(&p).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!(got, samples);
    }

    #[tokio::test]
    async fn local_tts_invokes_command_and_reads_output() {
        // A tiny shell "runner" that ignores text and writes a fixed WAV to {output}.
        // It proves: command templating ({output}), exec, and WAV read-back all wire up.
        let helper =
            std::env::temp_dir().join(format!("aftercode-runner-{}.sh", uuid::Uuid::new_v4()));
        // Pre-bake a WAV the runner will copy into place.
        let src_wav =
            std::env::temp_dir().join(format!("aftercode-src-{}.wav", uuid::Uuid::new_v4()));
        write_wav_i16_mono(&src_wav, 24000, &[5i16; 64]);
        // Invoked as `/bin/sh <helper> <output>`, so inside the script $1 == {output}.
        std::fs::write(
            &helper,
            format!("#!/bin/sh\ncp '{}' \"$1\"\n", src_wav.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let provider = LocalTts {
            command: "/bin/sh".into(),
            // $1 = script path, $2 = output (helper copies src -> $2)
            args: vec![helper.to_string_lossy().to_string(), "{output}".into()],
            sample_rate: 24000,
            host_reference: None,
            expert_reference: None,
            host_reference_text: None,
            expert_reference_text: None,
        };
        let pcm = provider
            .synthesize("hello", VoiceRole::Host, Language::En)
            .await
            .unwrap();
        std::fs::remove_file(&helper).ok();
        std::fs::remove_file(&src_wav).ok();
        assert_eq!(provider.sample_rate(), 24000);
        assert_eq!(pcm.samples.len(), 64);
        assert!(pcm.samples.iter().all(|&s| s == 5));
    }

    #[test]
    fn local_tts_requires_command() {
        let mut cfg = base_cfg();
        cfg.local_tts_command = None;
        assert!(local_tts_from_cfg(&cfg).is_err());
        cfg.local_tts_command = Some("/bin/true".into());
        cfg.local_tts_sample_rate = 22050;
        assert_eq!(local_tts_from_cfg(&cfg).unwrap().sample_rate(), 22050);
    }

    fn base_cfg() -> Config {
        Config {
            database_url: "x".into(),
            bind_addr: "x".into(),
            public_url: "x".into(),
            llm_provider: "mock".into(),
            anthropic_api_key: None,
            openai_api_key: Some("k".into()),
            elevenlabs_api_key: None,
            host_voice_id: None,
            expert_voice_id: None,
            tts_provider: "local".into(),
            openai_tts_model: "gpt-4o-mini-tts".into(),
            openai_tts_voice_host: "alloy".into(),
            openai_tts_voice_expert: "onyx".into(),
            local_tts_command: None,
            local_tts_args: None,
            local_tts_sample_rate: 24000,
            local_tts_host_reference: None,
            local_tts_expert_reference: None,
            local_tts_host_reference_text: None,
            local_tts_expert_reference_text: None,
            blob_store: "mock".into(),
            localfs_dir: "x".into(),
            s3_bucket: None,
        }
    }
}
