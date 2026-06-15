# Session handoff (`--transcript`) + thin-episode guardrail

**Date:** 2026-06-15
**Status:** Approved (Ron)

## Problem

Episodes are built from the agent **session** + the git **diff**. Today the session
is reconstructed by scraping each agent's on-disk storage (Claude Code JSONL keyed by
cwd-encoding; Cursor SQLite matched by exact `workspace.json` folder). This is fragile:
per-agent, per-version, keyed by exact path, and it guesses which conversation is "the
current one." When it misses (e.g. Cursor run from a subfolder), `episode` silently falls
back to git-diff-only and produces a useless episode about whatever lone file changed
(e.g. `package-lock.json`) — with no warning.

The key insight: **the agent that invokes the CLI already holds the full session.**
Reconstructing it from disk is backwards. Let the caller hand it over.

## Scope (this change)

1. **`aftercode episode --transcript <file|->`** — explicit session handoff.
   - `-` reads stdin; otherwise reads the given file.
   - When provided, this is the session source (disk auto-detection is skipped).
   - Agent-agnostic: works for Cursor/Codex/anything because the invoking agent supplies it.

2. **Thin-episode guardrail** — `episode` refuses when no session conversation was
   collected (no `UserPrompt`/`AgentResponse` events), unless `--allow-thin` is passed.
   Prints actionable guidance (pipe `--transcript -`, run from the workspace root, or
   `--allow-thin`). `preview` prints the same warning so it's visible before generating.

3. **Fix the printed link** — `episode` currently prints `…/episodes/{id}` (the auth-gated
   Phase-2 web route → 401 in a browser). Print the working audio link
   `…/static/episodes/{id}.mp3` instead.

Out of scope (later, per the layered design): Claude-Code hook auto-capture, MCP
`generate_episode` tool, improved subfolder-matching in the disk scrapers.

## Transcript input format (forgiving, in priority order)

`parse_transcript_input(text)`:
1. **Claude Code JSONL** — reuse `session::claude_code::parse_transcript`. If it yields
   events, use them. (Lets you pipe a real `~/.claude/projects/**/*.jsonl`.)
2. **Simple JSONL** — one object per line `{"role":"user"|"assistant","text":"..."}`.
   Trivial for any agent to emit.
3. **Plain text** — chunk into ≤`PER_EVENT_CHARS` pieces as `AgentResponse` events
   (so nothing is lost to the single-event cap).

Existing secret-redaction and size caps (`apply_caps`) still apply downstream.

## Acceptance

- `echo '{"role":"user","text":"hi"}' | aftercode episode --transcript -` builds a context
  whose events include that prompt.
- `aftercode episode` with no detectable session and only a diff exits non-zero with the
  guidance message; `--allow-thin` lets it proceed.
- Unit tests cover all three `parse_transcript_input` branches.
- Proof: pipe this session's trimmed transcript via `--transcript -` and regenerate the
  Eyal episode (rich, not package-lock).
