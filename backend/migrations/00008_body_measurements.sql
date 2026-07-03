-- Body-measurement log (one row per user per day) for progress charts.
-- Weight is required; body-fat % is optional. Lean mass is derived
-- (weight × (1 − bf%)), never stored. Referential integrity + per-day
-- uniqueness + the list-lookup index are the DB's job.

CREATE TABLE body_measurements (
    id                  UUID PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    measured_on         DATE NOT NULL,
    weight_kg           DOUBLE PRECISION NOT NULL,
    body_fat_percentage DOUBLE PRECISION,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, measured_on)
);
CREATE INDEX idx_body_measurements_user_id ON body_measurements (user_id, measured_on);
