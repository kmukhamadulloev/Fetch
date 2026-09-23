-- Rebuild the parent with nullable download linkage. Preserve dependent rows
-- explicitly: DROP TABLE would otherwise cascade playback and reject journals.
CREATE TEMP TABLE saved_playback AS SELECT * FROM playback_progress;
CREATE TEMP TABLE saved_metadata_edits AS SELECT * FROM metadata_edits;
DROP TABLE playback_progress;
DROP TABLE metadata_edits;
CREATE TABLE completed_files_new (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT REFERENCES download_jobs(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK(size_bytes >= 0),
    mime_type TEXT NOT NULL,
    title TEXT,
    browser_playable INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    thumbnail_path TEXT,
    origin TEXT CHECK(origin IN ('conversion', 'edit')),
    source_file_id TEXT,
    CHECK ((job_id IS NOT NULL AND origin IS NULL AND source_file_id IS NULL)
        OR (job_id IS NULL AND origin IS NOT NULL AND source_file_id IS NOT NULL))
);
INSERT INTO completed_files_new
    (id, job_id, filename, path, size_bytes, mime_type, title, browser_playable, created_at, thumbnail_path)
    SELECT id, job_id, filename, path, size_bytes, mime_type, title, browser_playable, created_at, thumbnail_path FROM completed_files;
DROP TABLE completed_files;
ALTER TABLE completed_files_new RENAME TO completed_files;
CREATE INDEX completed_files_created_at_idx ON completed_files(created_at DESC);
CREATE TABLE playback_progress (
    file_id TEXT PRIMARY KEY NOT NULL REFERENCES completed_files(id) ON DELETE CASCADE,
    position_seconds REAL NOT NULL CHECK(position_seconds >= 0),
    duration_seconds REAL NOT NULL CHECK(duration_seconds > 0),
    completed INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
INSERT INTO playback_progress SELECT * FROM saved_playback;
DROP TABLE saved_playback;
CREATE TABLE metadata_edits (
    file_id TEXT PRIMARY KEY NOT NULL REFERENCES completed_files(id),
    journal_json TEXT NOT NULL,
    committed INTEGER NOT NULL DEFAULT 0
);
INSERT INTO metadata_edits SELECT * FROM saved_metadata_edits;
DROP TABLE saved_metadata_edits;
CREATE TABLE processing_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    record_json TEXT NOT NULL,
    private_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX processing_jobs_created_idx ON processing_jobs(created_at DESC);
PRAGMA user_version = 8;
