-- R-0041: authored_programs table (SPEC-0041 §2.1)
-- Stores a trainer/self-authored `core::authoring::AuthoredProgram` whole, as
-- JSONB — the domain model is already the serialization contract, so a
-- relational shred would only duplicate it and drift. `name` is denormalized
-- out of the JSONB so the list endpoint is a pure SQL projection that never
-- deserializes a program. No uniqueness on `name`: the id is the identity.

CREATE TABLE authored_programs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    program     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- `id` is part of the ordering, not decoration: `created_at DESC` alone is not
-- a total order, so two programs created in the same tick could swap places
-- between requests. It also makes the index keyset-paginatable later.
CREATE INDEX idx_authored_programs_user_created
    ON authored_programs (user_id, created_at DESC, id DESC);
