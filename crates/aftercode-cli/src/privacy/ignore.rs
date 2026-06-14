use ignore::gitignore::GitignoreBuilder;

/// Build a matcher from the config ignore_paths and report whether a path is ignored.
pub struct Matcher {
    inner: ignore::gitignore::Gitignore,
}

impl Matcher {
    pub fn new(patterns: &[String]) -> anyhow::Result<Self> {
        let mut b = GitignoreBuilder::new(".");
        for p in patterns {
            b.add_line(None, p)?;
        }
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
