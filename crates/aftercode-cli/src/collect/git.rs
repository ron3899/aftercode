use git2::{DiffFormat, Repository};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct GitData {
    pub changed_files: Vec<String>,
    pub diff_summary: Option<String>,
    pub commit_messages: Vec<String>,
    /// (path, patch text) per changed, non-binary file.
    pub diff_hunks: Vec<(String, String)>,
}

/// Collect changed files (working dir vs HEAD), a short diff summary, and
/// commit messages within the last `since_days` days.
pub fn collect(repo_path: &str, since_days: i64) -> anyhow::Result<GitData> {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        // Not a git repo: don't hard-fail — proceed with no diff so the episode
        // can still be built from the agent session alone. Other git errors are
        // real and propagate.
        Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(GitData::default()),
        Err(e) => return Err(e.into()),
    };

    // Changed files: diff HEAD tree vs workdir.
    let mut changed = Vec::new();
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), None)?;
    diff.foreach(
        &mut |d, _| {
            if let Some(p) = d.new_file().path() {
                changed.push(p.display().to_string());
            }
            true
        },
        None,
        None,
        None,
    )?;
    let stats = diff.stats()?;
    let additions = stats.insertions();
    let deletions = stats.deletions();
    let summary = if changed.is_empty() {
        None
    } else {
        Some(format!(
            "{} files changed, +{additions}/-{deletions}",
            changed.len()
        ))
    };

    // Per-file diff hunks (skip binary). Keyed by path to group lines.
    let mut hunks: BTreeMap<String, String> = BTreeMap::new();
    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        if delta.new_file().is_binary() || delta.old_file().is_binary() {
            return true;
        }
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let buf = hunks.entry(path).or_default();
        let origin = line.origin();
        if matches!(origin, '+' | '-' | ' ') {
            buf.push(origin);
        }
        buf.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;
    let diff_hunks: Vec<(String, String)> = hunks.into_iter().collect();

    // Commit messages within the window.
    let mut msgs = Vec::new();
    if let Ok(mut walk) = repo.revwalk() {
        if walk.push_head().is_ok() {
            let cutoff = chrono::Utc::now().timestamp() - since_days * 86_400;
            for oid in walk.flatten() {
                if let Ok(commit) = repo.find_commit(oid) {
                    if commit.time().seconds() < cutoff {
                        break;
                    }
                    if let Some(m) = commit.summary() {
                        msgs.push(m.to_string());
                    }
                }
            }
        }
    }

    Ok(GitData {
        changed_files: changed,
        diff_summary: summary,
        commit_messages: msgs,
        diff_hunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run(dir: &std::path::Path, args: &[&str]) {
        let ok = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {:?} failed", args);
    }

    #[test]
    fn non_git_dir_returns_empty_not_error() {
        // A directory that isn't a git repo must degrade to empty data so the
        // episode can still be built from the agent session alone.
        let dir = tempfile::tempdir().unwrap();
        let data = collect(dir.path().to_str().unwrap(), 1).expect("should not error");
        assert!(data.changed_files.is_empty());
        assert!(data.diff_hunks.is_empty());
        assert!(data.commit_messages.is_empty());
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
        // uncommitted change -> shows as changed file
        std::fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();

        let data = collect(p.to_str().unwrap(), 7).unwrap();
        assert!(data.commit_messages.iter().any(|m| m == "first commit"));
        assert!(data.changed_files.iter().any(|f| f == "a.txt"));
        assert!(data.diff_summary.is_some());
        // diff hunks include the added line content
        let patch = data
            .diff_hunks
            .iter()
            .find(|(p, _)| p == "a.txt")
            .map(|(_, t)| t.clone())
            .expect("hunk for a.txt");
        assert!(
            patch.contains("+two"),
            "patch should show added line, got:\n{patch}"
        );
    }
}
