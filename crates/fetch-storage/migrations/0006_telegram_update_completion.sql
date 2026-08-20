PRAGMA user_version = 6;

ALTER TABLE telegram_updates ADD COLUMN completed_at TEXT;

CREATE INDEX telegram_updates_completed_idx ON telegram_updates(completed_at, update_id);
