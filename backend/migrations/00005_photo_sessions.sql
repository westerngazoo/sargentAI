-- R-0006 / SPEC-0006 — photo sessions and their photos (metadata only).
-- Image bytes live in the object store; `storage_key` is the only link.
-- Validation lives in crates/core (SPEC-0002 OQ-A1); the DB enforces
-- referential integrity and the FK cascades (user → sessions → photos).

CREATE TABLE photo_sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    performed_on DATE NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_photo_sessions_user_id ON photo_sessions (user_id);

CREATE TABLE photo_session_photos (
    id           UUID PRIMARY KEY,
    session_id   UUID NOT NULL REFERENCES photo_sessions(id) ON DELETE CASCADE,
    angle        TEXT,
    storage_key  TEXT NOT NULL,
    content_type TEXT NOT NULL,
    byte_size    BIGINT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_photo_session_photos_session_id
    ON photo_session_photos (session_id);
