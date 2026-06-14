# Aftercode — Session-Aware Collection Design

**Date:** 2026-06-15
**Status:** Approved
**Scope:** Make the CLI generate episodes from the developer's **actual coding-agent session** (prompts + what the agent did) plus the **real git diff**, instead of only a one-line diff summary. Auto-detects which agent was used (Claude Code, Codex, Cursor).

## 1. Problem

Today `collect::build` sends a one-line `git_diff_summary` (`"N files changed, +X/-Y"`) + changed-file paths + commit messages. The LLM can't see *what* changed (that Redis/RabbitMQ were added) or *what the agent did/why*. Episodes are generic. We need the real session context.

## 2. `SessionReader` trait (CLI)

```rust
pub struct AgentSession {
    pub agent: String,            // "claude-code" | "codex" | "cursor"
    pub ended_at: i64,            // unix seconds of last activity (for newest-wins)
    pub events: Vec<CodingEvent>, // aftercode_core::session::CodingEvent
}

pub trait SessionReader {
    fn agent_name(&self) -> &'static str;
    /// Most-recent session for this project dir, if any. Best-effort: returns
    /// None on absence or any parse failure (never panics).
    fn latest_session(&self, project_dir: &Path) -> Option<AgentSession>;
}
```

Impls: `ClaudeCodeReader`, `CodexReader`, `CursorReader`. New agents = new impl.

## 3. Readers

### ClaudeCodeReader (JSONL)
- Path: `~/.claude/projects/<encoded-cwd>/<session>.jsonl`. `encoded-cwd` = repo absolute path with every char that is not alphanumeric replaced by `-` (Claude Code's scheme; in practice `/` and `.` → `-`).
- Current session = the `*.jsonl` in that dir with newest mtime; `ended_at` = that mtime.
- Per line (JSON): `type` = `user` | `assistant`; `message.content` is a string or an array of blocks:
  - text block → `UserPrompt` (user) / `AgentResponse` (assistant)
  - `tool_use` with `name`=`Bash` → command event: `EventType::AgentResponse` content `"$ <command>"` (captures `docker compose up redis`, `cargo add lapin`)
  - `tool_use` `Edit`/`Write`/`NotebookEdit` → `FileChanged` with the `file_path` input
- Skip `tool_result` blocks (noisy/large).

### CodexReader (JSONL)
- Path: newest `~/.codex/sessions/**/rollout-*.jsonl` whose recorded `cwd` matches the project dir (Codex records cwd in the session header line); fallback to newest file mtime.
- Codex line schema: events with `type` like `message` (role user/assistant) and `function_call`/`local_shell_call` (shell commands). Map: user message → `UserPrompt`, assistant message → `AgentResponse`, shell call → command event, file patches → `FileChanged`.

### CursorReader (SQLite, best-effort)
- Cursor stores per-workspace state in `~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/state.vscdb` (SQLite). `~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/workspace.json` has `"folder": "file://<path>"` — match against project dir to find the hash.
- Open `state.vscdb` read-only; table `ItemTable(key TEXT, value BLOB)`. Chat/composer JSON lives under keys such as `composer.composerData`, `aiService.prompts`, `workbench.panel.aichat.view.aichat.chatdata`. Parse best-effort: pull prompt texts and response texts into `UserPrompt`/`AgentResponse`.
- Schema is undocumented and version-fragile → the whole reader is wrapped; any error → `None` + a logged warning. `ended_at` from blob timestamps if present, else file mtime.

## 4. Auto-detection (registry)

```rust
pub fn detect_best(project_dir, forced: Option<&str>) -> Option<AgentSession>
```
- Build all readers; if `forced` (`--agent`) set, use only that one.
- Otherwise call `latest_session` on each; pick the highest `ended_at`. Tie / both within a few seconds → prefer richer transcript (claude-code/codex over cursor).
- Returns `None` if no agent session found (caller falls back to diff-only).

## 5. Full git diff (git.rs)

- Add `diff_hunks: Vec<(String /*path*/, String /*patch*/)>` to `GitData`, produced from the same `git2` diff via `Diff::print(DiffFormat::Patch, ...)` grouped per file.
- Skip files matching `ignore_paths` and binary deltas.

## 6. Merge into SessionContext (collect/mod.rs)

`build()` now:
1. `git = git::collect(...)` (paths, summary, commits, **hunks**).
2. `session = registry::detect_best(cwd, agent_flag)`.
3. `events` = session events (if any) ++ one `GitDiff` event per non-ignored diff hunk (`content` = `"<path>\n<patch>"`).
4. Redact secrets from every event's content; drop diff hunks for ignored paths.
5. Apply **size caps**: per-event char cap (default 8000) and total payload cap (default ~150 000 chars) — keep most-recent events, append `[…truncated N events/chars]` marker.
6. `changed_files`/`git_diff_summary`/`commit_messages` unchanged (metadata).
7. New `--agent` flag on `episode`; `language`/`from`/`length` unchanged.

No backend change: `normalize` already flattens all events into the prompt.

## 7. CLI surface

- `aftercode episode [--agent claude-code|codex|cursor] ...`
- `aftercode preview` — prints detected agent + event counts by type + diff files, before upload.
- `aftercode status` — shows which agent session is detected for this repo.

## 8. Privacy

- `send_raw_code` stays `false`: we send diffs + transcript text, never whole source files.
- Secret scanner runs on all event content. `ignore_paths` honored for diffs.
- Caps bound payload size/cost. `preview` shows exactly what is sent.

## 9. Errors & testing

- Readers are total functions returning `Option` — absence or malformed data → `None`, never a crash; `episode` always degrades to diff-only.
- Tests:
  - ClaudeCodeReader + CodexReader against fixture `.jsonl` → asserts prompt/agent/bash/file-edit events.
  - CursorReader against a hand-built tiny `state.vscdb` → best-effort parse; malformed-blob test → graceful `None`.
  - encoded-cwd mapping unit test.
  - `detect_best`: two readers, different `ended_at` → newest wins; none → `None`.
  - git diff hunks: temp repo change → hunk text present; ignored file excluded; binary skipped.
  - collect merge: secret line redacted; total/per-event caps enforced (truncation marker present).
- New dep: `rusqlite = { features = ["bundled"] }` (CLI only).

## 10. Out of scope

Windsurf and other agents (future `SessionReader` impls). No web UI changes. No backend changes.
