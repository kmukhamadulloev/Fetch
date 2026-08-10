PRAGMA user_version = 2;

CREATE TABLE download_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_json TEXT NOT NULL,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX download_jobs_status_idx ON download_jobs(status);
CREATE INDEX download_jobs_created_at_idx ON download_jobs(created_at DESC);

CREATE TABLE completed_files (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES download_jobs(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK(size_bytes >= 0),
    mime_type TEXT NOT NULL,
    title TEXT,
    browser_playable INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE INDEX completed_files_created_at_idx ON completed_files(created_at DESC);

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE diagnostic_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    subsystem TEXT NOT NULL,
    message TEXT NOT NULL,
    details TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX diagnostic_logs_created_at_idx ON diagnostic_logs(created_at DESC);
