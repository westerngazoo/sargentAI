-- R-0014: user_programs table (SPEC-0014 §2.3)
-- Stores the chosen program+diet proposal for each user.
-- One row is active at a time; choosing a new program deactivates the previous.
-- source_session_id is nullable so future non-photo code paths can omit it.

CREATE TABLE user_programs (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archetype_id      TEXT        NOT NULL,
    source_session_id UUID        REFERENCES photo_sessions(id) ON DELETE SET NULL,
    program           JSONB       NOT NULL,
    diet              JSONB       NOT NULL,
    active            BOOLEAN     NOT NULL DEFAULT TRUE,
    chosen_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_programs_user_active ON user_programs (user_id, active);
