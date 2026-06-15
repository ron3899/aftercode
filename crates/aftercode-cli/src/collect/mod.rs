pub mod errors;
pub mod git;
pub mod hooks;

use crate::config::Config;
use crate::privacy::{ignore::Matcher, secrets};
use crate::session;
use aftercode_core::session::{CodingEvent, EventType, Language, SessionContext};
use serde_json::Value;

/// Caps to bound payload size/cost.
const PER_EVENT_CHARS: usize = 8_000;
const TOTAL_CHARS: usize = 150_000;

fn lang_from_str(s: &str) -> Language {
    if s == "he" {
        Language::He
    } else {
        Language::En
    }
}

fn dates_for(from: &str) -> Vec<String> {
    use chrono::{Duration, Utc};
    let today = Utc::now().date_naive();
    let day = if from == "yesterday" {
        today - Duration::days(1)
    } else {
        today
    };
    vec![day.format("%Y-%m-%d").to_string()]
}

/// Which agent session was detected for this repo (for preview/status). None if
/// no agent session found. `forced` corresponds to `--agent`.
pub fn detected_agent(forced: Option<&str>) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    session::detect_best(&cwd, forced).map(|s| s.agent)
}

/// Parse a transcript handed in via `--transcript` into coding events. Forgiving,
/// in priority order: (1) Claude Code JSONL, (2) simple `{"role","text"}` JSONL,
/// (3) plain text chunked into ≤PER_EVENT_CHARS events so nothing is lost to the
/// per-event cap. Public for testing.
pub fn parse_transcript_input(text: &str) -> Vec<CodingEvent> {
    // 1. Claude Code JSONL transcript (lets you pipe ~/.claude/projects/**/*.jsonl).
    let cc = crate::session::claude_code::parse_transcript(text);
    if !cc.is_empty() {
        return cc;
    }
    // 2. Simple JSONL: one {"role":"user"|"assistant","text":"..."} per line.
    let mut simple: Vec<CodingEvent> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(t) = v.get("text").and_then(|x| x.as_str()) else {
            continue;
        };
        if t.trim().is_empty() {
            continue;
        }
        let role = v
            .get("role")
            .and_then(|x| x.as_str())
            .unwrap_or("assistant");
        let event_type = if role == "user" {
            EventType::UserPrompt
        } else {
            EventType::AgentResponse
        };
        simple.push(CodingEvent {
            event_type,
            timestamp: String::new(),
            content: t.to_string(),
            metadata: Value::Null,
        });
    }
    if !simple.is_empty() {
        return simple;
    }
    // 3. Plain text: chunk into ≤PER_EVENT_CHARS pieces (by char count).
    chunk_plaintext(text)
}

fn chunk_plaintext(text: &str) -> Vec<CodingEvent> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    chars
        .chunks(PER_EVENT_CHARS)
        .map(|c| CodingEvent {
            event_type: EventType::AgentResponse,
            timestamp: String::new(),
            content: c.iter().collect(),
            metadata: Value::Null,
        })
        .collect()
}

/// Build a SessionContext from the current directory: the agent session
/// transcript (auto-detected) + the real git diff, with ignore rules applied,
/// secrets redacted, and size capped.
pub fn build(
    cfg: &Config,
    language_override: Option<String>,
    from: &str,
    length: Option<u8>,
    agent: Option<String>,
    transcript: Option<String>,
) -> anyhow::Result<SessionContext> {
    let matcher = Matcher::new(&cfg.privacy.ignore_paths)?;
    let since_days = if from == "yesterday" { 2 } else { 1 };
    let git = git::collect(".", since_days)?;
    let cwd = std::env::current_dir()?;

    let mut events: Vec<CodingEvent> = Vec::new();

    // 1. Session transcript (the richest signal). An explicitly handed-in
    //    transcript (`--transcript`) is authoritative — the agent that invoked
    //    the CLI knows its own history better than any disk scrape — so when it's
    //    present we use it and skip on-disk auto-detection.
    if let Some(t) = &transcript {
        events.extend(parse_transcript_input(t));
    } else if let Some(sess) = session::detect_best(&cwd, agent.as_deref()) {
        events.extend(sess.events);
    }

    // 2. Hook events (back-compat: .aftercode/events/*.jsonl).
    events.extend(hooks::collect(&dates_for(from)).unwrap_or_default());

    // 3. Real git diff hunks as GitDiff events (skip ignored files).
    if cfg.privacy.send_diffs {
        for (path, patch) in &git.diff_hunks {
            if matcher.is_ignored(path) {
                continue;
            }
            events.push(CodingEvent {
                event_type: EventType::GitDiff,
                timestamp: String::new(),
                content: format!("{path}\n{patch}"),
                metadata: Value::Null,
            });
        }
    }

    // Redact secrets from every event, then enforce caps.
    for ev in &mut events {
        ev.content = secrets::redact(&ev.content);
    }
    let events = apply_caps(events);

    let changed_files: Vec<String> = git
        .changed_files
        .into_iter()
        .filter(|f| !matcher.is_ignored(f))
        .collect();
    let diff_summary = git.diff_summary.map(|d| secrets::redact(&d));
    let commit_messages: Vec<String> = git
        .commit_messages
        .iter()
        .map(|m| secrets::redact(m))
        .collect();
    let terminal_errors: Vec<String> = errors::collect()
        .iter()
        .map(|e| secrets::redact(e))
        .collect();

    let language = language_override
        .map(|s| lang_from_str(&s))
        .unwrap_or_else(|| lang_from_str(&cfg.language));
    let minutes = length.unwrap_or(cfg.episode_length_minutes);

    Ok(SessionContext {
        project_id: cfg.project_id.clone(),
        language,
        episode_length_minutes: minutes,
        collected_at: chrono::Utc::now().to_rfc3339(),
        events,
        changed_files,
        git_diff_summary: diff_summary,
        commit_messages,
        terminal_errors,
    })
}

/// Truncate each event to PER_EVENT_CHARS, then keep the most-recent events
/// within TOTAL_CHARS. Prepends a marker if anything was dropped.
fn apply_caps(mut events: Vec<CodingEvent>) -> Vec<CodingEvent> {
    for ev in &mut events {
        if ev.content.chars().count() > PER_EVENT_CHARS {
            let kept: String = ev.content.chars().take(PER_EVENT_CHARS).collect();
            ev.content = format!("{kept}\n[…truncated]");
        }
    }
    let mut total = 0usize;
    let mut kept_rev: Vec<CodingEvent> = Vec::new();
    let original = events.len();
    for ev in events.into_iter().rev() {
        let n = ev.content.chars().count();
        if total + n > TOTAL_CHARS && !kept_rev.is_empty() {
            break;
        }
        total += n;
        kept_rev.push(ev);
    }
    kept_rev.reverse();
    let dropped = original - kept_rev.len();
    if dropped > 0 {
        kept_rev.insert(
            0,
            CodingEvent {
                event_type: EventType::AgentResponse,
                timestamp: String::new(),
                content: format!("[…{dropped} earlier events truncated for size]"),
                metadata: Value::Null,
            },
        );
    }
    kept_rev
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Privacy};
    use std::process::Command;

    fn base_cfg() -> Config {
        Config {
            project_id: "p".into(),
            project_name: "p".into(),
            language: "en".into(),
            episode_length_minutes: 10,
            api_base_url: "http://x".into(),
            privacy: Privacy::default(),
        }
    }

    #[test]
    #[serial_test::serial(fs)]
    fn builds_context_with_diff_and_drops_ignored_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        for a in [
            vec!["init", "-q"],
            vec!["config", "user.email", "t@e.com"],
            vec!["config", "user.name", "t"],
        ] {
            Command::new("git").args(&a).status().unwrap();
        }
        std::fs::write("keep.rs", "fn x(){}").unwrap();
        std::fs::write(".env", "SECRET=xyz").unwrap();
        Command::new("git")
            .args(["add", "keep.rs"])
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-qm", "add keep"])
            .status()
            .unwrap();
        // change keep.rs (adds a line with a secret) + the ignored .env
        std::fs::write(
            "keep.rs",
            "fn x(){}\nlet api_key = \"abcdef0123456789abcd\";\n",
        )
        .unwrap();
        std::fs::write(".env", "SECRET=changed0123456789").unwrap();

        let ctx = build(&base_cfg(), Some("he".into()), "today", Some(5), None, None).unwrap();
        std::env::set_current_dir(prev).unwrap();

        assert!(matches!(ctx.language, Language::He));
        assert_eq!(ctx.episode_length_minutes, 5);
        // diff event for keep.rs present
        let diff_ev: Vec<_> = ctx
            .events
            .iter()
            .filter(|e| e.event_type == EventType::GitDiff)
            .collect();
        assert!(diff_ev.iter().any(|e| e.content.starts_with("keep.rs")));
        // .env is ignored -> no diff event for it
        assert!(!diff_ev.iter().any(|e| e.content.starts_with(".env")));
        // secret line in the diff is redacted
        assert!(ctx
            .events
            .iter()
            .all(|e| !e.content.contains("abcdef0123456789")));
    }

    #[test]
    fn transcript_parses_simple_role_text_jsonl() {
        let input = "{\"role\":\"user\",\"text\":\"how do I measure churn?\"}\n{\"role\":\"assistant\",\"text\":\"split voluntary from involuntary\"}\n";
        let evs = parse_transcript_input(input);
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].event_type, EventType::UserPrompt);
        assert!(evs[0].content.contains("measure churn"));
        assert_eq!(evs[1].event_type, EventType::AgentResponse);
    }

    #[test]
    fn transcript_parses_claude_code_jsonl() {
        // A Claude Code transcript line (type=user, message.content is a string).
        let input = "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"build me a dashboard\"},\"timestamp\":\"2026-06-15T00:00:00Z\"}\n";
        let evs = parse_transcript_input(input);
        assert!(!evs.is_empty());
        assert!(evs.iter().any(|e| e.content.contains("dashboard")));
    }

    #[test]
    fn transcript_plaintext_is_chunked_not_dropped() {
        let big = "y".repeat(PER_EVENT_CHARS * 2 + 10);
        let evs = parse_transcript_input(&big);
        assert_eq!(evs.len(), 3); // 2 full chunks + remainder
        assert!(evs
            .iter()
            .all(|e| e.content.chars().count() <= PER_EVENT_CHARS));
        assert!(evs.iter().all(|e| e.event_type == EventType::AgentResponse));
    }

    #[test]
    fn transcript_empty_yields_nothing() {
        assert!(parse_transcript_input("   \n  ").is_empty());
    }

    #[test]
    fn caps_truncate_and_mark() {
        let big = "x".repeat(PER_EVENT_CHARS + 500);
        let mut events: Vec<CodingEvent> = (0..40)
            .map(|i| CodingEvent {
                event_type: EventType::AgentResponse,
                timestamp: String::new(),
                content: format!("{i}-{big}"),
                metadata: Value::Null,
            })
            .collect();
        events.push(CodingEvent {
            event_type: EventType::GitDiff,
            timestamp: String::new(),
            content: "newest".into(),
            metadata: Value::Null,
        });
        let out = apply_caps(events);
        // per-event cap applied
        assert!(out
            .iter()
            .all(|e| e.content.chars().count() <= PER_EVENT_CHARS + 32));
        // total cap dropped some -> marker present
        assert!(out[0].content.contains("truncated for size"));
        // newest kept
        assert!(out.iter().any(|e| e.content == "newest"));
    }
}
