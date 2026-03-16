ALTER TABLE notes ADD COLUMN provenance TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE notes ADD COLUMN trust_override REAL;
