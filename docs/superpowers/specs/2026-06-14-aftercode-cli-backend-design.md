# Aftercode — Design Spec (Phase 1: Rust CLI + API Backend)

**Date:** 2026-06-14
**Status:** Approved (design), pending spec review → implementation plan
**Scope:** This spec covers Phase 1 — the Rust CLI and the podcast-generation API backend. Phases 2 (Web UI) and 3 (extensions, auto-daily, RSS) are mapped at the end as separate spec→plan→build cycles.

---

## 1. Product Summary

Aftercode turns a developer's daily AI coding-agent sessions into personalized learning podcasts. The CLI collects what the user built/debugged that day (git diffs, changed files, commits, agent hook events, terminal errors), normalizes it, and sends it to a backend that extracts the deeper technical topics, writes a two-speaker script (host + expert), generates audio via ElevenLabs, and stores a playable episode. Hebrew and English supported.

The product is open source (MIT/Apache-2.0 dual license), self-hostable, CLI-first.

Full product requirements live in `docs/PRD.md`. This spec is the engineering design for Phase 1.

---

## 2. Decisions (locked)

| Area | Decision |
|---|---|
| CLI language | **Rust** (clap derive) |
| Backend language | **Rust** (Axum + sqlx + Tokio), same Cargo workspace |
| Shared types | `aftercode-core` crate — session-context schema + episode types defined once |
| LLM provider | **Pluggable** `LlmProvider` trait; Anthropic (default, `claude-opus-4-8`) + OpenAI impls, chosen by config/env |
| Audio assembly | **Pure-Rust PCM concat** — ElevenLabs PCM per segment, sample concat + silence gaps, `mp3lame-encoder` for final MP3. No external binary. |
| Job execution | **In-process Tokio worker** — `POST` returns `queued`, spawned task runs pipeline, updates status in Postgres; CLI polls. Redis noted as scale path. |
| Object storage | `BlobStore` trait — `LocalFs` (dev) + S3-compatible (R2/prod) |
| Auth | **API key / personal bearer token** — `aftercode login <token>`; backend checks bearer. No email infra. |
| Database | PostgreSQL via sqlx (migrations) |
| Languages | Hebrew + English |

---

## 3. Repository Layout

```
aftercode/
├── Cargo.toml                  # workspace manifest
├── crates/
│   ├── aftercode-core/         # shared lib (no I/O): types, schema, errors
│   ├── aftercode-cli/          # binary: `aftercode`
│   └── aftercode-server/       # binary: `aftercode-server` + lib for testing
├── migrations/                 # sqlx Postgres migrations (0001_init.sql, ...)
├── docs/
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── SELF_HOSTING.md
│   └── superpowers/specs/      # this spec + future specs
├── web/                        # Phase 2 placeholder (React UI)
├── .env.example
├── README.md
├── LICENSE-MIT
└── LICENSE-APACHE
```

Cargo workspace with three members. `aftercode-core` has zero I/O dependencies so both CLI and server depend on it without pulling each other's deps.

---

## 4. `aftercode-core` (shared crate)

Single source of truth for the wire contract between CLI and backend. All types `serde::{Serialize, Deserialize}`.

### 4.1 Session context (CLI → backend)

```rust
pub struct SessionContext {
    pub project_id: String,
    pub language: Language,              // He | En
    pub episode_length_minutes: u8,     // 5 | 10 | 15
    pub collected_at: String,           // RFC3339
    pub events: Vec<CodingEvent>,
    pub changed_files: Vec<String>,     // paths only by default
    pub git_diff_summary: Option<String>,
    pub commit_messages: Vec<String>,
    pub terminal_errors: Vec<String>,
}

pub struct CodingEvent {
    pub event_type: EventType,          // UserPrompt | AgentResponse | FileChanged | TerminalError | GitDiff | Commit
    pub timestamp: String,              // RFC3339
    pub content: String,
    pub metadata: serde_json::Value,
}

pub enum Language { He, En }
```

### 4.2 Episode / topics / script types

Mirror the backend data model (§7). `LearningTopic`, `EpisodeScript { segments: Vec<ScriptSegment>, summary_points, quiz }`, `ScriptSegment { speaker: Speaker, text }`, `Speaker { Host, Expert }`, `EpisodeStatus` enum, `EpisodeSummary`/`EpisodeDetail` API DTOs.

### 4.3 Errors

`thiserror`-based `CoreError`. CLI and server wrap it in their own error enums.

---

## 5. CLI (`aftercode-cli`)

clap derive. Commands map to PRD §7.

### 5.1 Commands

| Command | Responsibility |
|---|---|
| `aftercode init` | Interactive setup: project name, language, episode length, optional hook install. Writes `.aftercode/config.json`, registers project with backend, creates default ignore rules, detects git repo. |
| `aftercode login <token>` | Store bearer token in `~/.config/aftercode/credentials.json` (0600). |
| `aftercode status` | Show project connection, git state, hook status, last episode. |
| `aftercode preview` | Collect today's context, run secret scan, print exactly what would be uploaded. No network. |
| `aftercode episode [--language he\|en] [--from today\|yesterday] [--length 5\|10\|15]` | Collect → secret-scan → upload → trigger generation → poll status → print title/topics/URL. |
| `aftercode ignore <pattern>` | Append to `ignore_paths` in config. |
| `aftercode open` | Open web UI URL. |

### 5.2 Modules

```
aftercode-cli/src/
├── main.rs               # clap parse → dispatch
├── config.rs             # .aftercode/config.json load/save
├── credentials.rs        # ~/.config/aftercode token store (0600)
├── collect/
│   ├── mod.rs            # orchestrates collectors → SessionContext
│   ├── git.rs            # diff, changed files, commits via git2
│   ├── hooks.rs          # read .aftercode/events/*.jsonl (Claude Code / Codex)
│   └── errors.rs         # optional terminal-error capture file
├── privacy/
│   ├── ignore.rs         # gitignore-style matching (ignore crate)
│   └── secrets.rs        # regex secret detection (.env, keys, tokens)
├── client.rs             # reqwest HTTP client, bearer auth, typed calls
└── ui.rs                 # terminal output, spinners, status polling
```

### 5.3 Config files

`.aftercode/config.json` (per project, PRD §6):
```json
{
  "project_id": "proj_123",
  "project_name": "owla-backend",
  "language": "he",
  "episode_length_minutes": 10,
  "api_base_url": "http://localhost:8080",
  "providers": { "tts": "elevenlabs" },
  "privacy": {
    "ignore_paths": [".env", "node_modules", "dist", "build"],
    "send_raw_code": false,
    "send_diffs": true
  }
}
```

`~/.config/aftercode/credentials.json` (global, mode 0600): `{ "token": "ak_..." }`.

### 5.4 Privacy (PRD §8, §23 Risk 2)

Default: send paths + diffs + errors + prompts/agent summaries. Never send `.env`, secrets, keys, or `ignore_paths` matches. Secret scan runs on all content before upload; `preview` shows the exact payload. Secrets found → redacted + warned, never uploaded.

### 5.5 Hook capture

Claude Code / Codex hooks append JSONL events to `.aftercode/events/<date>.jsonl`. `init` can install hook config (writes to the agent's settings to call a tiny `aftercode hook-event` shim, or documents manual setup). The `hooks.rs` collector reads and parses these files for the requested day. MVP: file-based, append-only.

---

## 6. Backend (`aftercode-server`)

Axum + sqlx (Postgres) + Tokio. Bearer-token auth middleware.

### 6.1 Endpoints (PRD §18)

```
GET  /healthz
GET  /me                              # token → user
POST /cli/register-project            # init
POST /cli/upload-session              # store SessionContext → coding_session
POST /cli/generate-episode            # create episode (queued) + spawn pipeline
GET  /cli/episode-status/:id          # poll status
GET  /episodes                        # list (UI)
GET  /episodes/:id                    # detail (UI)
POST /episodes/:id/retry              # re-run failed
GET  /projects, POST /projects, GET /projects/:id
```

`cli/generate-episode` accepts the request shape in PRD §19 and returns `{ episode_id, status }`.

### 6.2 Modules

```
aftercode-server/src/
├── main.rs               # config, db pool, router, listen
├── config.rs             # env: DATABASE_URL, ANTHROPIC_API_KEY, OPENAI_API_KEY,
│                         #      ELEVENLABS_*, LLM_PROVIDER, BLOB_STORE, S3_*
├── auth.rs               # bearer-token extractor / middleware
├── db/                   # sqlx queries, models
├── routes/               # one module per endpoint group
├── pipeline/
│   ├── mod.rs            # run(session) -> Episode, drives stages + status updates
│   ├── normalize.rs      # SessionContext → clean prompt context
│   ├── topics.rs         # LLM: extract topics (structured JSON)
│   ├── gaps.rs           # LLM: knowledge-gap detection
│   ├── rank.rs           # select/rank topics (PRD §10)
│   ├── script.rs         # LLM: two-speaker script (structured JSON), lang-specific prompts
│   ├── tts.rs            # per-segment ElevenLabs PCM
│   └── assemble.rs       # PCM concat + silence + mp3lame encode
├── providers/
│   ├── llm.rs            # LlmProvider trait
│   ├── anthropic.rs      # default; claude-opus-4-8, output_config.format
│   ├── openai.rs         # alt
│   └── tts.rs            # TtsProvider trait + ElevenLabsProvider
├── storage/
│   ├── blob.rs           # BlobStore trait
│   ├── localfs.rs        # dev
│   └── s3.rs             # R2/S3 (aws-sdk-s3 or rusoto-free s3 client)
└── worker.rs             # tokio::spawn wrapper, status transitions, error capture
```

### 6.3 Provider traits

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> Result<Vec<LearningTopic>>;
    async fn write_script(&self, topics: &[LearningTopic], opts: &ScriptOpts) -> Result<EpisodeScript>;
}

#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(&self, text: &str, voice: VoiceRole, lang: Language) -> Result<PcmAudio>;
}

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<String /* url */>;
}
```

Anthropic impl uses `claude-opus-4-8`, `thinking: {type:"adaptive"}`, and `output_config.format` (JSON schema) so topic/script stages return schema-valid JSON — validated against `aftercode-core` types. OpenAI impl uses JSON mode. Provider selected at startup from `LLM_PROVIDER` env.

### 6.4 Generation execution (PRD §16)

`POST /cli/generate-episode`:
1. Insert `podcast_episodes` row, status `queued`.
2. `tokio::spawn(run_pipeline(episode_id, pool, providers))`.
3. Return `{ episode_id, status: "queued" }` immediately.

Worker transitions status: `queued → extracting_topics → writing_script → generating_audio → ready` (or `failed` + `error` message). Each transition is a DB update. CLI polls `GET /cli/episode-status/:id`.

### 6.5 Audio assembly (PRD §14)

1. For each `ScriptSegment`, call `TtsProvider::synthesize` → `PcmAudio` (i16 samples, 44.1kHz mono) using host/expert voice per speaker.
2. Concatenate sample buffers, inserting silence (zero samples) between segments: same-speaker 300ms, speaker-switch 500–700ms, section transition 900–1200ms.
3. Encode the combined PCM to MP3 with `mp3lame-encoder`.
4. `BlobStore::put` → `audio_url`. Store on episode, set `ready`, compute `duration_seconds`.

---

## 7. Data Model (PRD §17)

sqlx migrations. Tables: `users`, `projects`, `coding_sessions`, `coding_events`, `learning_topics`, `podcast_episodes`. `podcast_episodes.script_json` and `topics_json` are JSONB. `podcast_episodes.status` is a Postgres enum (`queued|extracting_topics|writing_script|generating_audio|ready|failed`). FKs: project→user, session→project, event→session, topic→session, episode→(user,project,session).

Auth: a `users` row holds a hashed personal token (or token table). MVP self-host: a server CLI subcommand / seed creates a user + token; `aftercode login` stores it.

---

## 8. Hebrew / English (PRD §13)

Language carried on `SessionContext` and episode. `script.rs` selects a language-specific system prompt. Hebrew prompt encodes PRD §13 rules: natural Israeli developer speech, keep technical terms in English where natural, avoid over-formal Hebrew, not word-for-word translation. ElevenLabs voice IDs configurable per language (`ELEVENLABS_HOST_VOICE_ID_HE`, etc., falling back to default voices).

---

## 9. Errors & Testing

**Errors:** `thiserror` in core; server maps pipeline errors → episode `failed` with a user-facing message (PRD §16). CLI maps network/auth/collection errors to clear terminal messages.

**Testing:**
- Unit tests per collector (git via a temp repo fixture; hooks via sample JSONL; secret scanner against known secret patterns).
- Unit tests per pipeline stage with **mocked** `LlmProvider` / `TtsProvider` / `BlobStore` (traits make this clean).
- `assemble.rs` tested on synthetic PCM (verify gap insertion + MP3 output is non-empty/decodable).
- One integration test: fake `SessionContext` → mocked providers → asserts a `ready` episode with MP3 bytes and a valid script JSON.
- HTTP route tests via `axum::Router` with an in-memory/throwaway test DB (sqlx test transactions).

---

## 10. Configuration / Secrets

Nothing hardcoded. `.env.example` documents: `DATABASE_URL`, `LLM_PROVIDER` (`anthropic|openai`), `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `ELEVENLABS_API_KEY`, `ELEVENLABS_HOST_VOICE_ID`, `ELEVENLABS_EXPERT_VOICE_ID` (+ optional `_HE`/`_EN` variants), `BLOB_STORE` (`localfs|s3`), `S3_*`, `APP_PUBLIC_URL`, `BIND_ADDR`.

---

## 11. Open-Source Posture

- Dual MIT/Apache-2.0 license.
- `README.md`: what it is, quickstart (self-host backend + install CLI), screenshots placeholder.
- `docs/SELF_HOSTING.md`: Postgres + env + run backend + create token + `aftercode login`.
- `docs/ARCHITECTURE.md`: this design, condensed, for contributors.
- CI (GitHub Actions): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` against a Postgres service container.

---

## 12. Full-Product Roadmap (planned, built later)

| Phase | Scope | Cycle |
|---|---|---|
| **Phase 1 (this spec)** | Rust CLI + API backend (collect → topics → script → ElevenLabs audio → episode storage) | spec → plan → build now |
| **Phase 2** | React + Vite Web UI (PRD §15): login, episodes list, episode detail w/ audio player, transcript, topics, takeaways, quiz. Consumes `/episodes` + `/episodes/:id`. | own spec → plan → build |
| **Phase 3** | PRD §22: deeper Claude Code/Codex integration, auto-daily generation, email notify, download/share, VS Code/Cursor extension, team workspace, learning history, spaced repetition, private RSS feed. | own specs per feature |

---

## 13. Phase 1 Acceptance (PRD §21)

- `aftercode init|status|preview|episode` work; `--language he|en` honored.
- Backend accepts session upload, extracts topics, writes structured script, calls ElevenLabs, concatenates segments to one MP3, stores episode metadata + audio URL.
- Episode uses two distinct voices; Hebrew and English both generate; speaker turns separated by silence gaps.
- Secret scanning + `preview` + ignore rules prevent secret/full-source upload by default.
- Mocked-provider integration test produces a `ready` episode end to end.
