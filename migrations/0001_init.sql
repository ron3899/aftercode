CREATE TABLE users (
  id BLOB PRIMARY KEY NOT NULL,
  email TEXT UNIQUE NOT NULL,
  token_hash TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE projects (
  id BLOB PRIMARY KEY NOT NULL,
  user_id BLOB NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  default_language TEXT NOT NULL DEFAULT 'en',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE coding_sessions (
  id BLOB PRIMARY KEY NOT NULL,
  project_id BLOB NOT NULL REFERENCES projects(id),
  user_id BLOB NOT NULL REFERENCES users(id),
  source TEXT,
  context_json TEXT NOT NULL,
  summary TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE podcast_episodes (
  id BLOB PRIMARY KEY NOT NULL,
  user_id BLOB NOT NULL REFERENCES users(id),
  project_id BLOB NOT NULL REFERENCES projects(id),
  session_id BLOB NOT NULL REFERENCES coding_sessions(id),
  title TEXT NOT NULL DEFAULT '',
  language TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued','extracting_topics','writing_script','generating_audio','ready','failed')),
  duration_seconds INTEGER,
  audio_url TEXT,
  script_json TEXT,
  topics_json TEXT,
  transcript_text TEXT,
  summary TEXT,
  error TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
