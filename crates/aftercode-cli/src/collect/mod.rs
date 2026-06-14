pub mod errors;
pub mod git;
pub mod hooks;

use crate::config::Config;
use crate::privacy::{ignore::Matcher, secrets};
use aftercode_core::session::{Language, SessionContext};

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

/// Build a SessionContext from the current directory, honoring ignore rules and
/// redacting secrets from all free-text fields.
pub fn build(
    cfg: &Config,
    language_override: Option<String>,
    from: &str,
    length: Option<u8>,
) -> anyhow::Result<SessionContext> {
    let matcher = Matcher::new(&cfg.privacy.ignore_paths)?;
    let since_days = if from == "yesterday" { 2 } else { 1 };
    let git = git::collect(".", since_days)?;

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

    let mut events = hooks::collect(&dates_for(from))?;
    for ev in &mut events {
        ev.content = secrets::redact(&ev.content);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Privacy};
    use std::process::Command;

    #[test]
    #[serial_test::serial(fs)]
    fn builds_context_and_drops_ignored_and_secrets() {
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
        std::fs::write(".env", "API_KEY=abcdef0123456789abcd").unwrap();
        Command::new("git")
            .args(["add", "keep.rs"])
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-qm", "add keep"])
            .status()
            .unwrap();
        std::fs::write("keep.rs", "fn x(){ /* edit */ }").unwrap();

        let cfg = Config {
            project_id: "p".into(),
            project_name: "p".into(),
            language: "en".into(),
            episode_length_minutes: 10,
            api_base_url: "http://x".into(),
            privacy: Privacy::default(),
        };
        let ctx = build(&cfg, Some("he".into()), "today", Some(5)).unwrap();
        std::env::set_current_dir(prev).unwrap();

        assert!(matches!(ctx.language, Language::He));
        assert_eq!(ctx.episode_length_minutes, 5);
        assert!(ctx.changed_files.iter().any(|f| f == "keep.rs"));
        assert!(!ctx.changed_files.iter().any(|f| f == ".env")); // ignored
    }
}
