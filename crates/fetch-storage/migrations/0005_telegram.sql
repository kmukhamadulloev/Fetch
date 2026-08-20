PRAGMA user_version = 5;

CREATE TABLE telegram_state (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    polling_offset INTEGER NOT NULL DEFAULT 0 CHECK(polling_offset >= 0),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO telegram_state (singleton, polling_offset) VALUES (1, 0);

CREATE TABLE telegram_updates (
    update_id INTEGER PRIMARY KEY NOT NULL CHECK(update_id >= 0),
    claimed_at TEXT NOT NULL
);

CREATE TABLE telegram_pending_actions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL CHECK(user_id > 0),
    action_json TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed_at TEXT
);

CREATE INDEX telegram_pending_actions_owner_idx
    ON telegram_pending_actions(user_id, expires_at);

CREATE TABLE telegram_job_owners (
    job_id TEXT PRIMARY KEY NOT NULL REFERENCES download_jobs(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL CHECK(user_id > 0),
    chat_id INTEGER NOT NULL
);

CREATE INDEX telegram_job_owners_user_idx ON telegram_job_owners(user_id);
