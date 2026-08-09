-- R-0042: goals table (SPEC-0042 §2.5)
-- Stores a `core::goals::Goal` whole, as JSONB — the R-0041 precedent: the
-- domain model is already the serialization contract, and nothing queries the
-- dates today, so a relational shred would be premature. No status column:
-- derived state stored is derived state stale (AC8) — every read recomputes
-- the pace report from the trailing signal.

CREATE TABLE goals (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    goal        JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- `id` is part of the ordering, not decoration: `created_at DESC` alone is
-- not a total order, so two goals created in the same tick could swap places
-- between requests. It also makes the index keyset-paginatable later.
CREATE INDEX idx_goals_user_created ON goals (user_id, created_at DESC, id DESC);
