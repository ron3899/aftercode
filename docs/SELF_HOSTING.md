# Self-Hosting Aftercode

Aftercode is a Rust workspace: an API backend (`aftercode-server`) and a CLI (`aftercode`).

## Prerequisites

- Rust (stable) — https://rustup.rs
- Node (only to build the web UI)

## 1. Database

None to set up. Storage is a local **SQLite** file (`aftercode.db`), created and migrated
automatically the first time the server runs. To put it elsewhere, set `DATABASE_URL`
(e.g. `sqlite:///var/lib/aftercode/aftercode.db?mode=rwc`).

## 2. Environment

Copy `.env.example` to `.env` and fill in:

| Variable | Purpose |
|---|---|
| `DATABASE_URL` | SQLite URL, default `sqlite://aftercode.db?mode=rwc` (auto-created) |
| `BIND_ADDR` | Listen address, default `0.0.0.0:8080` |
| `APP_PUBLIC_URL` | Public base URL used in audio/episode links |
| `LLM_PROVIDER` | `anthropic` (default), `openai`, or `mock` |
| `ANTHROPIC_API_KEY` | Required when `LLM_PROVIDER=anthropic` (model `claude-opus-4-8`) |
| `OPENAI_API_KEY` | Required when `LLM_PROVIDER=openai` |
| `TTS_PROVIDER` | `elevenlabs` (default), `openai`, or `mock`. Falls back to mock if the chosen provider's key is missing |
| `ELEVENLABS_API_KEY` | ElevenLabs key (when `TTS_PROVIDER=elevenlabs`) |
| `ELEVENLABS_HOST_VOICE_ID` | ElevenLabs voice for the host speaker |
| `ELEVENLABS_EXPERT_VOICE_ID` | ElevenLabs voice for the expert speaker |
| `OPENAI_TTS_MODEL` | OpenAI TTS model when `TTS_PROVIDER=openai`, default `gpt-4o-mini-tts` (uses `OPENAI_API_KEY`) |
| `OPENAI_TTS_VOICE_HOST` | OpenAI voice for host, default `alloy` |
| `OPENAI_TTS_VOICE_EXPERT` | OpenAI voice for expert, default `onyx` |
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

- Episode generation runs in-process (Tokio task) and updates a status machine in SQLite;
  the CLI polls `GET /cli/episode-status/:id`. Redis is a future scale path, not required.
- Audio is assembled in pure Rust (per-segment PCM → silence gaps → MP3), no `ffmpeg` needed.
