# Aftercode Architecture

Condensed for contributors. Full design: `docs/superpowers/specs/2026-06-14-aftercode-cli-backend-design.md`.

## Workspace

One Cargo workspace, three crates:

- **`aftercode-core`** — pure data types, no I/O. The wire contract between CLI and backend:
  `SessionContext`/`CodingEvent` (CLI → backend), `LearningTopic`/`EpisodeScript`/`Speaker`/
  `EpisodeStatus`/`EpisodeDetail`, and `PcmAudio` + gap constants. Defined once, shared.
- **`aftercode-server`** — Axum + sqlx(Postgres) + Tokio. API + generation pipeline.
- **`aftercode-cli`** — clap. Collects sessions, applies privacy, drives the backend.

## Backend

### Request flow

```
POST /cli/generate-episode
  → insert podcast_episodes row (status=queued)
  → tokio::spawn(worker)            (returns {episode_id, status} immediately)
  → CLI polls GET /cli/episode-status/:id
```

### Pipeline (`src/pipeline`)

```
normalize → extract_topics (LLM) → rank → write_script (LLM) →
synthesize per segment (TTS) → concat_with_gaps → encode_mp3 → BlobStore::put
```

`worker.rs` runs the pipeline and funnels stage statuses through an **ordered mpsc channel**,
so the terminal `ready`/`failed` write always lands after every progress write (fire-and-forget
spawns would otherwise clobber the final status — this was a real bug caught by the e2e test).

### Swappable traits

- `LlmProvider` — `AnthropicProvider` (default, `claude-opus-4-8`, structured JSON via
  `output_config.format`), `OpenAiProvider` (JSON mode), `MockLlm` (tests/offline).
- `TtsProvider` — `ElevenLabsProvider` (per-segment PCM 44.1kHz), `MockTts`.
- `BlobStore` — `LocalFs` (served at `/static`), `S3Store` (R2/S3), `MockBlob`.

Selected at startup from env in `state.rs`. Tests inject mocks via `AppState::for_test`.

### Audio

Pure Rust: request PCM per segment, concatenate i16 samples inserting silence gaps
(same-speaker 300ms, switch 600ms), encode the combined buffer to MP3 with `mp3lame-encoder`.
No external binary.

### Auth

Bearer token. `hash_token` (SHA-256, base64) stored in `users.token_hash`; `AuthUser` extractor
validates per request. `aftercode-server seed-user <email>` mints a token for self-hosters.

### Data model

`users, projects, coding_sessions, coding_events, learning_topics, podcast_episodes`
(`migrations/0001_init.sql`). Script/topics stored as JSONB; status is a Postgres enum.

## CLI

- `collect/` — `git` (git2: changed files, diff summary, commits), `hooks` (read
  `.aftercode/events/*.jsonl`), `errors` (read `.aftercode/errors.log`).
- `privacy/` — `secrets` (regex scan + line redaction), `ignore` (gitignore-style matcher).
- `collect::build` assembles a `SessionContext`, dropping ignored paths and redacting secrets
  from every free-text field before anything leaves the machine.
- `client.rs` — bearer-authed reqwest calls. `commands/` — init/login/status/preview/episode/
  ignore/open. Config in `.aftercode/config.json`; token in `~/.config/aftercode/credentials.json`
  (0600).

## Testing

Unit tests per collector, privacy module, and pipeline stage (mocked providers). One backend
integration test drives `POST /cli/generate-episode` through the worker to a `ready` episode with
real MP3 bytes. Env/cwd-mutating tests are serialized with `serial_test`.
