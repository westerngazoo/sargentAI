-- R-0004 / SPEC-0004 — workout log (sessions → exercises → sets).
-- Validation lives in crates/core (SPEC-0002 OQ-A1); the DB enforces
-- referential integrity and ordering support only.

CREATE TABLE workout_sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    performed_on DATE NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_workout_sessions_user_id ON workout_sessions (user_id);

CREATE TABLE workout_exercises (
    id           UUID PRIMARY KEY,
    session_id   UUID NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    name         TEXT NOT NULL,
    muscle_group TEXT
);
CREATE INDEX idx_workout_exercises_session_id ON workout_exercises (session_id);

CREATE TABLE workout_sets (
    id          UUID PRIMARY KEY,
    exercise_id UUID NOT NULL REFERENCES workout_exercises(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    reps        INTEGER NOT NULL,
    weight_kg   DOUBLE PRECISION,
    rpe         DOUBLE PRECISION
);
CREATE INDEX idx_workout_sets_exercise_id ON workout_sets (exercise_id);
