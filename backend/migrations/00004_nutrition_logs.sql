-- R-0005 / SPEC-0005 — nutrition log (one row per user per day).
-- Validation lives in crates/core (SPEC-0002 OQ-A1); the DB enforces
-- referential integrity, per-day uniqueness, and the list-lookup index only.
-- Calories are derived (4·protein + 4·carbs + 9·fat), never stored.

CREATE TABLE nutrition_logs (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    performed_on DATE NOT NULL,
    protein_g    DOUBLE PRECISION NOT NULL,
    carbs_g      DOUBLE PRECISION NOT NULL,
    fat_g        DOUBLE PRECISION NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, performed_on)
);
CREATE INDEX idx_nutrition_logs_user_id ON nutrition_logs (user_id);
