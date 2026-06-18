use crate::client::Client;
use crate::config::Config;
use crate::{collect, credentials};
use std::io::{self, Write};

fn prompt(q: &str, default: &str) -> String {
    print!("{q} [{default}]: ");
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    let s = s.trim();
    if s.is_empty() {
        default.to_string()
    } else {
        s.to_string()
    }
}

pub async fn init(
    yes: bool,
    name_arg: Option<String>,
    language_arg: Option<String>,
    length_arg: Option<u8>,
    backend_arg: Option<String>,
) -> anyhow::Result<()> {
    // Re-running init must NOT silently reset a working config (backend URL,
    // project id). Load any existing config and use its values as the defaults.
    let existing = Config::load().ok();
    let default_name = existing
        .as_ref()
        .map(|c| c.project_name.clone())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        })
        .unwrap_or_else(|| "project".into());
    let default_lang = existing
        .as_ref()
        .map(|c| c.language.clone())
        .unwrap_or_else(|| "en".into());
    let default_len = existing
        .as_ref()
        .map(|c| c.episode_length_minutes)
        .unwrap_or(10);
    let default_api = existing
        .as_ref()
        .map(|c| c.api_base_url.clone())
        .unwrap_or_else(|| "http://localhost:8080".into());

    // With a flag set, use it. With --yes (and no flag), take the default
    // silently. Otherwise prompt interactively.
    let name = name_arg.unwrap_or_else(|| {
        if yes {
            default_name
        } else {
            prompt("Project name", &default_name)
        }
    });
    let language = language_arg.unwrap_or_else(|| {
        if yes {
            default_lang
        } else {
            prompt("Language (he/en)", &default_lang)
        }
    });
    let length: u8 = length_arg.unwrap_or_else(|| {
        if yes {
            default_len
        } else {
            prompt("Episode length (5/10/15)", &default_len.to_string())
                .parse()
                .unwrap_or(default_len)
        }
    });
    let api = backend_arg.unwrap_or_else(|| {
        if yes {
            default_api
        } else {
            prompt("Backend URL", &default_api)
        }
    });

    let token = credentials::load_token().ok();
    let prior_project = existing
        .as_ref()
        .map(|c| c.project_id.clone())
        .filter(|p| p != "local");
    let project_id = if let Some(t) = token {
        match Client::new(api.clone(), t)
            .register_project(&name, &language)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                // Keep the previously-registered project rather than dropping to "local".
                let fallback = prior_project.clone().unwrap_or_else(|| "local".into());
                eprintln!(
                    "warning: could not register project ({e}); keeping project_id={fallback}"
                );
                fallback
            }
        }
    } else {
        eprintln!(
            "Not logged in — run `aftercode login <token>` then `aftercode init` again to register."
        );
        prior_project.unwrap_or_else(|| "local".into())
    };

    let cfg = Config {
        project_id,
        project_name: name,
        language,
        episode_length_minutes: length,
        api_base_url: api,
        // preserve existing privacy/ignore settings on re-init
        privacy: existing.map(|c| c.privacy).unwrap_or_default(),
    };
    cfg.save()?;
    println!("Wrote .aftercode/config.json");
    Ok(())
}

pub fn login(token: Option<String>, backend: Option<String>) -> anyhow::Result<()> {
    match token {
        Some(t) => {
            credentials::save_token(&t)?;
            println!("Saved credentials.");
            Ok(())
        }
        None => crate::auth_login::browser_login(backend),
    }
}

pub async fn status() -> anyhow::Result<()> {
    // Don't hard-fail without a project config — `status` should still report
    // login state (e.g. right after `aftercode login`, before `init`).
    let cfg = Config::load().ok();
    let git_ok = git2::Repository::open(".").is_ok();
    let hooks_ok = std::path::Path::new(".aftercode/events").exists();

    // Backend from project config, else the default.
    let backend = cfg
        .as_ref()
        .map(|c| c.api_base_url.clone())
        .unwrap_or_else(|| "http://localhost:8080".into());

    // Validate the token against the backend — a local token may be stale/invalid.
    let auth = match credentials::load_token() {
        Err(_) => "no — run `aftercode login`".to_string(),
        Ok(tok) => {
            if Client::new(backend.clone(), tok).token_valid().await {
                "yes (token valid)".to_string()
            } else {
                "token present but INVALID/expired — run `aftercode login`".to_string()
            }
        }
    };

    println!("Aftercode status\n");
    match &cfg {
        Some(c) => {
            println!("Project:   {}", c.project_name);
            println!("Language:  {}", c.language);
        }
        None => println!("Project:   none here — run `aftercode init` in your project"),
    }
    println!("Backend:   {backend}");
    println!("Logged in: {auth}");
    println!(
        "Git:       {}",
        if git_ok { "connected" } else { "not a repo" }
    );
    println!(
        "Hooks:     {}",
        if hooks_ok {
            "connected"
        } else {
            "not configured"
        }
    );
    let agent = collect::detected_agent(None);
    println!(
        "Agent:     {}",
        agent.unwrap_or_else(|| "none detected (will use git diff only)".into())
    );
    Ok(())
}

pub fn preview() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let agent = collect::detected_agent(None);
    let ctx = collect::build(&cfg, None, "today", None, None, None)?;
    println!("Aftercode will send:\n");
    println!(
        "Agent session: {}",
        agent
            .clone()
            .unwrap_or_else(|| "none (git diff only)".into())
    );
    println!("\nChanged files:");
    for f in &ctx.changed_files {
        println!("  - {f}");
    }
    if let Some(d) = &ctx.git_diff_summary {
        println!("\nDiff: {d}");
    }
    if !ctx.terminal_errors.is_empty() {
        println!("\nDetected errors:");
        for e in &ctx.terminal_errors {
            println!("  - {e}");
        }
    }
    // event breakdown by type
    use aftercode_core::session::EventType;
    let count = |t: EventType| ctx.events.iter().filter(|e| e.event_type == t).count();
    println!("\nEvents collected: {} total", ctx.events.len());
    println!("  prompts:        {}", count(EventType::UserPrompt));
    println!("  agent messages: {}", count(EventType::AgentResponse));
    println!("  file changes:   {}", count(EventType::FileChanged));
    println!("  diff hunks:     {}", count(EventType::GitDiff));
    if count(EventType::UserPrompt) == 0 && count(EventType::AgentResponse) == 0 {
        println!(
            "\n⚠  No agent session captured — an episode now would be built from the git diff\n   only (likely thin). Pipe your conversation with `--transcript -`, run from the\n   workspace root your agent has open, or pass `--allow-thin` to force it."
        );
    }
    println!(
        "\nLanguage: {:?}  Length: {} min",
        ctx.language, ctx.episode_length_minutes
    );
    Ok(())
}

pub async fn episode(
    language: Option<String>,
    from: String,
    length: Option<u8>,
    agent: Option<String>,
    transcript: Option<String>,
    allow_thin: bool,
) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let token = credentials::load_token()?;

    // Resolve an explicit session handoff: `--transcript -` (stdin) or a file path.
    let transcript_text: Option<String> = match transcript.as_deref() {
        None => None,
        Some("-") => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Some(buf)
        }
        Some(path) => Some(
            std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("could not read --transcript file {path}: {e}"))?,
        ),
    };

    if transcript_text.is_some() {
        println!("Using the transcript you handed in for this episode.");
    } else if let Some(a) = collect::detected_agent(agent.as_deref()) {
        println!("Using {a} session for this repo.");
    } else {
        println!("No agent session detected — using git diff only.");
    }

    let ctx = collect::build(
        &cfg,
        language.clone(),
        &from,
        length,
        agent,
        transcript_text,
    )?;

    // Guardrail: refuse to ship a "thin" episode built from a lone diff with no
    // conversation behind it (the package-lock.json trap), unless forced.
    use aftercode_core::session::EventType;
    let has_session = ctx.events.iter().any(|e| {
        matches!(
            e.event_type,
            EventType::UserPrompt | EventType::AgentResponse
        )
    });
    if !has_session && !allow_thin {
        anyhow::bail!(
            "No session conversation was captured — the episode would be built from the git \
             diff alone and would be thin.\n\nDo one of:\n  • pipe your conversation:  \
             <your-agent-transcript> | aftercode episode --transcript -\n  • run from the \
             workspace root your agent has open (so the session is detected)\n  • force it:  \
             aftercode episode --allow-thin\n\nRun `aftercode preview` to see what was collected."
        );
    }

    let lang = language.unwrap_or_else(|| cfg.language.clone());
    let client = Client::new(cfg.api_base_url.clone(), token);

    println!("Uploading session and generating episode...");
    let episode_id = client.generate_episode(&ctx, &lang).await?;

    use indicatif::{ProgressBar, ProgressStyle};
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    let title = loop {
        let s = client.episode_status(&episode_id).await?;
        let status = s["status"].as_str().unwrap_or("queued").to_string();
        pb.set_message(status.clone());
        pb.tick();
        if status == "ready" {
            break s["title"].as_str().unwrap_or("").to_string();
        }
        if status == "failed" {
            pb.finish_and_clear();
            anyhow::bail!(
                "Episode generation failed: {}",
                s["error"].as_str().unwrap_or("unknown")
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    };
    pb.finish_and_clear();

    println!("\nGenerated episode:\n  \"{title}\"\n");
    println!(
        "Listen: {}/static/episodes/{episode_id}.mp3",
        cfg.api_base_url.trim_end_matches('/')
    );
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
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(cmd).arg(&url).status().ok();
    println!("Opening {url}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Privacy;
    #[test]
    #[serial_test::serial(fs)]
    fn ignore_appends_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        Config {
            project_id: "p".into(),
            project_name: "p".into(),
            language: "en".into(),
            episode_length_minutes: 10,
            api_base_url: "http://x".into(),
            privacy: Privacy::default(),
        }
        .save()
        .unwrap();
        ignore("*.secret".into()).unwrap();
        let c = Config::load().unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert!(c.privacy.ignore_paths.contains(&"*.secret".to_string()));
    }
}
