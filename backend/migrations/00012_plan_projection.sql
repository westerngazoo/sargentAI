-- R-0061 / SPEC-0061 §2.3 — the plan projection.
-- Additive only. `starts_on` keeps a default so an image rolled back past
-- this migration still inserts programs without naming the column.
-- No CHECK pairs the two session columns: ON DELETE SET NULL would break it.
-- The API enforces the pairing (§2.4).

ALTER TABLE user_programs
  ADD COLUMN starts_on DATE NOT NULL DEFAULT ((now() AT TIME ZONE 'UTC')::date);
UPDATE user_programs SET starts_on = (chosen_at AT TIME ZONE 'UTC')::date;

ALTER TABLE workout_sessions
  ADD COLUMN user_program_id UUID REFERENCES user_programs(id) ON DELETE SET NULL,
  ADD COLUMN plan_day_index  SMALLINT;
CREATE INDEX idx_workout_sessions_user_program ON workout_sessions (user_program_id);
