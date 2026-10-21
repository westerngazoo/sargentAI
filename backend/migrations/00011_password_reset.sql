-- R-0060 / SPEC-0060 §2.1 — password reset tokens, and the marker that ends
-- sessions issued before a reset (AC4).

-- NULL means "never changed", so every existing session stays valid and the
-- migration is additive.
ALTER TABLE users ADD COLUMN password_changed_at TIMESTAMPTZ;

CREATE TABLE password_reset_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  BYTEA NOT NULL UNIQUE,      -- SHA-256 of the token, never the token
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ
);
CREATE INDEX password_reset_tokens_user_id ON password_reset_tokens(user_id);

-- Rate-limit ledger (AC5). `key_hash` is SHA-256 of "email:<normalized>" or
-- "ip:<address>", so the ledger stores neither emails nor addresses. Rows
-- older than the one-hour window are purged by the handler on every insert.
CREATE TABLE password_reset_requests (
    key_hash     BYTEA NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX password_reset_requests_key ON password_reset_requests(key_hash, requested_at);
