PRAGMA user_version = 4;

CREATE TABLE playback_progress (
    file_id TEXT PRIMARY KEY NOT NULL REFERENCES completed_files(id) ON DELETE CASCADE,
    position_seconds REAL NOT NULL CHECK(position_seconds >= 0),
    duration_seconds REAL NOT NULL CHECK(duration_seconds > 0),
    completed INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
