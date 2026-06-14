# PRD — Aftercode: Personalized Coding Podcast Generator

## 1. Product Summary

Aftercode is a CLI-first product that connects to a developer's daily coding-agent workflow and turns their sessions into personalized learning podcasts. It analyzes what the user discussed, built, debugged, and shipped with their coding agent, identifies the deeper technical topics behind the work, and generates a short podcast episode in Hebrew or English.

The goal is to help developers go deeper into the topics they touched during "vibe coding" instead of only copying, accepting, or shipping AI-generated code without fully understanding it.

## 2. Core Problem

Developers using coding agents move very fast, but often do not deeply understand the concepts behind the code the agent helped them write (e.g. database locking behind a migration fix, event loops behind an async rewrite, state management behind a React bug fix, auth flow behind OAuth callback logic). The result: developers ship faster, but learning becomes shallow. Aftercode turns daily coding-agent sessions into personalized audio lessons.

## 3. Target User

**Primary ICP:** Developers, founders, indie hackers, and product engineers who use coding agents heavily — Cursor / Claude Code / Codex / Windsurf users, AI-assisted coding daily, non-senior builders who can ship with AI but want to understand the underlying concepts.

**Mindset:** "I built this with AI, but I'm not sure I fully understand what happened." "I want to become better, not just faster."

## 4. Positioning

- One-liner: Aftercode turns your daily AI coding sessions into personalized learning podcasts.
- Stronger: Turn vibe coding into real learning.
- Alt: Learn from the code you shipped today.

## 5. MVP Scope

Includes: CLI install + local setup; collection of coding-session context; topic generation; podcast script generation; ElevenLabs audio with two voices (host + expert); Hebrew + English; minimal web UI listing podcasts; episode detail page (player, topics, summary, transcript).

Excludes: full Cursor/VS Code extension, mobile app, advanced team features, real-time generation, public feed, editing studio, voice cloning, marketplace integrations.

## 6. User Flow

**First-time setup:**
```
npm install -g aftercode   # (Phase 1: cargo/binary install for the Rust CLI)
aftercode init
```
`init` asks: project name? language (Hebrew/English)? episode length (5/10/15 min)? connect Claude Code hooks? connect Codex hooks? — then writes `.aftercode/config.json`:
```json
{
  "project_id": "proj_123",
  "project_name": "owla-backend",
  "language": "he",
  "episode_length_minutes": 10,
  "providers": { "tts": "elevenlabs" },
  "privacy": {
    "ignore_paths": [".env", "node_modules", "dist", "build"],
    "send_raw_code": false,
    "send_diffs": true
  }
}
```

**Daily usage:** user works normally with a coding agent, then runs `aftercode episode`. The CLI collects the day's context (agent prompts, agent responses if available, git diff, changed files, commit messages, terminal errors, hook events), sends a normalized session to the backend, which generates learning topics, knowledge gaps, outline, two-speaker script, audio (ElevenLabs), and an episode page. The CLI prints the title, topics, and URL.

## 7. CLI Requirements

- `aftercode init` — create config, authenticate, register project, detect git, optionally install hooks, default ignore rules.
- `aftercode status` — project connection + collector status.
- `aftercode episode` — generate episode from recent activity. Flags: `--language en|he`, `--from today|yesterday`, `--length 5|15`.
- `aftercode preview` — show what will be sent before upload (changed files, detected errors, potential topics).
- `aftercode ignore <pattern>` — manage ignored files/folders.
- `aftercode open` — open web UI.

## 8. Data Collection

MVP sources: git diff, changed-files metadata, commit messages, CLI session notes, Claude Code hooks (if available), Codex hooks (if available).

Event shape:
```json
{ "event_type": "user_prompt | agent_response | file_changed | terminal_error | git_diff | commit",
  "timestamp": "2026-06-14T19:00:00Z", "content": "...", "metadata": {} }
```

**Privacy (default):** send file paths, diffs, error messages, user prompts and agent summaries (if available). Do NOT send `.env`, secrets, keys, credentials, or ignored files. By default do not upload full source files. Later: `aftercode privacy strict|balanced|full-context`.

## 9. Backend Pipeline

```
Coding session → Normalize context → Extract topics → Detect knowledge gaps →
Rank topics → Create podcast outline → Write podcast script →
Generate audio with ElevenLabs → Store episode → Render episode page
```

## 10. Topic Extraction

Receives session context, returns structured topics:
```json
{ "topics": [ {
  "title": "Production-safe Postgres indexes",
  "summary": "The user modified an Alembic migration to create an index concurrently and avoid production locking.",
  "evidence": ["Migration used postgresql_concurrently=True", "autocommit_block was added", "Terminal mentioned transaction limitation"],
  "knowledge_gap": "The user may not understand why CREATE INDEX CONCURRENTLY cannot run inside a normal Alembic transaction.",
  "difficulty": "intermediate", "priority": "high" } ] }
```
Prefer topics that are repeated, tied to actual code changes, conceptually deep, skill-improving, and related to bugs/architecture/infra/databases/APIs/frontend state/auth/performance/deployment.

## 11. Knowledge Gap Detection

Detect moments where the agent solved something the user may not fully understand. Signals: vague prompts ("fix this", "why is this failing?"); complex agent change; change accepted without follow-up; terminal error vanished after agent change; code involves a known deeper concept; same issue recurred.

## 12. Podcast Script Generation

Two-speaker conversation. **Host:** guides, asks simple questions, connects to the session. **Expert:** explains the concept clearly and practically. Tone: calm mentor, practical, direct, not overproduced, not cheesy, focused on the user's actual session.

10-min structure: intro 20–30s; what happened in your session 1–2m; deeper concept 3–4m; why the fix worked 2m; how to think next time 1–2m; summary 30s; optional quiz 20s.

Script stored as structured JSON (segments with speaker+text, summary_points, quiz), e.g.:
```json
{ "title": "Why your Alembic migration almost locked production", "language": "en",
  "segments": [ {"speaker":"host","text":"..."}, {"speaker":"expert","text":"..."} ],
  "summary_points": ["..."],
  "quiz": { "question": "...", "answer": "..." } }
```

## 13. Language Support

Hebrew + English. Default set in `init`; override per episode with `--language`. Hebrew episodes: natural (not word-for-word translated), keep technical terms in English where natural, mixed Hebrew-English like Israeli developers speak, avoid over-formal Hebrew. English: clear, practical, no corporate language, like a technical mentor.

## 14. ElevenLabs Integration

Two voices (`host_voice_id`, `expert_voice_id`) in backend config (`ELEVENLABS_API_KEY`, `ELEVENLABS_HOST_VOICE_ID`, `ELEVENLABS_EXPERT_VOICE_ID`).

Generate by segment, not one blob: for each segment call TTS with the correct voice, save segment audio, add pauses between segments, concatenate into one final MP3, upload. Pause rules: same-speaker 300ms, speaker switch 500–700ms, section transition 900–1200ms. Output: MP3 44.1kHz, stored in S3 / Cloudflare R2 / equivalent.

## 15. Minimal Web UI

Pages: login (magic link or email/password for MVP; Phase 1 uses token auth, UI is Phase 2); podcasts list (title, date, project, language, duration, topics, play, status); episode detail (player, title, date, project, topics, summary, transcript, key takeaways, quiz).

## 16. Episode Statuses

`queued → extracting_topics → writing_script → generating_audio → ready → failed`. On failure show a clear error ("Audio generation failed. Retry episode.").

## 17. Backend Data Model

`users(id, email, created_at)`; `projects(id, user_id, name, default_language, created_at)`; `coding_sessions(id, project_id, user_id, source, started_at, ended_at, raw_context_ref, summary, created_at)`; `coding_events(id, session_id, event_type, content, metadata, created_at)`; `learning_topics(id, session_id, title, summary, evidence, knowledge_gap, difficulty, priority, created_at)`; `podcast_episodes(id, user_id, project_id, session_id, title, language, status, duration_seconds, audio_url, script_json, transcript_text, summary, topics_json, created_at, updated_at)`.

## 18. API Endpoints

Auth: `POST /auth/login`, `POST /auth/logout`, `GET /me`. Projects: `POST /projects`, `GET /projects`, `GET /projects/:id`. Sessions: `POST /sessions`, `GET /sessions/:id`. Episodes: `POST /episodes/generate`, `GET /episodes`, `GET /episodes/:id`, `POST /episodes/:id/retry`. CLI: `POST /cli/register-project`, `POST /cli/upload-session`, `POST /cli/generate-episode`, `GET /cli/episode-status/:id`.

## 19. Episode Generation API Example

Request:
```json
{ "project_id": "proj_123", "language": "he", "episode_length_minutes": 10,
  "session_context": {
    "changed_files": ["migrations/versions/add_agent_sessions_agent_started_index.py"],
    "git_diff_summary": "Added postgresql_concurrently=True and autocommit_block",
    "terminal_errors": ["CREATE INDEX CONCURRENTLY cannot run inside a transaction block"],
    "agent_messages": ["The migration should use autocommit_block because concurrent index creation cannot run inside a transaction."] } }
```
Response: `{ "episode_id": "ep_123", "status": "queued" }`.

## 20. Success Criteria

User can install CLI, init a project, generate a podcast from a session, see it in the web UI, with two-speaker audio, in Hebrew or English, explaining real topics from their activity — and feel it helped them understand something they touched while coding.

## 21. MVP Acceptance Criteria

**CLI:** init/status/preview/episode work; `--language he|en` supported. **Backend:** accepts session uploads; extracts topics; creates structured script; calls ElevenLabs; concatenates segments to one MP3; stores episode metadata + audio URL. **UI:** list, detail, play, transcript, topics, takeaways. **Audio:** two voices; Hebrew + English work; natural speaker separation; final episode stored and playable.

## 22. Future Versions

- **V1.1:** better Claude Code/Codex integration, auto-daily generation, email when ready, download MP3, share link.
- **V1.2:** VS Code/Cursor extension, full transcript capture, team workspace, learning history, topic recommendations.
- **V1.3:** personalized curriculum, spaced repetition, post-episode quizzes, "explain deeper" follow-up episodes, private RSS feed.

## 23. Key Product Risks

1. Audio feels like a gimmick → tie content deeply to the real session ("I finally understand what I shipped"). 2. Privacy → CLI preview, ignore rules, no full-source upload by default, secret scanning, local config. 3. Weak topic extraction → evidence-based extraction; every topic cites changed files/errors/diffs/prompts/agent actions. 4. Hebrew unnatural → Hebrew-specific script rules; keep tech terms in English; avoid over-formal Hebrew. 5. ElevenLabs cost → 5–15 min episodes, segment generation, cache segments, per-user usage limits.

## 24. Recommended Tech Stack

**CLI:** Rust (clap). Local config `.aftercode/config.json`. **Backend:** Rust (Axum) + PostgreSQL + in-process worker (Redis as scale path) + object storage (S3/R2) + LLM (Anthropic default, OpenAI alt, pluggable) + TTS (ElevenLabs). **Frontend (Phase 2):** React + Vite, audio player, list + detail.

> Note: original PRD suggested Node/TypeScript CLI and FastAPI/Node backend. Phase 1 design (see `superpowers/specs/2026-06-14-aftercode-cli-backend-design.md`) selects Rust for both, in one Cargo workspace, per project decision.

## 25. Initial Build Plan (Milestones)

1. CLI skeleton — init, status, config, token storage, project registration. 2. Session collection — git diff/changed files/commit collectors, manual context upload, preview. 3. Backend generation — upload endpoint, topic extraction, gap detection, script generation, episode model. 4. ElevenLabs audio — two-voice segment TTS, concatenation, MP3 storage, status updates. 5. Minimal UI — list, detail, player, transcript, topics/takeaways. 6. Polish — Hebrew/English quality, retry, error messages, privacy preview.

## 26. Demo Scenario

User fixes a backend migration with a coding agent, runs `aftercode episode --language en`. Aftercode detects the Alembic change, concurrent index creation, autocommit block, production-locking risk, and generates "Why your Alembic migration almost locked production" — a two-speaker podcast explaining what happened, why it matters, and how to think next time.

## 27. Core Principle

Aftercode should not feel like a generic podcast generator. It should feel like a personal technical mentor that watched what you built today and turned it into a short, useful lesson.
