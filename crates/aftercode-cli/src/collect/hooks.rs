use aftercode_core::session::CodingEvent;
use std::path::Path;

/// Read newline-delimited JSON CodingEvents from .aftercode/events/<date>.jsonl
/// for the given dates (YYYY-MM-DD strings). Missing files are skipped.
pub fn collect(dates: &[String]) -> anyhow::Result<Vec<CodingEvent>> {
    let mut events = Vec::new();
    for d in dates {
        let path = Path::new(".aftercode")
            .join("events")
            .join(format!("{d}.jsonl"));
        let Ok(txt) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in txt.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(ev) = serde_json::from_str::<CodingEvent>(line) {
                events.push(ev);
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aftercode_core::session::EventType;
    #[test]
    #[serial_test::serial(fs)]
    fn reads_jsonl_events() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all(".aftercode/events").unwrap();
        let ev = CodingEvent {
            event_type: EventType::UserPrompt,
            timestamp: "2026-06-14T10:00:00Z".into(),
            content: "fix this".into(),
            metadata: serde_json::json!({}),
        };
        std::fs::write(
            ".aftercode/events/2026-06-14.jsonl",
            format!("{}\n", serde_json::to_string(&ev).unwrap()),
        )
        .unwrap();
        let out = collect(&["2026-06-14".into()]).unwrap();
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "fix this");
    }
}
