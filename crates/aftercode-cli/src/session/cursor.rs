use super::{AgentSession, SessionReader};
use aftercode_core::session::{CodingEvent, EventType};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::Path;

pub struct CursorReader;

impl SessionReader for CursorReader {
    fn agent_name(&self) -> &'static str {
        "cursor"
    }

    fn latest_session(&self, project_dir: &Path) -> Option<AgentSession> {
        // macOS: ~/Library/Application Support/Cursor/User ; Linux: ~/.config/Cursor/User
        let user_dir = dirs::config_dir()?.join("Cursor").join("User");
        read_session(&user_dir, project_dir)
    }
}

/// Best-effort: never panics. Any error / missing data => None.
/// Public + base-dir injectable for testing.
pub fn read_session(user_dir: &Path, project_dir: &Path) -> Option<AgentSession> {
    let ws_storage = user_dir.join("workspaceStorage");
    let hash_dir = find_workspace(&ws_storage, project_dir)?;

    let ws_db = hash_dir.join("state.vscdb");
    let composer_ids = workspace_composer_ids(&ws_db);
    if composer_ids.is_empty() {
        return None;
    }

    let global_db = user_dir.join("globalStorage").join("state.vscdb");
    let conn = open_ro(&global_db)?;

    // Pick the most-recently-updated composer for this workspace.
    let mut best: Option<(String, Value, i64)> = None; // (id, composerData, lastUpdated secs)
    for cid in &composer_ids {
        let Some(blob) = kv_get(&conn, &format!("composerData:{cid}")) else {
            continue;
        };
        let Ok(d) = serde_json::from_str::<Value>(&blob) else {
            continue;
        };
        let updated = d
            .get("lastUpdatedAt")
            .or_else(|| d.get("createdAt"))
            .and_then(|v| v.as_i64())
            .map(|ms| ms / 1000)
            .unwrap_or(0);
        if best.as_ref().map(|(_, _, u)| updated > *u).unwrap_or(true) {
            best = Some((cid.clone(), d, updated));
        }
    }
    let (cid, data, ended_at) = best?;

    let events = extract_bubbles(&conn, &cid, &data);
    if events.is_empty() {
        return None;
    }
    Some(AgentSession {
        agent: "cursor".into(),
        ended_at,
        events,
    })
}

/// Walk `fullConversationHeadersOnly` and fetch each `bubbleId:<cid>:<bid>` row.
fn extract_bubbles(conn: &Connection, cid: &str, data: &Value) -> Vec<CodingEvent> {
    let mut events = Vec::new();
    let headers = data
        .get("fullConversationHeadersOnly")
        .and_then(|v| v.as_array());
    let Some(headers) = headers else {
        return events;
    };
    for h in headers {
        let Some(bid) = h.get("bubbleId").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(raw) = kv_get(conn, &format!("bubbleId:{cid}:{bid}")) else {
            continue;
        };
        let Ok(b) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let text = b.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if text.trim().is_empty() {
            continue;
        }
        // bubble type: 1 = user, 2 = assistant
        let is_user = match b
            .get("type")
            .and_then(|v| v.as_i64())
            .or_else(|| h.get("type").and_then(|v| v.as_i64()))
        {
            Some(1) => true,
            Some(2) => false,
            _ => false,
        };
        events.push(CodingEvent {
            event_type: if is_user {
                EventType::UserPrompt
            } else {
                EventType::AgentResponse
            },
            timestamp: String::new(),
            content: text.to_string(),
            metadata: Value::Null,
        });
    }
    events
}

/// Find the workspaceStorage hash dir whose workspace.json folder == project_dir.
fn find_workspace(ws_storage: &Path, project_dir: &Path) -> Option<std::path::PathBuf> {
    let want = canon(project_dir);
    for entry in std::fs::read_dir(ws_storage).ok()?.flatten() {
        let wj = entry.path().join("workspace.json");
        let Ok(txt) = std::fs::read_to_string(&wj) else {
            continue;
        };
        let Ok(d) = serde_json::from_str::<Value>(&txt) else {
            continue;
        };
        if let Some(folder) = d.get("folder").and_then(|v| v.as_str()) {
            let path = folder.strip_prefix("file://").unwrap_or(folder);
            let path = percent_decode(path);
            if canon(Path::new(&path)) == want {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Composer ids referenced by this workspace (allComposers + selected/lastFocused).
fn workspace_composer_ids(ws_db: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Some(conn) = open_ro(ws_db) else {
        return out;
    };
    let Some(blob) = kv_get(&conn, "composer.composerData") else {
        return out;
    };
    let Ok(d) = serde_json::from_str::<Value>(&blob) else {
        return out;
    };
    if let Some(arr) = d.get("allComposers").and_then(|v| v.as_array()) {
        for c in arr {
            if let Some(id) = c.get("composerId").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    for key in ["selectedComposerIds", "lastFocusedComposerIds"] {
        if let Some(arr) = d.get(key).and_then(|v| v.as_array()) {
            for id in arr.iter().filter_map(|v| v.as_str()) {
                if !out.iter().any(|e| e == id) {
                    out.push(id.to_string());
                }
            }
        }
    }
    out
}

fn open_ro(path: &Path) -> Option<Connection> {
    let p = path.to_string_lossy();
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    // `mode=ro` (read-only, NOT immutable) merges the -wal, so an active Cursor
    // session — whose newest data lives in the WAL, not the main db file — is
    // visible. `immutable=1` ignores the WAL and would miss/mis-rank it.
    // Fall back to immutable when ro can't open (e.g. Cursor closed, no -shm).
    Connection::open_with_flags(format!("file:{p}?mode=ro"), flags)
        .or_else(|_| Connection::open_with_flags(format!("file:{p}?immutable=1"), flags))
        .ok()
}

fn kv_get(conn: &Connection, key: &str) -> Option<String> {
    for table in ["cursorDiskKV", "ItemTable"] {
        let sql = format!("SELECT value FROM {table} WHERE key = ?1");
        if let Ok(mut stmt) = conn.prepare(&sql) {
            let got: rusqlite::Result<String> = stmt.query_row([key], |r| {
                r.get::<_, String>(0).or_else(|_| {
                    r.get::<_, Vec<u8>>(0)
                        .map(|b| String::from_utf8_lossy(&b).into_owned())
                })
            });
            if let Ok(v) = got {
                return Some(v);
            }
        }
    }
    None
}

fn canon(p: &Path) -> String {
    std::fs::canonicalize(p)
        .unwrap_or_else(|_| p.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_db(path: &Path, table: &str, rows: &[(&str, &str)]) {
        let conn = Connection::open(path).unwrap();
        conn.execute(
            &format!("CREATE TABLE {table} (key TEXT PRIMARY KEY, value TEXT)"),
            [],
        )
        .unwrap();
        for (k, v) in rows {
            conn.execute(
                &format!("INSERT INTO {table} (key,value) VALUES (?1,?2)"),
                [k, v],
            )
            .unwrap();
        }
    }

    #[test]
    fn reads_synthetic_cursor_session() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("User");
        let proj = tmp.path().join("myproj");
        std::fs::create_dir_all(&proj).unwrap();
        let proj_canon = canon(&proj);

        let ws = user.join("workspaceStorage").join("hash1");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(
            ws.join("workspace.json"),
            format!("{{\"folder\":\"file://{proj_canon}\"}}"),
        )
        .unwrap();
        make_db(
            &ws.join("state.vscdb"),
            "ItemTable",
            &[(
                "composer.composerData",
                r#"{"allComposers":[{"composerId":"c1"}]}"#,
            )],
        );

        let global = user.join("globalStorage");
        std::fs::create_dir_all(&global).unwrap();
        make_db(
            &global.join("state.vscdb"),
            "cursorDiskKV",
            &[
                (
                    "composerData:c1",
                    r#"{"composerId":"c1","lastUpdatedAt":1700000000000,"fullConversationHeadersOnly":[{"bubbleId":"b1","type":1},{"bubbleId":"b2","type":2}]}"#,
                ),
                ("bubbleId:c1:b1", r#"{"type":1,"text":"add redis caching"}"#),
                (
                    "bubbleId:c1:b2",
                    r#"{"type":2,"text":"Added a Redis client and config."}"#,
                ),
            ],
        );

        let sess = read_session(&user, &proj).expect("should read");
        assert_eq!(sess.agent, "cursor");
        assert_eq!(sess.ended_at, 1_700_000_000);
        let c: Vec<_> = sess
            .events
            .iter()
            .map(|e| (e.event_type, e.content.as_str()))
            .collect();
        assert_eq!(c[0], (EventType::UserPrompt, "add redis caching"));
        assert_eq!(
            c[1],
            (EventType::AgentResponse, "Added a Redis client and config.")
        );
    }

    #[test]
    fn missing_workspace_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("User");
        std::fs::create_dir_all(user.join("workspaceStorage")).unwrap();
        assert!(read_session(&user, tmp.path()).is_none());
    }

    #[test]
    fn malformed_blob_degrades_to_none() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("User");
        let proj = tmp.path().join("p");
        std::fs::create_dir_all(&proj).unwrap();
        let pc = canon(&proj);
        let ws = user.join("workspaceStorage").join("h");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(
            ws.join("workspace.json"),
            format!("{{\"folder\":\"file://{pc}\"}}"),
        )
        .unwrap();
        make_db(
            &ws.join("state.vscdb"),
            "ItemTable",
            &[("composer.composerData", "{not json")],
        );
        let global = user.join("globalStorage");
        std::fs::create_dir_all(&global).unwrap();
        make_db(&global.join("state.vscdb"), "cursorDiskKV", &[]);
        assert!(read_session(&user, &proj).is_none());
    }

    /// Live check against real Cursor data. Ignored by default; run with:
    /// `AFTERCODE_CURSOR_LIVE=/abs/path/to/repo cargo test -p aftercode-cli -- --ignored live_cursor`
    #[test]
    #[ignore]
    fn live_cursor() {
        let proj = std::env::var("AFTERCODE_CURSOR_LIVE").expect("set AFTERCODE_CURSOR_LIVE");
        let user = dirs::config_dir().unwrap().join("Cursor").join("User");
        let sess = read_session(&user, Path::new(&proj));
        match sess {
            Some(s) => {
                eprintln!("cursor: {} events, ended_at={}", s.events.len(), s.ended_at);
                for e in s.events.iter().take(4) {
                    eprintln!(
                        "  {:?}: {}",
                        e.event_type,
                        &e.content[..e.content.len().min(80)]
                    );
                }
                assert!(!s.events.is_empty());
            }
            None => panic!("no cursor session found for {proj}"),
        }
    }
}
