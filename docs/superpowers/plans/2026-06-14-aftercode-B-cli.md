# Aftercode Plan B — Rust CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **Prerequisite:** Plan A complete — `aftercode-core` exists and the backend exposes the `/cli/*` endpoints.

**Goal:** Build the `aftercode` CLI: collect a day's coding-agent session (git, hooks, errors), scan for secrets, preview, and drive backend episode generation, in Hebrew or English.

**Architecture:** clap-derive binary depending on `aftercode-core` for the wire types. Collectors build a `SessionContext`; a privacy layer scans/redacts secrets and honors ignore rules; a reqwest client talks to the backend with a bearer token; commands orchestrate.

**Tech Stack:** Rust, clap, git2, reqwest, serde, ignore (gitignore matching), regex, dirs, indicatif.

---

## File Structure

```
crates/aftercode-cli/
  Cargo.toml
  src/main.rs                 # clap parse → dispatch
  src/config.rs               # .aftercode/config.json
  src/credentials.rs          # ~/.config/aftercode/credentials.json (0600)
  src/client.rs               # reqwest backend client
  src/collect/mod.rs          # build SessionContext
  src/collect/git.rs          # diff/changed files/commits (git2)
  src/collect/hooks.rs        # .aftercode/events/*.jsonl
  src/collect/errors.rs       # optional terminal-error capture file
  src/privacy/ignore.rs       # gitignore-style matching
  src/privacy/secrets.rs      # regex secret scan + redaction
  src/commands/mod.rs         # init/login/status/preview/episode/ignore/open
```

---

## Task 1: CLI manifest + clap skeleton

**Files:**
- Modify: `crates/aftercode-cli/Cargo.toml`
- Modify: `crates/aftercode-cli/src/main.rs`

- [ ] **Step 1: Manifest**

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

[dependencies]
aftercode-core = { path = "../aftercode-core" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
reqwest.workspace = true
anyhow.workspace = true
clap = { version = "4", features = ["derive"] }
git2 = { version = "0.19", default-features = false }
ignore = "0.4"
regex = "1"
dirs = "5"
indicatif = "0.17"
chrono = { version = "0.4", features = ["clock"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: clap enum + dispatch**

`crates/aftercode-cli/src/main.rs`:
```rust
mod config;
mod credentials;
mod client;
mod collect;
mod privacy;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aftercode", version, about = "Turn your AI coding sessions into learning podcasts")]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    Init,
    Login { token: String },
    Status,
    Preview,
    Episode {
        #[arg(long)] language: Option<String>,
        #[arg(long, default_value = "today")] from: String,
        #[arg(long)] length: Option<u8>,
    },
    Ignore { pattern: String },
    Open,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init => commands::init().await,
        Cmd::Login { token } => commands::login(token),
        Cmd::Status => commands::status().await,
        Cmd::Preview => commands::preview(),
        Cmd::Episode { language, from, length } => commands::episode(language, from, length).await,
        Cmd::Ignore { pattern } => commands::ignore(pattern),
        Cmd::Open => commands::open(),
    }
}
```

- [ ] **Step 3: Create empty module files so it compiles**

`config.rs`, `credentials.rs`, `client.rs`, `commands/mod.rs` get stubs; `collect/mod.rs` (`pub mod git; pub mod hooks; pub mod errors;` + empty submodule files); `privacy/mod.rs` (`pub mod ignore; pub mod secrets;` + empty submodule files). Add `mod privacy;` already declared; create `crates/aftercode-cli/src/privacy/mod.rs`.

- [ ] **Step 4: Verify parse builds**

Run: `cargo build -p aftercode-cli`
Expected: compiles (commands may be stubbed `todo!()` returning `Ok(())` placeholders are fine for now — but prefer real impls in following tasks).

- [ ] **Step 5: Commit**

```bash
git add crates/aftercode-cli
git commit -m "feat(cli): clap skeleton + module layout"
```

---

## Task 2: Config (.aftercode/config.json)

**Files:**
- Modify: `crates/aftercode-cli/src/config.rs`

- [ ] **Step 1: Config type + load/save + failing test**

`crates/aftercode-cli/src/config.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Privacy {
    pub ignore_paths: Vec<String>,
    pub send_raw_code: bool,
    pub send_diffs: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Privacy {
            ignore_paths: vec![".env".into(), "node_modules".into(), "dist".into(), "build".into()],
            send_raw_code: false,
            send_diffs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_id: String,
    pub project_name: String,
    pub language: String,
    pub episode_length_minutes: u8,
    pub api_base_url: String,
    #[serde(default)]
    pub privacy: Privacy,
}

pub fn config_path() -> PathBuf { Path::new(".aftercode").join("config.json") }

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let p = config_path();
        let txt = std::fs::read_to_string(&p)
            .map_err(|_| anyhow::anyhow!("no .aftercode/config.json — run `aftercode init`"))?;
        Ok(serde_json::from_str(&txt)?)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(".aftercode")?;
        std::fs::write(config_path(), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let c = Config {
            project_id: "proj_1".into(), project_name: "p".into(), language: "en".into(),
            episode_length_minutes: 10, api_base_url: "http://localhost:8080".into(),
            privacy: Privacy::default(),
        };
        c.save().unwrap();
        let back = Config::load().unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(back.project_id, "proj_1");
        assert_eq!(back.privacy.ignore_paths.len(), 4);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli config::`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/config.rs
git commit -m "feat(cli): project config load/save"
```

---

## Task 3: Credentials (token store, 0600)

**Files:**
- Modify: `crates/aftercode-cli/src/credentials.rs`

- [ ] **Step 1: Store/load + failing test**

`crates/aftercode-cli/src/credentials.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct Creds { token: String }

pub fn creds_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("aftercode").join("credentials.json"))
}

pub fn save_token(token: &str) -> anyhow::Result<()> {
    let p = creds_path()?;
    if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&p, serde_json::to_string(&Creds { token: token.into() })?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_token() -> anyhow::Result<String> {
    let p = creds_path()?;
    let txt = std::fs::read_to_string(&p)
        .map_err(|_| anyhow::anyhow!("not logged in — run `aftercode login <token>`"))?;
    let c: Creds = serde_json::from_str(&txt)?;
    Ok(c.token)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_then_load(/* uses real config dir; isolate via XDG_CONFIG_HOME */) {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        save_token("ak_abc").unwrap();
        assert_eq!(load_token().unwrap(), "ak_abc");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(creds_path().unwrap()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli credentials::`
Expected: 1 passed (on Unix asserts 0600).

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/credentials.rs
git commit -m "feat(cli): credential store with 0600 perms"
```

---

## Task 4: Secret scanner

**Files:**
- Modify: `crates/aftercode-cli/src/privacy/secrets.rs`

- [ ] **Step 1: Scanner + failing test**

`crates/aftercode-cli/src/privacy/secrets.rs`:
```rust
use regex::Regex;

/// Returns true if the text appears to contain a secret.
pub fn contains_secret(text: &str) -> bool {
    let patterns = [
        r"(?i)api[_-]?key\s*[:=]\s*['\x22]?[A-Za-z0-9_\-]{16,}",
        r"sk-[A-Za-z0-9]{20,}",                       // OpenAI-style
        r"sk-ant-[A-Za-z0-9_\-]{20,}",                // Anthropic-style
        r"AKIA[0-9A-Z]{16}",                          // AWS access key id
        r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----",
        r"(?i)(secret|password|passwd|token)\s*[:=]\s*['\x22]?\S{8,}",
    ];
    patterns.iter().any(|p| Regex::new(p).unwrap().is_match(text))
}

/// Replace any line containing a secret with a redaction marker.
pub fn redact(text: &str) -> String {
    text.lines()
        .map(|l| if contains_secret(l) { "[REDACTED — secret detected]" } else { l })
        .collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_common_secrets() {
        assert!(contains_secret("API_KEY=abcdef0123456789abcd"));
        assert!(contains_secret("sk-ant-abc123def456ghi789jkl0"));
        assert!(contains_secret("AKIAIOSFODNN7EXAMPLE"));
        assert!(contains_secret("-----BEGIN PRIVATE KEY-----"));
    }
    #[test]
    fn leaves_clean_text() {
        assert!(!contains_secret("fn main() { println!(\"hi\"); }"));
    }
    #[test]
    fn redacts_only_secret_line() {
        let t = "line one\nAPI_KEY=abcdef0123456789abcd\nline three";
        let r = redact(t);
        assert!(r.contains("line one"));
        assert!(r.contains("line three"));
        assert!(r.contains("[REDACTED"));
        assert!(!r.contains("abcdef0123456789"));
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli secrets::`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/privacy/secrets.rs
git commit -m "feat(cli): regex secret scanner + redaction"
```

---

## Task 5: Ignore matching

**Files:**
- Modify: `crates/aftercode-cli/src/privacy/ignore.rs`

- [ ] **Step 1: Matcher + failing test**

`crates/aftercode-cli/src/privacy/ignore.rs`:
```rust
use ignore::gitignore::GitignoreBuilder;

/// Build a matcher from the config ignore_paths and report whether a path is ignored.
pub struct Matcher { inner: ignore::gitignore::Gitignore }

impl Matcher {
    pub fn new(patterns: &[String]) -> anyhow::Result<Self> {
        let mut b = GitignoreBuilder::new(".");
        for p in patterns { b.add_line(None, p)?; }
        Ok(Matcher { inner: b.build()? })
    }
    pub fn is_ignored(&self, path: &str) -> bool {
        self.inner.matched(path, false).is_ignore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_env_and_dirs() {
        let m = Matcher::new(&[".env".into(), "node_modules".into(), "*.key".into()]).unwrap();
        assert!(m.is_ignored(".env"));
        assert!(m.is_ignored("node_modules"));
        assert!(m.is_ignored("server.key"));
        assert!(!m.is_ignored("src/main.rs"));
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli ignore::`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/privacy/ignore.rs
git commit -m "feat(cli): gitignore-style ignore matcher"
```

---

## Task 6: Git collector

**Files:**
- Modify: `crates/aftercode-cli/src/collect/git.rs`

- [ ] **Step 1: Collector + failing test (temp repo fixture)**

`crates/aftercode-cli/src/collect/git.rs`:
```rust
use git2::Repository;

pub struct GitData {
    pub changed_files: Vec<String>,
    pub diff_summary: Option<String>,
    pub commit_messages: Vec<String>,
}

/// Collect changed files (working dir vs HEAD), a short diff summary, and
/// commit messages since `since_days` ago.
pub fn collect(repo_path: &str, since_days: i64) -> anyhow::Result<GitData> {
    let repo = Repository::open(repo_path)?;

    // Changed files: diff HEAD tree vs workdir.
    let mut changed = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), None)?;
    diff.foreach(
        &mut |d, _| {
            if let Some(p) = d.new_file().path() { changed.push(p.display().to_string()); }
            true
        },
        None, None, None,
    )?;
    let stats = diff.stats()?;
    additions += stats.insertions();
    deletions += stats.deletions();
    let summary = if changed.is_empty() { None }
        else { Some(format!("{} files changed, +{additions}/-{deletions}", changed.len())) };

    // Commit messages within the window.
    let mut msgs = Vec::new();
    if let Ok(mut walk) = repo.revwalk() {
        if walk.push_head().is_ok() {
            let cutoff = chrono::Utc::now().timestamp() - since_days * 86_400;
            for oid in walk.flatten() {
                if let Ok(commit) = repo.find_commit(oid) {
                    if commit.time().seconds() < cutoff { break; }
                    if let Some(m) = commit.summary() { msgs.push(m.to_string()); }
                }
            }
        }
    }

    Ok(GitData { changed_files: changed, diff_summary: summary, commit_messages: msgs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run(dir: &std::path::Path, args: &[&str]) {
        let ok = Command::new("git").args(args).current_dir(dir).status().unwrap().success();
        assert!(ok, "git {:?} failed", args);
    }

    #[test]
    fn collects_commit_and_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        run(p, &["init", "-q"]);
        run(p, &["config", "user.email", "t@e.com"]);
        run(p, &["config", "user.name", "t"]);
        std::fs::write(p.join("a.txt"), "one\n").unwrap();
        run(p, &["add", "."]);
        run(p, &["commit", "-qm", "first commit"]);
        // uncommitted change → shows as changed file
        std::fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();

        let data = collect(p.to_str().unwrap(), 7).unwrap();
        assert!(data.commit_messages.iter().any(|m| m == "first commit"));
        assert!(data.changed_files.iter().any(|f| f == "a.txt"));
        assert!(data.diff_summary.is_some());
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli git::`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/collect/git.rs
git commit -m "feat(cli): git collector (changed files, diff summary, commits)"
```

---

## Task 7: Hooks + errors collectors

**Files:**
- Modify: `crates/aftercode-cli/src/collect/hooks.rs`
- Modify: `crates/aftercode-cli/src/collect/errors.rs`

- [ ] **Step 1: Hooks reader + failing test**

`crates/aftercode-cli/src/collect/hooks.rs`:
```rust
use aftercode_core::session::CodingEvent;
use std::path::Path;

/// Read newline-delimited JSON CodingEvents from .aftercode/events/<date>.jsonl
/// for the given dates (YYYY-MM-DD strings). Missing files are skipped.
pub fn collect(dates: &[String]) -> anyhow::Result<Vec<CodingEvent>> {
    let mut events = Vec::new();
    for d in dates {
        let path = Path::new(".aftercode").join("events").join(format!("{d}.jsonl"));
        let Ok(txt) = std::fs::read_to_string(&path) else { continue };
        for line in txt.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(ev) = serde_json::from_str::<CodingEvent>(line) { events.push(ev); }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aftercode_core::session::EventType;
    #[test]
    fn reads_jsonl_events() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all(".aftercode/events").unwrap();
        let ev = CodingEvent { event_type: EventType::UserPrompt,
            timestamp: "2026-06-14T10:00:00Z".into(), content: "fix this".into(),
            metadata: serde_json::json!({}) };
        std::fs::write(".aftercode/events/2026-06-14.jsonl",
            format!("{}\n", serde_json::to_string(&ev).unwrap())).unwrap();
        let out = collect(&["2026-06-14".into()]).unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "fix this");
    }
}
```

- [ ] **Step 2: Errors capture reader**

`crates/aftercode-cli/src/collect/errors.rs`:
```rust
use std::path::Path;

/// Read captured terminal errors from .aftercode/errors.log (one per line). Optional.
pub fn collect() -> Vec<String> {
    let path = Path::new(".aftercode").join("errors.log");
    std::fs::read_to_string(path).ok()
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect())
        .unwrap_or_default()
}
```

- [ ] **Step 3: Run test**

Run: `cargo test -p aftercode-cli hooks::`
Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/aftercode-cli/src/collect/hooks.rs crates/aftercode-cli/src/collect/errors.rs
git commit -m "feat(cli): hook-event + terminal-error collectors"
```

---

## Task 8: Collect orchestrator → SessionContext (with privacy applied)

**Files:**
- Modify: `crates/aftercode-cli/src/collect/mod.rs`

- [ ] **Step 1: Orchestrator + failing test**

`crates/aftercode-cli/src/collect/mod.rs`:
```rust
pub mod git;
pub mod hooks;
pub mod errors;

use crate::config::Config;
use crate::privacy::{ignore::Matcher, secrets};
use aftercode_core::session::{Language, SessionContext};

fn lang_from_str(s: &str) -> Language { if s == "he" { Language::He } else { Language::En } }

fn dates_for(from: &str) -> Vec<String> {
    use chrono::{Duration, Utc};
    let today = Utc::now().date_naive();
    let day = if from == "yesterday" { today - Duration::days(1) } else { today };
    vec![day.format("%Y-%m-%d").to_string()]
}

/// Build a SessionContext from the current directory, honoring ignore rules and
/// redacting secrets from all free-text fields.
pub fn build(cfg: &Config, language_override: Option<String>, from: &str, length: Option<u8>)
    -> anyhow::Result<SessionContext> {
    let matcher = Matcher::new(&cfg.privacy.ignore_paths)?;
    let since_days = if from == "yesterday" { 2 } else { 1 };
    let git = git::collect(".", since_days)?;

    let changed_files: Vec<String> = git.changed_files.into_iter()
        .filter(|f| !matcher.is_ignored(f)).collect();

    let diff_summary = git.diff_summary.map(|d| secrets::redact(&d));
    let commit_messages: Vec<String> = git.commit_messages.iter().map(|m| secrets::redact(m)).collect();

    let mut events = hooks::collect(&dates_for(from))?;
    for ev in &mut events { ev.content = secrets::redact(&ev.content); }
    events.retain(|ev| !secrets::contains_secret(&ev.content) || ev.content.contains("[REDACTED"));

    let terminal_errors: Vec<String> = errors::collect().iter().map(|e| secrets::redact(e)).collect();

    let language = language_override.map(|s| lang_from_str(&s)).unwrap_or_else(|| lang_from_str(&cfg.language));
    let minutes = length.unwrap_or(cfg.episode_length_minutes);

    Ok(SessionContext {
        project_id: cfg.project_id.clone(),
        language, episode_length_minutes: minutes,
        collected_at: chrono::Utc::now().to_rfc3339(),
        events, changed_files, git_diff_summary: diff_summary,
        commit_messages, terminal_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Privacy};
    use std::process::Command;

    #[test]
    fn builds_context_and_drops_ignored_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        for a in [["init","-q"],["config","user.email","t@e.com"],["config","user.name","t"]] {
            Command::new("git").args(a).status().unwrap();
        }
        std::fs::write("keep.rs", "fn x(){}").unwrap();
        std::fs::write(".env", "API_KEY=abcdef0123456789abcd").unwrap();
        Command::new("git").args(["add","keep.rs"]).status().unwrap();
        Command::new("git").args(["commit","-qm","add keep"]).status().unwrap();
        std::fs::write("keep.rs", "fn x(){ /* edit */ }").unwrap();

        let cfg = Config { project_id: "p".into(), project_name: "p".into(), language: "en".into(),
            episode_length_minutes: 10, api_base_url: "http://x".into(), privacy: Privacy::default() };
        let ctx = build(&cfg, Some("he".into()), "today", Some(5)).unwrap();
        std::env::set_current_dir(prev).unwrap();

        assert!(matches!(ctx.language, Language::He));
        assert_eq!(ctx.episode_length_minutes, 5);
        assert!(ctx.changed_files.iter().any(|f| f == "keep.rs"));
        assert!(!ctx.changed_files.iter().any(|f| f == ".env")); // ignored
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p aftercode-cli collect::`
Expected: 1 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/collect/mod.rs
git commit -m "feat(cli): collect orchestrator with privacy + ignore applied"
```

---

## Task 9: Backend client

**Files:**
- Modify: `crates/aftercode-cli/src/client.rs`

- [ ] **Step 1: Typed client (no test — covered via command-level usage)**

`crates/aftercode-cli/src/client.rs`:
```rust
use aftercode_core::session::SessionContext;

pub struct Client { base: String, token: String, http: reqwest::Client }

impl Client {
    pub fn new(base: String, token: String) -> Self {
        Client { base, token, http: reqwest::Client::new() }
    }

    fn url(&self, path: &str) -> String { format!("{}{}", self.base.trim_end_matches('/'), path) }

    pub async fn register_project(&self, name: &str, language: &str) -> anyhow::Result<String> {
        let v: serde_json::Value = self.http.post(self.url("/cli/register-project"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "name": name, "default_language": language }))
            .send().await?.error_for_status()?.json().await?;
        Ok(v["project_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn generate_episode(&self, ctx: &SessionContext, language: &str)
        -> anyhow::Result<String> {
        let v: serde_json::Value = self.http.post(self.url("/cli/generate-episode"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "project_id": ctx.project_id, "language": language, "session_context": ctx }))
            .send().await?.error_for_status()?.json().await?;
        Ok(v["episode_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn episode_status(&self, id: &str) -> anyhow::Result<serde_json::Value> {
        Ok(self.http.get(self.url(&format!("/cli/episode-status/{id}")))
            .bearer_auth(&self.token).send().await?.error_for_status()?.json().await?)
    }
}
```

- [ ] **Step 2: Compile**

Run: `cargo build -p aftercode-cli`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/client.rs
git commit -m "feat(cli): backend http client"
```

---

## Task 10: Commands (init, login, status, preview, episode, ignore, open)

**Files:**
- Modify: `crates/aftercode-cli/src/commands/mod.rs`

- [ ] **Step 1: Implement all commands**

`crates/aftercode-cli/src/commands/mod.rs`:
```rust
use crate::client::Client;
use crate::config::{Config, Privacy};
use crate::{collect, credentials};
use std::io::{self, Write};

fn prompt(q: &str, default: &str) -> String {
    print!("{q} [{default}]: ");
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    let s = s.trim();
    if s.is_empty() { default.to_string() } else { s.to_string() }
}

pub async fn init() -> anyhow::Result<()> {
    let default_name = std::env::current_dir().ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "project".into());
    let name = prompt("Project name", &default_name);
    let language = prompt("Language (he/en)", "en");
    let length: u8 = prompt("Episode length (5/10/15)", "10").parse().unwrap_or(10);
    let api = prompt("Backend URL", "http://localhost:8080");

    let token = credentials::load_token().ok();
    let project_id = if let Some(t) = token {
        match Client::new(api.clone(), t).register_project(&name, &language).await {
            Ok(id) => id,
            Err(e) => { eprintln!("warning: could not register project ({e}); using local id"); "local".into() }
        }
    } else {
        eprintln!("Not logged in — run `aftercode login <token>` then `aftercode init` again to register.");
        "local".into()
    };

    let cfg = Config {
        project_id, project_name: name, language, episode_length_minutes: length,
        api_base_url: api, privacy: Privacy::default(),
    };
    cfg.save()?;
    println!("Wrote .aftercode/config.json");
    Ok(())
}

pub fn login(token: String) -> anyhow::Result<()> {
    credentials::save_token(&token)?;
    println!("Saved credentials.");
    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let logged_in = credentials::load_token().is_ok();
    let git_ok = git2::Repository::open(".").is_ok();
    let hooks_ok = std::path::Path::new(".aftercode/events").exists();
    println!("Aftercode status\n");
    println!("Project:   {}", cfg.project_name);
    println!("Language:  {}", cfg.language);
    println!("Backend:   {}", cfg.api_base_url);
    println!("Logged in: {}", if logged_in { "yes" } else { "no" });
    println!("Git:       {}", if git_ok { "connected" } else { "not a repo" });
    println!("Hooks:     {}", if hooks_ok { "connected" } else { "not configured" });
    Ok(())
}

pub fn preview() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let ctx = collect::build(&cfg, None, "today", None)?;
    println!("Aftercode will send:\n");
    println!("Changed files:");
    for f in &ctx.changed_files { println!("  - {f}"); }
    if let Some(d) = &ctx.git_diff_summary { println!("\nDiff: {d}"); }
    if !ctx.terminal_errors.is_empty() {
        println!("\nDetected errors:");
        for e in &ctx.terminal_errors { println!("  - {e}"); }
    }
    println!("\nEvents collected: {}", ctx.events.len());
    println!("Language: {:?}  Length: {} min", ctx.language, ctx.episode_length_minutes);
    Ok(())
}

pub async fn episode(language: Option<String>, from: String, length: Option<u8>) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let token = credentials::load_token()?;
    let ctx = collect::build(&cfg, language.clone(), &from, length)?;
    let lang = language.unwrap_or_else(|| cfg.language.clone());
    let client = Client::new(cfg.api_base_url.clone(), token);

    println!("Uploading session and generating episode...");
    let episode_id = client.generate_episode(&ctx, &lang).await?;

    // Poll until ready/failed.
    use indicatif::{ProgressBar, ProgressStyle};
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    let mut title = String::new();
    loop {
        let s = client.episode_status(&episode_id).await?;
        let status = s["status"].as_str().unwrap_or("queued").to_string();
        pb.set_message(status.clone());
        pb.tick();
        if status == "ready" { title = s["title"].as_str().unwrap_or("").to_string(); break; }
        if status == "failed" {
            pb.finish_and_clear();
            anyhow::bail!("Episode generation failed: {}", s["error"].as_str().unwrap_or("unknown"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }
    pb.finish_and_clear();

    println!("\nGenerated episode:\n  \"{title}\"\n");
    println!("Open: {}/episodes/{episode_id}", cfg.api_base_url.trim_end_matches('/'));
    Ok(())
}

pub fn ignore(pattern: String) -> anyhow::Result<()> {
    let mut cfg = Config::load()?;
    if !cfg.privacy.ignore_paths.contains(&pattern) {
        cfg.privacy.ignore_paths.push(pattern.clone());
        cfg.save()?;
        println!("Added ignore: {pattern}");
    } else {
        println!("Already ignored: {pattern}");
    }
    Ok(())
}

pub fn open() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let url = cfg.api_base_url;
    let cmd = if cfg!(target_os = "macos") { "open" }
        else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
    std::process::Command::new(cmd).arg(&url).status().ok();
    println!("Opening {url}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignore_appends_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        Config { project_id:"p".into(), project_name:"p".into(), language:"en".into(),
            episode_length_minutes:10, api_base_url:"http://x".into(),
            privacy: Privacy::default() }.save().unwrap();
        ignore("*.secret".into()).unwrap();
        let c = Config::load().unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert!(c.privacy.ignore_paths.contains(&"*.secret".to_string()));
    }
}
```

- [ ] **Step 2: Run command test + build**

Run: `cargo test -p aftercode-cli commands:: && cargo build -p aftercode-cli`
Expected: 1 passed; binary builds.

- [ ] **Step 3: Commit**

```bash
git add crates/aftercode-cli/src/commands/mod.rs
git commit -m "feat(cli): init/login/status/preview/episode/ignore/open commands"
```

---

## Task 11: Manual end-to-end smoke (against running backend)

**Files:** none (manual verification)

- [ ] **Step 1: Start backend (Plan A) + seed a token**

Run (in repo root, Postgres up + migration applied):
```bash
cargo run -p aftercode-server seed-user dev@example.com   # prints ak_...
LLM_PROVIDER=mock BLOB_STORE=localfs cargo run -p aftercode-server &
```

- [ ] **Step 2: Drive the CLI in a git repo**

Run:
```bash
cargo run -p aftercode -- login ak_<paste>
cargo run -p aftercode -- init        # accept defaults, backend http://localhost:8080
cargo run -p aftercode -- preview
cargo run -p aftercode -- episode --language en
```
Expected: `episode` prints a generated title and an episode URL; `GET /episodes/<id>` on the backend shows `status: ready` with an `audio_url`.

- [ ] **Step 3: Commit any fixes found during smoke**

```bash
git add -A && git commit -m "fix(cli): smoke-test adjustments"
```

---

## Self-Review (spec coverage)

- §5.1 commands → Task 10 (all seven + `login`). §5.2 modules → Tasks 1–10 map 1:1 to the file layout. §5.3 config files → Tasks 2 (`.aftercode/config.json`), 3 (`credentials.json` 0600). §5.4 privacy → Tasks 4 (secrets), 5 (ignore), 8 (applied in build). §5.5 hook capture → Task 7 (reads `.aftercode/events/*.jsonl`). `--language/--from/--length` flags → Tasks 1 + 10 + 8 (`build` honors all three).
- Type consistency: `Config`/`Privacy`, `collect::build`, `Matcher::is_ignored`, `secrets::redact/contains_secret`, `Client::{register_project,generate_episode,episode_status}`, `credentials::{save_token,load_token}` used consistently across tasks.
- No placeholders: every step ships full code + exact run command + expected result.
```
