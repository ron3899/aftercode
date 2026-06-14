# Self-Hosting Aftercode

Aftercode is a Rust workspace: an API backend (`aftercode-server`) and a CLI (`aftercode`).

## Prerequisites

- Rust (stable) — https://rustup.rs
- PostgreSQL 14+

## 1. Database

```bash
# Docker
docker run -d --name aftercode-pg \
  -e POSTGRES_USER=aftercode -e POSTGRES_PASSWORD=aftercode -e POSTGRES_DB=aftercode \
  -p 5432:5432 postgres:16

export DATABASE_URL=postgres://aftercode:aftercode@localhost:5432/aftercode
psql "$DATABASE_URL" -f migrations/0001_init.sql
```

## 2. Environment

Copy `.env.example` to `.env` and fill in:

| Variable | Purpose |
|---|---|
| `DATABASE_URL` | Postgres connection string (required) |
| `BIND_ADDR` | Listen address, default `0.0.0.0:8080` |
| `APP_PUBLIC_URL` | Public base URL used in audio/episode links |
| `LLM_PROVIDER` | `anthropic` (default), `openai`, or `mock` |
| `ANTHROPIC_API_KEY` | Required when `LLM_PROVIDER=anthropic` (model `claude-opus-4-8`) |
| `OPENAI_API_KEY` | Required when `LLM_PROVIDER=openai` |
| `ELEVENLABS_API_KEY` | ElevenLabs key; if unset, a silent mock TTS is used |
| `ELEVENLABS_HOST_VOICE_ID` | Voice for the host speaker |
| `ELEVENLABS_EXPERT_VOICE_ID` | Voice for the expert speaker |
| `BLOB_STORE` | `localfs` (default) or `s3` |
| `LOCALFS_DIR` | Audio dir for `localfs`, served at `/static` |
| `S3_BUCKET` | Bucket for `s3` (uses standard AWS env credentials; works with Cloudflare R2 via `AWS_ENDPOINT_URL`) |

**Try it with no API keys:** set `LLM_PROVIDER=mock`, `BLOB_STORE=localfs`, leave `ELEVENLABS_API_KEY` empty — the server generates a placeholder episode end-to-end.

## 3. Run the backend

```bash
# Create a user + personal token
cargo run -p aftercode-server seed-user you@example.com
# -> user you@example.com token: ak_xxxxxxxx

cargo run -p aftercode-server   # or build --release and run target/release/aftercode-server
```

## 4. Install + connect the CLI

```bash
cargo install --path crates/aftercode-cli   # provides `aftercode`
aftercode login ak_xxxxxxxx
cd your-project
aftercode init        # registers the project with the backend
aftercode preview     # see exactly what would be uploaded
aftercode episode --language en
```

## Hook events (optional)

The CLI reads agent events from `.aftercode/events/<YYYY-MM-DD>.jsonl` — newline-delimited
`CodingEvent` JSON (`event_type`, `timestamp`, `content`, `metadata`). Wire your coding agent's
hooks to append events there to enrich episodes with prompts and agent responses. Terminal
errors can be appended to `.aftercode/errors.log` (one per line).

## Notes

- Episode generation runs in-process (Tokio task) and updates a status machine in Postgres;
  the CLI polls `GET /cli/episode-status/:id`. Redis is a future scale path, not required.
- Audio is assembled in pure Rust (per-segment PCM → silence gaps → MP3), no `ffmpeg` needed.
