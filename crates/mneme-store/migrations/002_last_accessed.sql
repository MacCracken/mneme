ALTER TABLE notes ADD COLUMN last_accessed TEXT NOT NULL DEFAULT (datetime('now'));
CREATE INDEX IF NOT EXISTS idx_notes_last_accessed ON notes(last_accessed);
