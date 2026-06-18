# Security Policy

## Supported versions

Aftercode is pre-1.0; only the latest release on `main` receives security fixes.

## Reporting a vulnerability

**Do not open a public issue for security problems.**

Use GitHub's private reporting: **Security → Advisories → Report a vulnerability** on this repo, or email the maintainer listed on the GitHub profile. Include reproduction steps and the affected version/commit. Expect an acknowledgement within a few days.

## Threat model (important for self-hosters)

Aftercode is built as a **single-user, self-hosted** tool:

- **Loopback by default.** The server binds to `127.0.0.1`. The local browser-approval login has **no identity check** — anyone who can reach the port can mint a token. Do **not** bind to `0.0.0.0` or expose it publicly without putting real authentication (a reverse proxy with auth, VPN, etc.) in front of it.
- **One user, all episodes.** Any valid token can read every episode. Per-user scoping is deferred to a future multi-user release.
- **Your API keys** (Anthropic / OpenAI / ElevenLabs) are set in the web UI and stored in the SQLite DB (or, optionally, in `.env`, which is gitignored — never commit it). The DB is **not encrypted at rest** — it's a single-user local file; protect it with normal filesystem permissions and don't expose the data volume. Keys are masked on read and never returned to the client.
- **Audio is served unauthenticated** from `/static` by design (so generated MP3s are shareable). Don't put secrets in episode content.

If you deploy outside these assumptions, you own the additional hardening.
