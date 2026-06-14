use std::path::Path;

/// Read captured terminal errors from .aftercode/errors.log (one per line). Optional.
pub fn collect() -> Vec<String> {
    let path = Path::new(".aftercode").join("errors.log");
    std::fs::read_to_string(path)
        .ok()
        .map(|t| {
            t.lines()
                .filter(|l| !l.trim().is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}
