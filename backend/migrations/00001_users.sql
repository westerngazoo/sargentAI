-- R-0002 / SPEC-0002 — users table.
-- Holds exactly what authentication needs; profile fields live elsewhere
-- (R-0003 adds them to the same table or to a sibling, decided in SPEC-0003).

CREATE EXTENSION IF NOT EXISTS "pgcrypto";  -- for gen_random_uuid()

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- No separate index on `email`: the UNIQUE constraint already creates a
-- backing B-tree index (OQ-A3, architect-confirmed in SPEC-0002 §7).
