use regex::Regex;

/// Returns true if the text appears to contain a secret.
pub fn contains_secret(text: &str) -> bool {
    let patterns = [
        r"(?i)api[_-]?key\s*[:=]\s*['\x22]?[A-Za-z0-9_\-]{16,}",
        r"sk-[A-Za-z0-9]{20,}",        // OpenAI-style
        r"sk-ant-[A-Za-z0-9_\-]{20,}", // Anthropic-style
        r"AKIA[0-9A-Z]{16}",           // AWS access key id
        r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----",
        r"(?i)(secret|password|passwd|token)\s*[:=]\s*['\x22]?\S{8,}",
    ];
    patterns
        .iter()
        .any(|p| Regex::new(p).unwrap().is_match(text))
}

/// Replace any line containing a secret with a redaction marker.
pub fn redact(text: &str) -> String {
    text.lines()
        .map(|l| {
            if contains_secret(l) {
                "[REDACTED — secret detected]"
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
