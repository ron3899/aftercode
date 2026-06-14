# Aftercode Plan A — Core Crate + API Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `aftercode-core` shared crate and the `aftercode-server` backend that turns an uploaded coding session into a two-speaker podcast episode (topics → script → ElevenLabs audio → stored MP3), runnable end-to-end against mocked providers.

**Architecture:** Cargo workspace. `aftercode-core` holds the wire types (no I/O). `aftercode-server` is Axum + sqlx(Postgres) + Tokio with three swappable trait families (`LlmProvider`, `TtsProvider`, `BlobStore`) and an in-process Tokio worker that drives a status machine. Audio is assembled in pure Rust (PCM concat + silence + LAME encode).

**Tech Stack:** Rust, Cargo workspace, clap (CLI in Plan B), Axum, sqlx, Tokio, reqwest, serde, thiserror, async-trait, mp3lame-encoder, anthropic/openai via reqwest, ElevenLabs via reqwest, aws-sdk-s3.

---

## File Structure

```
Cargo.toml                                  # workspace
crates/aftercode-core/
  Cargo.toml
  src/lib.rs                                # re-exports
  src/session.rs                            # SessionContext, CodingEvent, EventType, Language
  src/episode.rs                            # LearningTopic, EpisodeScript, ScriptSegment, Speaker, EpisodeStatus, DTOs
  src/audio.rs                              # PcmAudio, VoiceRole, gap constants
  src/error.rs                              # CoreError
crates/aftercode-server/
  Cargo.toml
  src/main.rs                               # bootstrap
  src/config.rs                             # env config
  src/state.rs                              # AppState (pool + providers + blob + config)
  src/error.rs                              # ServerError + IntoResponse
  src/auth.rs                               # bearer extractor
  src/db/mod.rs, src/db/models.rs, src/db/queries.rs
  src/providers/llm.rs                      # LlmProvider trait + MockLlm
  src/providers/anthropic.rs
  src/providers/openai.rs
  src/providers/tts.rs                      # TtsProvider trait + ElevenLabsProvider + MockTts
  src/storage/blob.rs                       # BlobStore trait + MockBlob
  src/storage/localfs.rs
  src/storage/s3.rs
  src/pipeline/mod.rs                       # run_pipeline
  src/pipeline/normalize.rs
  src/pipeline/rank.rs
  src/pipeline/assemble.rs                  # PCM concat + silence + LAME
  src/routes/mod.rs, health.rs, projects.rs, sessions.rs, episodes.rs, cli.rs
  src/worker.rs                             # spawn + status transitions
migrations/0001_init.sql
.env.example
```

---

## Task 1: Workspace skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `crates/aftercode-core/Cargo.toml`
- Create: `crates/aftercode-core/src/lib.rs`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Create workspace manifest**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/aftercode-core", "crates/aftercode-server", "crates/aftercode-cli"]

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
anyhow = "1"
```

- [ ] **Step 2: Create core crate manifest**

`crates/aftercode-core/Cargo.toml`:
```toml
[package]
name = "aftercode-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

- [ ] **Step 3: Stub lib + pin toolchain**

`crates/aftercode-core/src/lib.rs`:
```rust
//! Shared types for Aftercode (no I/O).
```
`rust-toolchain.toml`:
```toml
[toolchain]
channel = "stable"
```

Create a placeholder so the workspace resolves before Plan B exists:
`crates/aftercode-cli/Cargo.toml`:
```toml
[package]
name = "aftercode-cli"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "aftercode"
path = "src/main.rs"
```
`crates/aftercode-cli/src/main.rs`:
```rust
fn main() { println!("aftercode cli — see Plan B"); }
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build -p aftercode-core`
Expected: compiles, no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates rust-toolchain.toml
git commit -m "chore: cargo workspace skeleton"
```

---

## Task 2: Core — Language + EventType + SessionContext

**Files:**
- Create: `crates/aftercode-core/src/session.rs`
- Test: in-file `#[cfg(test)]` module

- [ ] **Step 1: Write failing test**

`crates/aftercode-core/src/session.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language { He, En }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType { UserPrompt, AgentResponse, FileChanged, TerminalError, GitDiff, Commit }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingEvent {
    pub event_type: EventType,
    pub timestamp: String,
    pub content: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub project_id: String,
    pub language: Language,
    pub episode_length_minutes: u8,
    pub collected_at: String,
    #[serde(default)]
    pub events: Vec<CodingEvent>,
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub git_diff_summary: Option<String>,
    #[serde(default)]
    pub commit_messages: Vec<String>,
    #[serde(default)]
    pub terminal_errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Language::He).unwrap(), "\"he\"");
    }

    #[test]
    fn session_roundtrips() {
        let s = SessionContext {
            project_id: "proj_1".into(),
            language: Language::En,
            episode_length_minutes: 10,
            collected_at: "2026-06-14T19:00:00Z".into(),
            events: vec![],
            changed_files: vec!["a.rs".into()],
            git_diff_summary: Some("x".into()),
            commit_messages: vec![],
            terminal_errors: vec![],
        };
        let j = serde_json::to_string(&s).unwrap();
        let back: SessionContext = serde_json::from_str(&j).unwrap();
        assert_eq!(back.project_id, "proj_1");
        assert_eq!(back.episode_length_minutes, 10);
    }
}
```
Add to `lib.rs`: `pub mod session;`

- [ ] **Step 2: Run test, verify it passes (types compile + serde works)**

Run: `cargo test -p aftercode-core session`
Expected: 2 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-core
git commit -m "feat(core): session context types"
```

---

## Task 3: Core — episode/topic/script types

**Files:**
- Create: `crates/aftercode-core/src/episode.rs`

- [ ] **Step 1: Write types + failing test**

`crates/aftercode-core/src/episode.rs`:
```rust
use crate::session::Language;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearningTopic {
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub knowledge_gap: String,
    pub difficulty: String,
    pub priority: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Speaker { Host, Expert }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptSegment { pub speaker: Speaker, pub text: String }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quiz { pub question: String, pub answer: String }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeScript {
    pub title: String,
    pub language: Language,
    pub segments: Vec<ScriptSegment>,
    pub summary_points: Vec<String>,
    pub quiz: Option<Quiz>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeStatus {
    Queued, ExtractingTopics, WritingScript, GeneratingAudio, Ready, Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSummary {
    pub id: String,
    pub title: String,
    pub project_name: String,
    pub language: Language,
    pub status: EpisodeStatus,
    pub duration_seconds: Option<i32>,
    pub topics: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeDetail {
    pub id: String,
    pub title: String,
    pub language: Language,
    pub status: EpisodeStatus,
    pub audio_url: Option<String>,
    pub duration_seconds: Option<i32>,
    pub summary: Option<String>,
    pub transcript_text: Option<String>,
    pub topics: Vec<LearningTopic>,
    pub script: Option<EpisodeScript>,
    pub error: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_snake_case() {
        assert_eq!(serde_json::to_string(&EpisodeStatus::ExtractingTopics).unwrap(),
                   "\"extracting_topics\"");
    }
    #[test]
    fn speaker_lowercase() {
        assert_eq!(serde_json::to_string(&Speaker::Expert).unwrap(), "\"expert\"");
    }
}
```
Add to `lib.rs`: `pub mod episode;`

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-core episode`
Expected: 2 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-core && git commit -m "feat(core): episode, topic, script types"
```

---

## Task 4: Core — audio + error types

**Files:**
- Create: `crates/aftercode-core/src/audio.rs`
- Create: `crates/aftercode-core/src/error.rs`

- [ ] **Step 1: Write types + failing test**

`crates/aftercode-core/src/audio.rs`:
```rust
use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoiceRole { Host, Expert }

/// Mono i16 PCM at SAMPLE_RATE.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmAudio { pub samples: Vec<i16> }

impl PcmAudio {
    pub fn silence(ms: u32) -> Self {
        let n = (SAMPLE_RATE as u64 * ms as u64 / 1000) as usize;
        PcmAudio { samples: vec![0i16; n] }
    }
    pub fn duration_seconds(&self) -> f32 {
        self.samples.len() as f32 / SAMPLE_RATE as f32
    }
}

/// Pause lengths in ms (PRD §14).
pub const GAP_SAME_SPEAKER_MS: u32 = 300;
pub const GAP_SPEAKER_SWITCH_MS: u32 = 600;
pub const GAP_SECTION_MS: u32 = 1000;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_length_matches_rate() {
        assert_eq!(PcmAudio::silence(1000).samples.len(), SAMPLE_RATE as usize);
        assert_eq!(PcmAudio::silence(500).samples.len(), (SAMPLE_RATE / 2) as usize);
    }
}
```

`crates/aftercode-core/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid value: {0}")]
    Invalid(String),
}
```
Add to `lib.rs`: `pub mod audio;` and `pub mod error;`

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-core audio`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-core && git commit -m "feat(core): audio + error types"
```

---

## Task 5: Server crate skeleton + config

**Files:**
- Create: `crates/aftercode-server/Cargo.toml`
- Create: `crates/aftercode-server/src/main.rs`
- Create: `crates/aftercode-server/src/config.rs`
- Create: `.env.example`

- [ ] **Step 1: Server manifest**

`crates/aftercode-server/Cargo.toml`:
```toml
[package]
name = "aftercode-server"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "aftercode-server"
path = "src/main.rs"

[dependencies]
aftercode-core = { path = "../aftercode-core" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
async-trait.workspace = true
reqwest.workspace = true
anyhow.workspace = true
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "json", "uuid", "chrono", "macros"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
mp3lame-encoder = "0.2"
aws-sdk-s3 = "1"
aws-config = "1"
base64 = "0.22"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

- [ ] **Step 2: Config + failing test**

`crates/aftercode-server/src/config.rs`:
```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub public_url: String,
    pub llm_provider: String,    // "anthropic" | "openai" | "mock"
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub elevenlabs_api_key: Option<String>,
    pub host_voice_id: Option<String>,
    pub expert_voice_id: Option<String>,
    pub blob_store: String,      // "localfs" | "s3" | "mock"
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
            public_url: std::env::var("APP_PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8080".into()),
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
    fn from_env_requires_database_url() {
        std::env::remove_var("DATABASE_URL");
        assert!(Config::from_env().is_err());
    }
}
```

- [ ] **Step 3: main.rs stub**

`crates/aftercode-server/src/main.rs`:
```rust
mod config;
mod state;
mod error;
mod auth;
mod db;
mod providers;
mod storage;
mod pipeline;
mod routes;
mod worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = config::Config::from_env()?;
    let state = state::AppState::new(cfg.clone()).await?;
    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 4: `.env.example`**

`.env.example`:
```
DATABASE_URL=postgres://aftercode:aftercode@localhost:5432/aftercode
BIND_ADDR=0.0.0.0:8080
APP_PUBLIC_URL=http://localhost:8080
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=
OPENAI_API_KEY=
ELEVENLABS_API_KEY=
ELEVENLABS_HOST_VOICE_ID=
ELEVENLABS_EXPERT_VOICE_ID=
BLOB_STORE=localfs
LOCALFS_DIR=./data/audio
S3_BUCKET=
```

- [ ] **Step 5: Run config test (other modules are stubbed next task — temporarily comment unbuilt mods if needed)**

Run: `cargo test -p aftercode-server config::`
Expected: 1 passed. (If module-not-found errors, proceed to Task 6 which creates them, then re-run.)

- [ ] **Step 6: Commit**

```bash
git add crates/aftercode-server .env.example
git commit -m "feat(server): crate skeleton + env config"
```

---

## Task 6: Provider + storage traits with mocks

**Files:**
- Create: `crates/aftercode-server/src/providers/llm.rs`
- Create: `crates/aftercode-server/src/providers/tts.rs`
- Create: `crates/aftercode-server/src/storage/blob.rs`
- Create: `crates/aftercode-server/src/providers/mod.rs`, `src/storage/mod.rs`

- [ ] **Step 1: Define traits + mocks + failing test**

`crates/aftercode-server/src/providers/llm.rs`:
```rust
use aftercode_core::episode::{EpisodeScript, LearningTopic};
use aftercode_core::session::Language;
use async_trait::async_trait;

pub struct NormalizedContext { pub text: String, pub language: Language, pub minutes: u8 }
pub struct ScriptOpts { pub language: Language, pub minutes: u8 }

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>>;
    async fn write_script(&self, topics: &[LearningTopic], opts: &ScriptOpts) -> anyhow::Result<EpisodeScript>;
}

pub struct MockLlm;

#[async_trait]
impl LlmProvider for MockLlm {
    async fn extract_topics(&self, _ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        Ok(vec![LearningTopic {
            title: "Production-safe Postgres indexes".into(),
            summary: "Index built concurrently to avoid locking.".into(),
            evidence: vec!["postgresql_concurrently=True".into()],
            knowledge_gap: "Why CONCURRENTLY can't run in a txn.".into(),
            difficulty: "intermediate".into(),
            priority: "high".into(),
        }])
    }
    async fn write_script(&self, topics: &[LearningTopic], opts: &ScriptOpts) -> anyhow::Result<EpisodeScript> {
        use aftercode_core::episode::{ScriptSegment, Speaker, Quiz};
        Ok(EpisodeScript {
            title: format!("Why your migration matters: {}", topics[0].title),
            language: opts.language,
            segments: vec![
                ScriptSegment { speaker: Speaker::Host, text: "Today we unpack your session.".into() },
                ScriptSegment { speaker: Speaker::Expert, text: "Index creation can lock tables.".into() },
            ],
            summary_points: vec!["CONCURRENTLY reduces locking.".into()],
            quiz: Some(Quiz { question: "Why?".into(), answer: "Outside a transaction.".into() }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_llm_produces_topic_and_script() {
        let m = MockLlm;
        let ctx = NormalizedContext { text: "x".into(), language: Language::En, minutes: 10 };
        let topics = m.extract_topics(&ctx).await.unwrap();
        assert_eq!(topics.len(), 1);
        let script = m.write_script(&topics, &ScriptOpts { language: Language::En, minutes: 10 }).await.unwrap();
        assert_eq!(script.segments.len(), 2);
    }
}
```

`crates/aftercode-server/src/providers/tts.rs`:
```rust
use aftercode_core::audio::{PcmAudio, VoiceRole, SAMPLE_RATE};
use aftercode_core::session::Language;
use async_trait::async_trait;

#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(&self, text: &str, voice: VoiceRole, lang: Language) -> anyhow::Result<PcmAudio>;
}

pub struct MockTts;

#[async_trait]
impl TtsProvider for MockTts {
    async fn synthesize(&self, text: &str, _voice: VoiceRole, _lang: Language) -> anyhow::Result<PcmAudio> {
        // 50ms of audio per character, simple ramp so it's non-silent.
        let n = (SAMPLE_RATE as usize / 20) * text.len().max(1);
        let samples = (0..n).map(|i| ((i % 100) as i16 - 50) * 100).collect();
        Ok(PcmAudio { samples })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_tts_non_empty() {
        let p = MockTts.synthesize("hi", VoiceRole::Host, Language::En).await.unwrap();
        assert!(!p.samples.is_empty());
    }
}
```

`crates/aftercode-server/src/storage/blob.rs`:
```rust
use async_trait::async_trait;
use std::sync::Mutex;

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> anyhow::Result<String>;
}

#[derive(Default)]
pub struct MockBlob { pub puts: Mutex<Vec<(String, usize)>> }

#[async_trait]
impl BlobStore for MockBlob {
    async fn put(&self, key: &str, bytes: Vec<u8>, _ct: &str) -> anyhow::Result<String> {
        self.puts.lock().unwrap().push((key.to_string(), bytes.len()));
        Ok(format!("mock://{key}"))
    }
}
```

`crates/aftercode-server/src/providers/mod.rs`:
```rust
pub mod llm;
pub mod tts;
pub mod anthropic;
pub mod openai;
```
`crates/aftercode-server/src/storage/mod.rs`:
```rust
pub mod blob;
pub mod localfs;
pub mod s3;
```

- [ ] **Step 2: Create empty real-provider stubs so modules compile**

`crates/aftercode-server/src/providers/anthropic.rs`, `openai.rs`, `storage/localfs.rs`, `storage/s3.rs` each: `// implemented in later task` plus a `#[allow(dead_code)] pub struct Placeholder;`

- [ ] **Step 3: Run tests**

Run: `cargo test -p aftercode-server providers::`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/aftercode-server/src
git commit -m "feat(server): provider/storage traits + mocks"
```

---

## Task 7: Audio assembly (PCM concat + silence + LAME)

**Files:**
- Create: `crates/aftercode-server/src/pipeline/assemble.rs`
- Create: `crates/aftercode-server/src/pipeline/mod.rs`

- [ ] **Step 1: Write failing test**

`crates/aftercode-server/src/pipeline/assemble.rs`:
```rust
use aftercode_core::audio::{PcmAudio, Speaker as _, GAP_SAME_SPEAKER_MS, GAP_SPEAKER_SWITCH_MS, SAMPLE_RATE};
use aftercode_core::episode::Speaker;

/// One synthesized segment plus the speaker that produced it.
pub struct RenderedSegment { pub speaker: Speaker, pub audio: PcmAudio }

/// Concatenate segments, inserting a silence gap before each (except the first):
/// speaker switch -> GAP_SPEAKER_SWITCH_MS, same speaker -> GAP_SAME_SPEAKER_MS.
pub fn concat_with_gaps(segments: &[RenderedSegment]) -> PcmAudio {
    let mut out: Vec<i16> = Vec::new();
    let mut prev: Option<Speaker> = None;
    for seg in segments {
        if let Some(p) = prev {
            let gap = if p == seg.speaker { GAP_SAME_SPEAKER_MS } else { GAP_SPEAKER_SWITCH_MS };
            out.extend_from_slice(&PcmAudio::silence(gap).samples);
        }
        out.extend_from_slice(&seg.audio.samples);
        prev = Some(seg.speaker);
    }
    PcmAudio { samples: out }
}

/// Encode mono i16 PCM to an MP3 byte buffer.
pub fn encode_mp3(pcm: &PcmAudio) -> anyhow::Result<Vec<u8>> {
    use mp3lame_encoder::{Builder, FlushNoGap, MonoPcm};
    let mut builder = Builder::new().ok_or_else(|| anyhow::anyhow!("lame builder"))?;
    builder.set_num_channels(1).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    builder.set_sample_rate(SAMPLE_RATE).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    builder.set_brate(mp3lame_encoder::Bitrate::Kbps128).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut enc = builder.build().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut mp3 = Vec::with_capacity(pcm.samples.len());
    let n = enc.encode(MonoPcm(&pcm.samples), mp3.spare_capacity_mut())
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    unsafe { mp3.set_len(n); }
    let tail = enc.flush::<FlushNoGap>(mp3.spare_capacity_mut())
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    unsafe { mp3.set_len(mp3.len() + tail); }
    Ok(mp3)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seg(sp: Speaker, n: usize) -> RenderedSegment {
        RenderedSegment { speaker: sp, audio: PcmAudio { samples: vec![100i16; n] } }
    }

    #[test]
    fn switch_gap_inserted_between_different_speakers() {
        let segs = vec![seg(Speaker::Host, 10), seg(Speaker::Expert, 10)];
        let out = concat_with_gaps(&segs);
        let switch_gap = (SAMPLE_RATE as u64 * GAP_SPEAKER_SWITCH_MS as u64 / 1000) as usize;
        assert_eq!(out.samples.len(), 10 + switch_gap + 10);
    }

    #[test]
    fn same_speaker_gap_is_shorter() {
        let segs = vec![seg(Speaker::Host, 10), seg(Speaker::Host, 10)];
        let out = concat_with_gaps(&segs);
        let gap = (SAMPLE_RATE as u64 * GAP_SAME_SPEAKER_MS as u64 / 1000) as usize;
        assert_eq!(out.samples.len(), 10 + gap + 10);
    }

    #[test]
    fn encode_mp3_produces_nonempty_bytes() {
        let pcm = PcmAudio { samples: vec![0i16; SAMPLE_RATE as usize] };
        let mp3 = encode_mp3(&pcm).unwrap();
        assert!(mp3.len() > 100);
        // MP3 frame sync: first byte 0xFF, next has top 3 bits set.
        assert_eq!(mp3[0], 0xFF);
    }
}
```
> Note: remove the bogus `Speaker as _` import line — correct import is only `use aftercode_core::audio::{PcmAudio, GAP_SAME_SPEAKER_MS, GAP_SPEAKER_SWITCH_MS, SAMPLE_RATE};` and `use aftercode_core::episode::Speaker;`.

`crates/aftercode-server/src/pipeline/mod.rs`:
```rust
pub mod assemble;
pub mod normalize;
pub mod rank;
```

- [ ] **Step 2: Fix the import line noted above, create empty `normalize.rs`/`rank.rs` stubs**

`normalize.rs`: `// next task` ; `rank.rs`: `// next task`

- [ ] **Step 3: Run tests**

Run: `cargo test -p aftercode-server assemble`
Expected: 3 passed. If `mp3lame-encoder` API names differ in the installed version, run `cargo doc -p mp3lame-encoder --open` and adjust `Builder`/`encode`/`flush` calls to match; keep the test assertions unchanged.

- [ ] **Step 4: Commit**

```bash
git add crates/aftercode-server/src/pipeline
git commit -m "feat(server): pure-Rust audio concat + mp3 encode"
```

---

## Task 8: Normalize + rank stages

**Files:**
- Modify: `crates/aftercode-server/src/pipeline/normalize.rs`
- Modify: `crates/aftercode-server/src/pipeline/rank.rs`

- [ ] **Step 1: normalize + failing test**

`crates/aftercode-server/src/pipeline/normalize.rs`:
```rust
use crate::providers::llm::NormalizedContext;
use aftercode_core::session::SessionContext;

/// Flatten a SessionContext into a single prompt-ready text block.
pub fn normalize(ctx: &SessionContext) -> NormalizedContext {
    let mut t = String::new();
    if !ctx.changed_files.is_empty() {
        t.push_str("Changed files:\n");
        for f in &ctx.changed_files { t.push_str(&format!("- {f}\n")); }
    }
    if let Some(d) = &ctx.git_diff_summary { t.push_str(&format!("\nDiff summary:\n{d}\n")); }
    if !ctx.terminal_errors.is_empty() {
        t.push_str("\nTerminal errors:\n");
        for e in &ctx.terminal_errors { t.push_str(&format!("- {e}\n")); }
    }
    if !ctx.commit_messages.is_empty() {
        t.push_str("\nCommits:\n");
        for c in &ctx.commit_messages { t.push_str(&format!("- {c}\n")); }
    }
    for ev in &ctx.events {
        t.push_str(&format!("\n[{:?}] {}\n", ev.event_type, ev.content));
    }
    NormalizedContext { text: t, language: ctx.language, minutes: ctx.episode_length_minutes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aftercode_core::session::{Language, SessionContext};
    #[test]
    fn includes_files_and_errors() {
        let ctx = SessionContext {
            project_id: "p".into(), language: Language::En, episode_length_minutes: 10,
            collected_at: "t".into(), events: vec![], changed_files: vec!["m.py".into()],
            git_diff_summary: None, commit_messages: vec![],
            terminal_errors: vec!["CONCURRENTLY cannot run in a transaction".into()],
        };
        let n = normalize(&ctx);
        assert!(n.text.contains("m.py"));
        assert!(n.text.contains("CONCURRENTLY"));
    }
}
```

- [ ] **Step 2: rank + failing test**

`crates/aftercode-server/src/pipeline/rank.rs`:
```rust
use aftercode_core::episode::LearningTopic;

fn score(t: &LearningTopic) -> u8 {
    let p = match t.priority.as_str() { "high" => 3, "medium" => 2, _ => 1 };
    let e = if t.evidence.is_empty() { 0 } else { 1 };
    p + e
}

/// Sort topics by priority+evidence desc, keep at most `max`.
pub fn rank(mut topics: Vec<LearningTopic>, max: usize) -> Vec<LearningTopic> {
    topics.sort_by(|a, b| score(b).cmp(&score(a)));
    topics.truncate(max);
    topics
}

#[cfg(test)]
mod tests {
    use super::*;
    fn topic(pri: &str, ev: bool) -> LearningTopic {
        LearningTopic { title: "t".into(), summary: "s".into(),
            evidence: if ev { vec!["x".into()] } else { vec![] },
            knowledge_gap: "g".into(), difficulty: "intermediate".into(), priority: pri.into() }
    }
    #[test]
    fn high_priority_with_evidence_ranks_first() {
        let out = rank(vec![topic("low", false), topic("high", true)], 5);
        assert_eq!(out[0].priority, "high");
    }
    #[test]
    fn truncates_to_max() {
        let out = rank(vec![topic("high", true), topic("high", true), topic("high", true)], 2);
        assert_eq!(out.len(), 2);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p aftercode-server pipeline::`
Expected: normalize 1 + rank 2 = 3 passed (plus assemble 3).

- [ ] **Step 4: Commit**

```bash
git add crates/aftercode-server/src/pipeline
git commit -m "feat(server): normalize + rank pipeline stages"
```

---

## Task 9: Database migration + models

**Files:**
- Create: `migrations/0001_init.sql`
- Create: `crates/aftercode-server/src/db/mod.rs`, `models.rs`, `queries.rs`

- [ ] **Step 1: Migration**

`migrations/0001_init.sql`:
```sql
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT UNIQUE NOT NULL,
  token_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  default_language TEXT NOT NULL DEFAULT 'en',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE coding_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id),
  user_id UUID NOT NULL REFERENCES users(id),
  source TEXT,
  context_json JSONB NOT NULL,
  summary TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE episode_status AS ENUM
  ('queued','extracting_topics','writing_script','generating_audio','ready','failed');

CREATE TABLE podcast_episodes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  project_id UUID NOT NULL REFERENCES projects(id),
  session_id UUID NOT NULL REFERENCES coding_sessions(id),
  title TEXT NOT NULL DEFAULT '',
  language TEXT NOT NULL,
  status episode_status NOT NULL DEFAULT 'queued',
  duration_seconds INT,
  audio_url TEXT,
  script_json JSONB,
  topics_json JSONB,
  transcript_text TEXT,
  summary TEXT,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- [ ] **Step 2: db module (sqlx pool + query helpers)**

`crates/aftercode-server/src/db/mod.rs`:
```rust
pub mod models;
pub mod queries;
pub type Db = sqlx::PgPool;
```
`crates/aftercode-server/src/db/models.rs`:
```rust
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct User { pub id: Uuid, pub email: String, pub token_hash: String }
```
`crates/aftercode-server/src/db/queries.rs`:
```rust
use super::Db;
use uuid::Uuid;

pub async fn user_by_token_hash(db: &Db, hash: &str) -> anyhow::Result<Option<Uuid>> {
    let row = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE token_hash = $1")
        .bind(hash).fetch_optional(db).await?;
    Ok(row)
}

pub async fn insert_episode(db: &Db, user: Uuid, project: Uuid, session: Uuid, lang: &str)
    -> anyhow::Result<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO podcast_episodes (user_id, project_id, session_id, language)
         VALUES ($1,$2,$3,$4) RETURNING id")
        .bind(user).bind(project).bind(session).bind(lang)
        .fetch_one(db).await?;
    Ok(id)
}

pub async fn set_status(db: &Db, id: Uuid, status: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE podcast_episodes SET status = $1::episode_status, updated_at = now() WHERE id = $2")
        .bind(status).bind(id).execute(db).await?;
    Ok(())
}
```

- [ ] **Step 3: Verify migration applies (requires a running Postgres)**

Run:
```bash
createdb aftercode 2>/dev/null; \
psql "$DATABASE_URL" -f migrations/0001_init.sql
```
Expected: `CREATE TABLE` / `CREATE TYPE` lines, no errors. (Use `DATABASE_URL` from `.env`.)

- [ ] **Step 4: Compile check**

Run: `cargo build -p aftercode-server`
Expected: compiles (sqlx queries are runtime-checked here, not macro-checked).

- [ ] **Step 5: Commit**

```bash
git add migrations crates/aftercode-server/src/db
git commit -m "feat(server): db migration + query helpers"
```

---

## Task 10: AppState + error type + auth extractor

**Files:**
- Create: `crates/aftercode-server/src/state.rs`
- Create: `crates/aftercode-server/src/error.rs`
- Create: `crates/aftercode-server/src/auth.rs`

- [ ] **Step 1: AppState**

`crates/aftercode-server/src/state.rs`:
```rust
use crate::config::Config;
use crate::db::Db;
use crate::providers::llm::{LlmProvider, MockLlm};
use crate::providers::tts::{TtsProvider, MockTts};
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
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10).connect(&cfg.database_url).await?;
        let llm: Arc<dyn LlmProvider> = match cfg.llm_provider.as_str() {
            "mock" => Arc::new(MockLlm),
            "openai" => Arc::new(crate::providers::openai::OpenAiProvider::from_cfg(&cfg)?),
            _ => Arc::new(crate::providers::anthropic::AnthropicProvider::from_cfg(&cfg)?),
        };
        let tts: Arc<dyn TtsProvider> = if cfg.elevenlabs_api_key.is_some() {
            Arc::new(crate::providers::tts::eleven_from_cfg(&cfg)?)
        } else { Arc::new(MockTts) };
        let blob: Arc<dyn BlobStore> = match cfg.blob_store.as_str() {
            "s3" => Arc::new(crate::storage::s3::S3Store::from_cfg(&cfg).await?),
            "mock" => Arc::new(MockBlob::default()),
            _ => Arc::new(crate::storage::localfs::LocalFs::from_cfg(&cfg)),
        };
        Ok(AppState { db, cfg, llm, tts, blob })
    }

    /// Test constructor with injected mocks (no network).
    #[cfg(test)]
    pub fn for_test(db: Db, cfg: Config) -> Self {
        use std::sync::Arc;
        AppState { db, cfg, llm: Arc::new(MockLlm), tts: Arc::new(MockTts),
                   blob: Arc::new(MockBlob::default()) }
    }
}
```

- [ ] **Step 2: error type**

`crates/aftercode-server/src/error.rs`:
```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("unauthorized")] Unauthorized,
    #[error("not found")] NotFound,
    #[error("bad request: {0}")] BadRequest(String),
    #[error(transparent)] Other(#[from] anyhow::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            ServerError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ServerError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ServerError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServerError::Other(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (code, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
```

- [ ] **Step 3: auth extractor + token hashing**

`crates/aftercode-server/src/auth.rs`:
```rust
use crate::db::queries;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

pub fn hash_token(token: &str) -> String {
    use base64::Engine;
    // sha256 via ring-free std: use a simple stable hash chain. For real deployments
    // swap to sha2. MVP: sha2 is acceptable; add `sha2 = "0.10"` and use it here.
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(digest)
}

pub struct AuthUser(pub Uuid);

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ServerError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ServerError::Unauthorized)?;
        let token = header.strip_prefix("Bearer ").ok_or(ServerError::Unauthorized)?;
        let hash = hash_token(token);
        let uid = queries::user_by_token_hash(&state.db, &hash).await
            .map_err(ServerError::Other)?
            .ok_or(ServerError::Unauthorized)?;
        Ok(AuthUser(uid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_is_stable_and_nonempty() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert!(!hash_token("abc").is_empty());
    }
}
```
Add to `crates/aftercode-server/Cargo.toml` deps: `sha2 = "0.10"`.

- [ ] **Step 4: Run auth unit test**

Run: `cargo test -p aftercode-server auth::`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aftercode-server/src Cargo.toml crates/aftercode-server/Cargo.toml
git commit -m "feat(server): app state, error type, bearer auth"
```

---

## Task 11: Pipeline orchestrator + worker

**Files:**
- Modify: `crates/aftercode-server/src/pipeline/mod.rs`
- Create: `crates/aftercode-server/src/worker.rs`

- [ ] **Step 1: Orchestrator + failing test**

Append to `crates/aftercode-server/src/pipeline/mod.rs`:
```rust
use crate::providers::llm::{LlmProvider, ScriptOpts};
use crate::providers::tts::TtsProvider;
use crate::storage::blob::BlobStore;
use aftercode_core::audio::VoiceRole;
use aftercode_core::episode::{EpisodeScript, LearningTopic, Speaker};
use aftercode_core::session::SessionContext;
use assemble::{concat_with_gaps, encode_mp3, RenderedSegment};
use std::sync::Arc;

pub struct PipelineOutput {
    pub topics: Vec<LearningTopic>,
    pub script: EpisodeScript,
    pub audio_url: String,
    pub duration_seconds: i32,
    pub transcript: String,
}

/// Full generation: normalize → topics → rank → script → tts → assemble → store.
/// `on_status` is called as each stage starts so the worker can persist progress.
pub async fn run_pipeline(
    ctx: &SessionContext,
    episode_key: &str,
    llm: Arc<dyn LlmProvider>,
    tts: Arc<dyn TtsProvider>,
    blob: Arc<dyn BlobStore>,
    mut on_status: impl FnMut(&str),
) -> anyhow::Result<PipelineOutput> {
    on_status("extracting_topics");
    let norm = normalize::normalize(ctx);
    let topics = rank::rank(llm.extract_topics(&norm).await?, 3);

    on_status("writing_script");
    let script = llm.write_script(&topics, &ScriptOpts {
        language: ctx.language, minutes: ctx.episode_length_minutes,
    }).await?;

    on_status("generating_audio");
    let mut rendered = Vec::new();
    for seg in &script.segments {
        let role = match seg.speaker { Speaker::Host => VoiceRole::Host, Speaker::Expert => VoiceRole::Expert };
        let audio = tts.synthesize(&seg.text, role, ctx.language).await?;
        rendered.push(RenderedSegment { speaker: seg.speaker, audio });
    }
    let full = concat_with_gaps(&rendered);
    let duration = full.duration_seconds() as i32;
    let mp3 = encode_mp3(&full)?;
    let url = blob.put(&format!("episodes/{episode_key}.mp3"), mp3, "audio/mpeg").await?;

    let transcript = script.segments.iter()
        .map(|s| format!("{:?}: {}", s.speaker, s.text)).collect::<Vec<_>>().join("\n");

    Ok(PipelineOutput { topics, script, audio_url: url, duration_seconds: duration, transcript })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::llm::MockLlm;
    use crate::providers::tts::MockTts;
    use crate::storage::blob::MockBlob;
    use aftercode_core::session::Language;

    #[tokio::test]
    async fn pipeline_produces_ready_episode_with_mocks() {
        let ctx = SessionContext {
            project_id: "p".into(), language: Language::En, episode_length_minutes: 10,
            collected_at: "t".into(), events: vec![],
            changed_files: vec!["migration.py".into()],
            git_diff_summary: Some("added CONCURRENTLY".into()),
            commit_messages: vec![], terminal_errors: vec![],
        };
        let mut statuses = Vec::new();
        let out = run_pipeline(&ctx, "ep_test",
            Arc::new(MockLlm), Arc::new(MockTts), Arc::new(MockBlob::default()),
            |s| statuses.push(s.to_string())).await.unwrap();
        assert!(out.audio_url.starts_with("mock://"));
        assert!(out.duration_seconds > 0);
        assert!(!out.script.segments.is_empty());
        assert_eq!(statuses, vec!["extracting_topics","writing_script","generating_audio"]);
    }
}
```

- [ ] **Step 2: Run the integration-style test (acceptance: end-to-end with mocks)**

Run: `cargo test -p aftercode-server pipeline::tests::pipeline_produces_ready_episode_with_mocks`
Expected: PASS.

- [ ] **Step 3: Worker (spawns pipeline, persists status + result)**

`crates/aftercode-server/src/worker.rs`:
```rust
use crate::db::queries;
use crate::pipeline::run_pipeline;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use uuid::Uuid;

/// Spawn generation for an already-inserted (queued) episode.
pub fn spawn(state: AppState, episode_id: Uuid, ctx: SessionContext) {
    tokio::spawn(async move {
        let db = state.db.clone();
        let result = run_pipeline(
            &ctx, &episode_id.to_string(),
            state.llm.clone(), state.tts.clone(), state.blob.clone(),
            |s| { let db = db.clone(); let id = episode_id; let s = s.to_string();
                  tokio::spawn(async move { let _ = queries::set_status(&db, id, &s).await; }); },
        ).await;
        match result {
            Ok(out) => {
                let topics = serde_json::to_value(&out.topics).unwrap_or_default();
                let script = serde_json::to_value(&out.script).unwrap_or_default();
                let _ = sqlx::query(
                    "UPDATE podcast_episodes SET status='ready'::episode_status, title=$1,
                     audio_url=$2, duration_seconds=$3, topics_json=$4, script_json=$5,
                     transcript_text=$6, summary=$7, updated_at=now() WHERE id=$8")
                    .bind(&out.script.title).bind(&out.audio_url).bind(out.duration_seconds)
                    .bind(topics).bind(script).bind(&out.transcript)
                    .bind(out.script.summary_points.join(" ")).bind(episode_id)
                    .execute(&state.db).await;
            }
            Err(e) => {
                let _ = sqlx::query(
                    "UPDATE podcast_episodes SET status='failed'::episode_status, error=$1,
                     updated_at=now() WHERE id=$2")
                    .bind(e.to_string()).bind(episode_id).execute(&state.db).await;
            }
        }
    });
}
```

- [ ] **Step 4: Compile**

Run: `cargo build -p aftercode-server`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/aftercode-server/src
git commit -m "feat(server): pipeline orchestrator + in-process worker"
```

---

## Task 12: Real Anthropic + OpenAI + ElevenLabs + storage impls

**Files:**
- Modify: `crates/aftercode-server/src/providers/anthropic.rs`, `openai.rs`, `tts.rs`
- Modify: `crates/aftercode-server/src/storage/localfs.rs`, `s3.rs`

- [ ] **Step 1: Anthropic provider**

`crates/aftercode-server/src/providers/anthropic.rs`:
```rust
use super::llm::{LlmProvider, NormalizedContext, ScriptOpts};
use crate::config::Config;
use aftercode_core::episode::{EpisodeScript, LearningTopic};
use aftercode_core::session::Language;
use async_trait::async_trait;

pub struct AnthropicProvider { key: String, http: reqwest::Client }

impl AnthropicProvider {
    pub fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let key = cfg.anthropic_api_key.clone()
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
        Ok(Self { key, http: reqwest::Client::new() })
    }

    async fn call_json(&self, system: &str, user: &str, schema: serde_json::Value)
        -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({
            "model": "claude-opus-4-8",
            "max_tokens": 8000,
            "thinking": { "type": "adaptive" },
            "output_config": { "format": { "type": "json_schema", "schema": schema } },
            "system": system,
            "messages": [{ "role": "user", "content": user }]
        });
        let resp = self.http.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .json(&body).send().await?.error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        let text = v["content"].as_array().and_then(|a| a.iter()
            .find(|b| b["type"] == "text")).and_then(|b| b["text"].as_str())
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

fn lang_name(l: Language) -> &'static str { match l { Language::He => "Hebrew", Language::En => "English" } }

fn script_system(l: Language) -> String {
    match l {
        Language::He => "אתה כותב פודקאסט טכני בעברית בין מנחה (host) למומחה (expert). \
            דבר טבעי כמו מפתחים ישראלים, השאר מונחים טכניים באנגלית כשטבעי, הימנע מעברית פורמלית מדי. \
            החזר JSON בלבד.".to_string(),
        Language::En => "You write a technical two-speaker podcast (host + expert). \
            Calm mentor tone, practical, not cheesy. Return JSON only.".to_string(),
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        let system = "Extract deep technical learning topics from a coding session. \
            Every topic must cite evidence from the provided context. Return JSON only.";
        let user = format!("Coding session ({} min target, {}):\n\n{}",
            ctx.minutes, lang_name(ctx.language), ctx.text);
        let v = self.call_json(system, &user, topics_schema()).await?;
        Ok(serde_json::from_value(v["topics"].clone())?)
    }
    async fn write_script(&self, topics: &[LearningTopic], opts: &ScriptOpts)
        -> anyhow::Result<EpisodeScript> {
        let user = format!("Write a {}-minute episode in {} about these topics:\n{}",
            opts.minutes, lang_name(opts.language),
            serde_json::to_string_pretty(topics)?);
        let v = self.call_json(&script_system(opts.language), &user, script_schema()).await?;
        Ok(serde_json::from_value(v)?)
    }
}
```

- [ ] **Step 2: OpenAI provider (JSON mode)**

`crates/aftercode-server/src/providers/openai.rs`:
```rust
use super::llm::{LlmProvider, NormalizedContext, ScriptOpts};
use crate::config::Config;
use aftercode_core::episode::{EpisodeScript, LearningTopic};
use async_trait::async_trait;

pub struct OpenAiProvider { key: String, http: reqwest::Client }

impl OpenAiProvider {
    pub fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let key = cfg.openai_api_key.clone()
            .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        Ok(Self { key, http: reqwest::Client::new() })
    }
    async fn call_json(&self, system: &str, user: &str) -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "response_format": { "type": "json_object" },
            "messages": [{ "role":"system","content":system },{ "role":"user","content":user }]
        });
        let resp = self.http.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.key).json(&body).send().await?.error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        let text = v["choices"][0]["message"]["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("no content"))?;
        Ok(serde_json::from_str(text)?)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        let user = format!("Return JSON {{\"topics\":[...]}} with fields title, summary, \
            evidence[], knowledge_gap, difficulty, priority. Session:\n{}", ctx.text);
        let v = self.call_json("Extract evidence-based technical topics. JSON only.", &user).await?;
        Ok(serde_json::from_value(v["topics"].clone())?)
    }
    async fn write_script(&self, topics: &[LearningTopic], opts: &ScriptOpts)
        -> anyhow::Result<EpisodeScript> {
        let user = format!("Return JSON with title, language, segments[(speaker host|expert, text)], \
            summary_points[], quiz{{question,answer}}. {} minutes. Topics:\n{}",
            opts.minutes, serde_json::to_string(topics)?);
        let v = self.call_json("Two-speaker technical podcast. JSON only.", &user).await?;
        Ok(serde_json::from_value(v)?)
    }
}
```

- [ ] **Step 3: ElevenLabs TTS (append to tts.rs)**

Append to `crates/aftercode-server/src/providers/tts.rs`:
```rust
use crate::config::Config;

pub struct ElevenLabsProvider {
    key: String, host_voice: String, expert_voice: String, http: reqwest::Client,
}

pub fn eleven_from_cfg(cfg: &Config) -> anyhow::Result<ElevenLabsProvider> {
    Ok(ElevenLabsProvider {
        key: cfg.elevenlabs_api_key.clone().ok_or_else(|| anyhow::anyhow!("no eleven key"))?,
        host_voice: cfg.host_voice_id.clone().ok_or_else(|| anyhow::anyhow!("no host voice"))?,
        expert_voice: cfg.expert_voice_id.clone().ok_or_else(|| anyhow::anyhow!("no expert voice"))?,
        http: reqwest::Client::new(),
    })
}

#[async_trait]
impl TtsProvider for ElevenLabsProvider {
    async fn synthesize(&self, text: &str, voice: VoiceRole, _lang: Language) -> anyhow::Result<PcmAudio> {
        let vid = match voice { VoiceRole::Host => &self.host_voice, VoiceRole::Expert => &self.expert_voice };
        // Request raw PCM 44.1kHz so we can concat without decoding.
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{vid}?output_format=pcm_44100");
        let resp = self.http.post(&url)
            .header("xi-api-key", &self.key)
            .json(&serde_json::json!({ "text": text, "model_id": "eleven_multilingual_v2" }))
            .send().await?.error_for_status()?;
        let bytes = resp.bytes().await?;
        // pcm_44100 is little-endian i16 mono.
        let samples = bytes.chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
        Ok(PcmAudio { samples })
    }
}
```

- [ ] **Step 4: LocalFs + S3 storage**

`crates/aftercode-server/src/storage/localfs.rs`:
```rust
use super::blob::BlobStore;
use crate::config::Config;
use async_trait::async_trait;

pub struct LocalFs { dir: String, public_url: String }

impl LocalFs {
    pub fn from_cfg(cfg: &Config) -> Self {
        LocalFs { dir: cfg.localfs_dir.clone(), public_url: cfg.public_url.clone() }
    }
}

#[async_trait]
impl BlobStore for LocalFs {
    async fn put(&self, key: &str, bytes: Vec<u8>, _ct: &str) -> anyhow::Result<String> {
        let path = std::path::Path::new(&self.dir).join(key);
        if let Some(p) = path.parent() { tokio::fs::create_dir_all(p).await?; }
        tokio::fs::write(&path, bytes).await?;
        Ok(format!("{}/static/{key}", self.public_url.trim_end_matches('/')))
    }
}
```

`crates/aftercode-server/src/storage/s3.rs`:
```rust
use super::blob::BlobStore;
use crate::config::Config;
use async_trait::async_trait;

pub struct S3Store { client: aws_sdk_s3::Client, bucket: String, public_url: String }

impl S3Store {
    pub async fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let bucket = cfg.s3_bucket.clone().ok_or_else(|| anyhow::anyhow!("S3_BUCKET not set"))?;
        let conf = aws_config::load_from_env().await;
        Ok(S3Store { client: aws_sdk_s3::Client::new(&conf), bucket, public_url: cfg.public_url.clone() })
    }
}

#[async_trait]
impl BlobStore for S3Store {
    async fn put(&self, key: &str, bytes: Vec<u8>, ct: &str) -> anyhow::Result<String> {
        self.client.put_object().bucket(&self.bucket).key(key)
            .body(bytes.into()).content_type(ct).send().await?;
        Ok(format!("{}/{key}", self.public_url.trim_end_matches('/')))
    }
}
```

- [ ] **Step 5: Compile (no network calls in tests; real providers only built, not invoked)**

Run: `cargo build -p aftercode-server`
Expected: compiles.

- [ ] **Step 6: Commit**

```bash
git add crates/aftercode-server/src/providers crates/aftercode-server/src/storage
git commit -m "feat(server): anthropic/openai/elevenlabs + localfs/s3 impls"
```

---

## Task 13: HTTP routes + router

**Files:**
- Create: `crates/aftercode-server/src/routes/mod.rs`, `health.rs`, `projects.rs`, `sessions.rs`, `episodes.rs`, `cli.rs`

- [ ] **Step 1: Router + health + failing test**

`crates/aftercode-server/src/routes/mod.rs`:
```rust
pub mod health;
pub mod projects;
pub mod sessions;
pub mod episodes;
pub mod cli;

use crate::state::AppState;
use axum::routing::{get, post};
use axum::Router;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/me", get(projects::me))
        .route("/projects", post(projects::create).get(projects::list))
        .route("/cli/register-project", post(projects::register))
        .route("/cli/upload-session", post(sessions::upload))
        .route("/cli/generate-episode", post(cli::generate))
        .route("/cli/episode-status/:id", get(cli::status))
        .route("/episodes", get(episodes::list))
        .route("/episodes/:id", get(episodes::detail))
        .route("/episodes/:id/retry", post(episodes::retry))
        .with_state(state)
}
```
`crates/aftercode-server/src/routes/health.rs`:
```rust
use axum::Json;
pub async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[cfg(test)]
mod tests {
    use crate::routes::router;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let cfg = crate::config::Config {
            database_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| std::env::var("DATABASE_URL").unwrap()),
            bind_addr: "127.0.0.1:0".into(), public_url: "http://t".into(),
            llm_provider: "mock".into(), anthropic_api_key: None, openai_api_key: None,
            elevenlabs_api_key: None, host_voice_id: None, expert_voice_id: None,
            blob_store: "mock".into(), localfs_dir: "./data".into(), s3_bucket: None,
        };
        let db = sqlx::postgres::PgPoolOptions::new().max_connections(2)
            .connect(&cfg.database_url).await.unwrap();
        AppState::for_test(db, cfg)
    }

    #[tokio::test]
    async fn healthz_ok() {
        let app = router(test_state().await);
        let resp = app.oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
```

- [ ] **Step 2: projects routes**

`crates/aftercode-server/src/routes/projects.rs`:
```rust
use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

pub async fn me(AuthUser(uid): AuthUser) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "user_id": uid }))
}

#[derive(Deserialize)]
pub struct NewProject { pub name: String, #[serde(default)] pub default_language: Option<String> }

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(p): Json<NewProject>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let lang = p.default_language.unwrap_or_else(|| "en".into());
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO projects (user_id, name, default_language) VALUES ($1,$2,$3) RETURNING id")
        .bind(uid).bind(&p.name).bind(&lang).fetch_one(&st.db).await
        .map_err(|e| ServerError::Other(e.into()))?;
    Ok(Json(serde_json::json!({ "project_id": id, "project_name": p.name })))
}

pub async fn register(st: State<AppState>, user: AuthUser, body: Json<NewProject>)
    -> Result<Json<serde_json::Value>, ServerError> { create(st, user, body).await }

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser)
    -> Result<Json<serde_json::Value>, ServerError> {
    let rows = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, name, default_language FROM projects WHERE user_id=$1 ORDER BY created_at DESC")
        .bind(uid).fetch_all(&st.db).await.map_err(|e| ServerError::Other(e.into()))?;
    let items: Vec<_> = rows.into_iter()
        .map(|(id,name,lang)| serde_json::json!({ "id":id,"name":name,"default_language":lang }))
        .collect();
    Ok(Json(serde_json::json!({ "projects": items })))
}
```

- [ ] **Step 3: sessions route**

`crates/aftercode-server/src/routes/sessions.rs`:
```rust
use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use axum::extract::State;
use axum::Json;
use uuid::Uuid;

pub async fn upload(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(ctx): Json<SessionContext>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let project = Uuid::parse_str(&ctx.project_id)
        .map_err(|_| ServerError::BadRequest("project_id must be a uuid".into()))?;
    let ctx_json = serde_json::to_value(&ctx).map_err(|e| ServerError::Other(e.into()))?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO coding_sessions (project_id, user_id, source, context_json)
         VALUES ($1,$2,'cli',$3) RETURNING id")
        .bind(project).bind(uid).bind(ctx_json).fetch_one(&st.db).await
        .map_err(|e| ServerError::Other(e.into()))?;
    Ok(Json(serde_json::json!({ "session_id": id })))
}
```

- [ ] **Step 4: cli generate + status routes**

`crates/aftercode-server/src/routes/cli.rs`:
```rust
use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GenerateReq {
    pub project_id: String,
    pub session_id: Option<String>,
    pub session_context: Option<SessionContext>,
    pub language: Option<String>,
}

pub async fn generate(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(req): Json<GenerateReq>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let project = Uuid::parse_str(&req.project_id)
        .map_err(|_| ServerError::BadRequest("project_id must be uuid".into()))?;

    // Resolve a session: use provided session_id or persist the inline context.
    let (session_id, ctx) = if let Some(sid) = req.session_id {
        let sid = Uuid::parse_str(&sid).map_err(|_| ServerError::BadRequest("bad session_id".into()))?;
        let json: serde_json::Value = sqlx::query_scalar("SELECT context_json FROM coding_sessions WHERE id=$1")
            .bind(sid).fetch_optional(&st.db).await.map_err(|e| ServerError::Other(e.into()))?
            .ok_or(ServerError::NotFound)?;
        (sid, serde_json::from_value::<SessionContext>(json).map_err(|e| ServerError::Other(e.into()))?)
    } else {
        let ctx = req.session_context.ok_or(ServerError::BadRequest("need session_id or session_context".into()))?;
        let json = serde_json::to_value(&ctx).map_err(|e| ServerError::Other(e.into()))?;
        let sid = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO coding_sessions (project_id,user_id,source,context_json)
             VALUES ($1,$2,'cli',$3) RETURNING id")
            .bind(project).bind(uid).bind(json).fetch_one(&st.db).await
            .map_err(|e| ServerError::Other(e.into()))?;
        (sid, ctx)
    };

    let lang = req.language.unwrap_or_else(|| match ctx.language {
        aftercode_core::session::Language::He => "he".into(),
        aftercode_core::session::Language::En => "en".into(),
    });
    let episode_id = crate::db::queries::insert_episode(&st.db, uid, project, session_id, &lang).await
        .map_err(ServerError::Other)?;
    crate::worker::spawn(st.clone(), episode_id, ctx);
    Ok(Json(serde_json::json!({ "episode_id": episode_id, "status": "queued" })))
}

pub async fn status(State(st): State<AppState>, AuthUser(_): AuthUser, Path(id): Path<Uuid>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT status::text, title, error FROM podcast_episodes WHERE id=$1")
        .bind(id).fetch_optional(&st.db).await.map_err(|e| ServerError::Other(e.into()))?
        .ok_or(ServerError::NotFound)?;
    Ok(Json(serde_json::json!({ "episode_id": id, "status": row.0, "title": row.1, "error": row.2 })))
}
```

- [ ] **Step 5: episodes list/detail/retry routes**

`crates/aftercode-server/src/routes/episodes.rs`:
```rust
use crate::auth::AuthUser;
use crate::error::ServerError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser)
    -> Result<Json<serde_json::Value>, ServerError> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, Option<i32>, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>, String)>(
        "SELECT e.id, e.title, e.language, e.status::text, e.duration_seconds, e.topics_json,
                e.created_at, p.name
         FROM podcast_episodes e JOIN projects p ON p.id = e.project_id
         WHERE e.user_id=$1 ORDER BY e.created_at DESC")
        .bind(uid).fetch_all(&st.db).await.map_err(|e| ServerError::Other(e.into()))?;
    let items: Vec<_> = rows.into_iter().map(|(id,title,lang,status,dur,topics,created,proj)| {
        let topic_titles: Vec<String> = topics.as_ref()
            .and_then(|t| t.as_array())
            .map(|a| a.iter().filter_map(|x| x["title"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        serde_json::json!({ "id":id,"title":title,"language":lang,"status":status,
            "duration_seconds":dur,"topics":topic_titles,"project_name":proj,
            "created_at":created.to_rfc3339() })
    }).collect();
    Ok(Json(serde_json::json!({ "episodes": items })))
}

pub async fn detail(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>, Option<i32>, Option<String>, Option<String>, Option<serde_json::Value>, Option<serde_json::Value>, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, title, language, status::text, audio_url, duration_seconds, summary,
                transcript_text, topics_json, script_json, error, created_at
         FROM podcast_episodes WHERE id=$1 AND user_id=$2")
        .bind(id).bind(uid).fetch_optional(&st.db).await.map_err(|e| ServerError::Other(e.into()))?
        .ok_or(ServerError::NotFound)?;
    Ok(Json(serde_json::json!({
        "id":row.0,"title":row.1,"language":row.2,"status":row.3,"audio_url":row.4,
        "duration_seconds":row.5,"summary":row.6,"transcript_text":row.7,
        "topics":row.8,"script":row.9,"error":row.10,"created_at":row.11.to_rfc3339() })))
}

pub async fn retry(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>)
    -> Result<Json<serde_json::Value>, ServerError> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT s.context_json FROM podcast_episodes e
         JOIN coding_sessions s ON s.id = e.session_id
         WHERE e.id=$1 AND e.user_id=$2")
        .bind(id).bind(uid).fetch_optional(&st.db).await.map_err(|e| ServerError::Other(e.into()))?
        .ok_or(ServerError::NotFound)?;
    let ctx: aftercode_core::session::SessionContext =
        serde_json::from_value(row).map_err(|e| ServerError::Other(e.into()))?;
    crate::db::queries::set_status(&st.db, id, "queued").await.map_err(ServerError::Other)?;
    crate::worker::spawn(st.clone(), id, ctx);
    Ok(Json(serde_json::json!({ "episode_id": id, "status": "queued" })))
}
```

- [ ] **Step 6: serve static audio for localfs (add to router)**

In `routes/mod.rs`, add to the `Router`: `.nest_service("/static", tower_http::services::ServeDir::new(state.cfg.localfs_dir.clone()))` before `.with_state(state)`. Add `tower-http` feature `fs`: in `Cargo.toml` change tower-http features to `["trace","cors","fs"]`.

- [ ] **Step 7: Run route test (needs Postgres with migration applied)**

Run: `cargo test -p aftercode-server routes::health`
Expected: `healthz_ok` passes.

- [ ] **Step 8: Commit**

```bash
git add crates/aftercode-server/src/routes crates/aftercode-server/Cargo.toml
git commit -m "feat(server): http routes (projects, sessions, cli, episodes)"
```

---

## Task 14: Seed-user CLI subcommand + end-to-end HTTP test

**Files:**
- Modify: `crates/aftercode-server/src/main.rs` (add `seed-user` subcommand path)
- Create: `crates/aftercode-server/src/routes/episodes.rs` test (e2e)

- [ ] **Step 1: Seed-user via argv**

In `main.rs`, before starting the server, handle `seed-user`:
```rust
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("seed-user") {
        let email = args.get(2).cloned().unwrap_or_else(|| "dev@example.com".into());
        let token = format!("ak_{}", uuid::Uuid::new_v4().simple());
        let hash = auth::hash_token(&token);
        let db = sqlx::postgres::PgPoolOptions::new().connect(&cfg.database_url).await?;
        sqlx::query("INSERT INTO users (email, token_hash) VALUES ($1,$2)
                     ON CONFLICT (email) DO UPDATE SET token_hash=EXCLUDED.token_hash")
            .bind(&email).bind(&hash).execute(&db).await?;
        println!("user {email} token: {token}");
        return Ok(());
    }
```
(Place after `let cfg = config::Config::from_env()?;`.)

- [ ] **Step 2: e2e test — generate episode reaches `ready` with mocks**

Add to `crates/aftercode-server/src/routes/episodes.rs`:
```rust
#[cfg(test)]
mod tests {
    use crate::routes::router;
    use crate::state::AppState;
    use crate::auth::hash_token;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn setup() -> (AppState, String, Uuid) {
        let cfg = crate::config::Config {
            database_url: std::env::var("DATABASE_URL").unwrap(),
            bind_addr: "127.0.0.1:0".into(), public_url: "http://t".into(),
            llm_provider: "mock".into(), anthropic_api_key: None, openai_api_key: None,
            elevenlabs_api_key: None, host_voice_id: None, expert_voice_id: None,
            blob_store: "mock".into(), localfs_dir: "./data".into(), s3_bucket: None,
        };
        let db = sqlx::postgres::PgPoolOptions::new().connect(&cfg.database_url).await.unwrap();
        let token = "ak_test_token";
        let uid: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, token_hash) VALUES ($1,$2)
             ON CONFLICT (email) DO UPDATE SET token_hash=EXCLUDED.token_hash RETURNING id")
            .bind(format!("t-{}@e.com", Uuid::new_v4())).bind(hash_token(token))
            .fetch_one(&db).await.unwrap();
        let pid: Uuid = sqlx::query_scalar(
            "INSERT INTO projects (user_id, name) VALUES ($1,'p') RETURNING id")
            .bind(uid).fetch_one(&db).await.unwrap();
        (AppState::for_test(db, cfg), token.to_string(), pid)
    }

    #[tokio::test]
    async fn generate_then_status_reaches_ready() {
        let (state, token, pid) = setup().await;
        let db = state.db.clone();
        let app = router(state);
        let body = serde_json::json!({
            "project_id": pid,
            "language": "en",
            "session_context": {
                "project_id": pid, "language": "en", "episode_length_minutes": 10,
                "collected_at": "2026-06-14T19:00:00Z",
                "changed_files": ["m.py"], "git_diff_summary": "x" }
        });
        let resp = app.clone().oneshot(Request::builder().method("POST")
            .uri("/cli/generate-episode")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type","application/json")
            .body(Body::from(body.to_string())).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let eid = v["episode_id"].as_str().unwrap().to_string();

        // Worker runs async; poll the DB up to 5s.
        let id = Uuid::parse_str(&eid).unwrap();
        let mut status = String::new();
        for _ in 0..50 {
            status = sqlx::query_scalar::<_, String>("SELECT status::text FROM podcast_episodes WHERE id=$1")
                .bind(id).fetch_one(&db).await.unwrap();
            if status == "ready" || status == "failed" { break; }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert_eq!(status, "ready");
    }
}
```

- [ ] **Step 3: Run e2e test**

Run: `cargo test -p aftercode-server generate_then_status_reaches_ready -- --nocapture`
Expected: PASS (episode reaches `ready`).

- [ ] **Step 4: Commit**

```bash
git add crates/aftercode-server/src
git commit -m "feat(server): seed-user command + e2e generate test"
```

---

## Task 15: Backend docs + CI

**Files:**
- Create: `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `docs/SELF_HOSTING.md`, `docs/ARCHITECTURE.md`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: README + licenses + self-host doc**

`README.md`: project summary, quickstart (run Postgres, `psql -f migrations/0001_init.sql`, `cargo run -p aftercode-server seed-user`, copy token, `cargo run -p aftercode-server`). Add MIT + Apache-2.0 license texts to the two LICENSE files (standard SPDX texts). `docs/SELF_HOSTING.md`: env table from `.env.example`, provider setup (Anthropic/OpenAI/ElevenLabs keys, voice IDs), storage (localfs vs S3). `docs/ARCHITECTURE.md`: condensed from the design spec.

- [ ] **Step 2: CI**

`.github/workflows/ci.yml`:
```yaml
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env: { POSTGRES_USER: aftercode, POSTGRES_PASSWORD: aftercode, POSTGRES_DB: aftercode }
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready --health-interval 10s --health-timeout 5s --health-retries 5
    env:
      DATABASE_URL: postgres://aftercode:aftercode@localhost:5432/aftercode
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - run: psql "$DATABASE_URL" -f migrations/0001_init.sql
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 3: Verify locally**

Run: `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings`
Expected: clean (fix warnings as they surface).

- [ ] **Step 4: Commit**

```bash
git add README.md LICENSE-MIT LICENSE-APACHE docs .github
git commit -m "docs+ci: readme, licenses, self-hosting, github actions"
```

---

## Self-Review (spec coverage)

- §3 repo layout → Task 1. §4 core types → Tasks 2–4. §6.1 endpoints → Tasks 13–14. §6.3 traits → Task 6. §6.4 execution/status machine → Task 11. §6.5 audio assembly → Task 7. §7 data model → Task 9. §8 Hebrew/English → Task 12 (`script_system`). §9 errors+testing → Tasks 10, 7, 11, 14. §10 config → Task 5. §11 open-source posture → Task 15. §13 acceptance (mocked e2e `ready`) → Task 14.
- Type consistency: `LlmProvider`/`TtsProvider`/`BlobStore`, `run_pipeline`, `concat_with_gaps`, `encode_mp3`, `hash_token`, `EpisodeStatus` snake_case strings used consistently across tasks.
- Placeholder note fixed inline in Task 7 (bad import line called out for removal).
```
