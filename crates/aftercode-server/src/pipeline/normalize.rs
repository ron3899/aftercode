use crate::providers::llm::NormalizedContext;
use aftercode_core::session::SessionContext;

/// Flatten a SessionContext into a single prompt-ready text block.
pub fn normalize(ctx: &SessionContext) -> NormalizedContext {
    let mut t = String::new();
    if !ctx.changed_files.is_empty() {
        t.push_str("Changed files:\n");
        for f in &ctx.changed_files {
            t.push_str(&format!("- {f}\n"));
        }
    }
    if let Some(d) = &ctx.git_diff_summary {
        t.push_str(&format!("\nDiff summary:\n{d}\n"));
    }
    if !ctx.terminal_errors.is_empty() {
        t.push_str("\nTerminal errors:\n");
        for e in &ctx.terminal_errors {
            t.push_str(&format!("- {e}\n"));
        }
    }
    if !ctx.commit_messages.is_empty() {
        t.push_str("\nCommits:\n");
        for c in &ctx.commit_messages {
            t.push_str(&format!("- {c}\n"));
        }
    }
    for ev in &ctx.events {
        t.push_str(&format!("\n[{:?}] {}\n", ev.event_type, ev.content));
    }
    NormalizedContext {
        text: t,
        language: ctx.language,
        minutes: ctx.episode_length_minutes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aftercode_core::session::{Language, SessionContext};
    #[test]
    fn includes_files_and_errors() {
        let ctx = SessionContext {
            project_id: "p".into(),
            language: Language::En,
            episode_length_minutes: 10,
            collected_at: "t".into(),
            events: vec![],
            changed_files: vec!["m.py".into()],
            git_diff_summary: None,
            commit_messages: vec![],
            terminal_errors: vec!["CONCURRENTLY cannot run in a transaction".into()],
        };
        let n = normalize(&ctx);
        assert!(n.text.contains("m.py"));
        assert!(n.text.contains("CONCURRENTLY"));
    }
}
