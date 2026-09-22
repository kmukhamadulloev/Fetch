CREATE TABLE metadata_edits (
    file_id TEXT PRIMARY KEY NOT NULL REFERENCES completed_files(id),
    journal_json TEXT NOT NULL,
    committed INTEGER NOT NULL DEFAULT 0
);
