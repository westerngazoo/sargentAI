-- R-0003 / SPEC-0003 — user_profiles table (1:1 with users).
-- Validation (ranges, enum vocabularies, non-empty goals) lives in
-- crates/core, not in DB CHECKs (follows SPEC-0002 OQ-A1). The DB enforces
-- referential integrity only.

CREATE TABLE user_profiles (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    date_of_birth       DATE NOT NULL,
    height_cm           INTEGER NOT NULL,
    weight_kg           DOUBLE PRECISION NOT NULL,
    sex                 TEXT,
    body_fat_percentage DOUBLE PRECISION,
    goals               TEXT[] NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
