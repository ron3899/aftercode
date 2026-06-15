# Aftercode

**Turn your daily AI coding sessions into personalized learning podcasts.**

Aftercode connects to your coding-agent workflow (Claude Code, Codex, Cursor, …), figures out the deeper technical topics behind what you built and debugged that day, and generates a short two-speaker podcast episode — in Hebrew or English — that teaches you the concepts behind the code you shipped.

> Turn vibe coding into real learning.

Open source (MIT OR Apache-2.0), CLI-first, self-hostable.

## How it works

```
aftercode episode
   │  auto-detect your coding-agent session (Claude Code / Codex / Cursor) →
   │    your prompts + what the agent did/explained
   │  + the real git diff (the Redis client, RabbitMQ config, new deps it added)
   │  scan for secrets, apply ignore rules, redact, cap size
   ▼
backend pipeline
   normalize → extract topics → detect knowledge gaps → rank →
   write two-speaker script → ElevenLabs TTS per segment →
   concat + silence gaps → MP3 → store → episode
```

## Repository layout

```
crates/aftercode-core    shared types (session context, episode, audio)
crates/aftercode-server  Axum API backend + generation pipeline
crates/aftercode-cli     the `aftercode` CLI
migrations/              Postgres schema
```

## Quickstart (self-host)

Requires Rust (stable) and Postgres.

```bash
# 1. Database
createdb aftercode   # or use Docker: docker run -d -e POSTGRES_USER=aftercode \
                     #   -e POSTGRES_PASSWORD=aftercode -e POSTGRES_DB=aftercode -p 5432:5432 postgres:16
psql "$DATABASE_URL" -f migrations/0001_init.sql

# 2. Backend
cp .env.example .env   # fill in keys (or set LLM_PROVIDER=mock, BLOB_STORE=localfs to try without keys)
cargo run -p aftercode-server seed-user you@example.com   # prints a token: ak_...
cargo run -p aftercode-server                             # serves on :8080

# 3. CLI
cargo install --path crates/aftercode-cli   # installs `aftercode`
aftercode login ak_...
cd your-project && aftercode init
aftercode preview
aftercode episode --language en
```

`aftercode episode` prints the generated title and the audio link
(`/static/episodes/<id>.mp3`).

### Reliable session capture

Episodes are built from your agent **session** + the git **diff**. Aftercode auto-detects
the session from Claude Code / Codex / Cursor on-disk storage, but that can miss (e.g. a
Cursor run from a subfolder). Two safeguards:

- **Hand the transcript in directly** — the agent that ran the CLI already has the session,
  so pipe it: `your-transcript | aftercode episode --transcript -` (accepts a Claude Code
  `.jsonl`, simple `{"role","text"}` JSONL, or plain text). This skips disk auto-detection.
- **Thin-episode guardrail** — if no session conversation is captured, `episode` refuses
  (so you never get an episode about a lone `package-lock.json`). Override with `--allow-thin`.
  Run `aftercode preview` first to see what was collected.

## Configuration

All secrets/config via env (see `.env.example` and `docs/SELF_HOSTING.md`):
`DATABASE_URL`, `LLM_PROVIDER` (`anthropic` default / `openai` / `mock`), `ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`, `ELEVENLABS_API_KEY` + voice IDs, `BLOB_STORE` (`localfs` / `s3`), `S3_*`.

The default LLM model is `claude-opus-4-8` with structured JSON output for evidence-based
topic and script generation. Providers are pluggable behind a trait.

## Privacy

Aftercode sends your agent-session transcript (prompts + agent messages + the commands/edits
it ran) and the **git diff hunks** — never full source files, `.env`, secrets, or ignored
paths. A regex secret scanner redacts anything that looks like a key/token before upload,
payload size is capped, and `aftercode preview` shows exactly what would be sent (including a
per-type event breakdown and which agent was detected).

## Status

Phase 1 (this repo): Rust CLI + API backend, end-to-end with mocked or real providers.
Phase 2: web UI. Phase 3: editor extensions, auto-daily, private RSS. See
`docs/superpowers/specs` and `docs/superpowers/plans`.

## License

Licensed under either of MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`) at your option.
