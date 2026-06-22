# Agent Task — Write Requirement R-0031 (Nutrition LLM Substitution)

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** create `R-0031-nutrition-substitution` from `main`, push to it
**Output file:** `requirements/0031-nutrition-substitution.md`

---

## What you are doing

You are writing a new accepted requirement file for the fitAI project. The owner
wants a feature where users can ask "I don't have [food X], what can I use
instead?" and the app responds with macro-equivalent substitutes — powered by a
Claude API call on the backend.

---

## Step 1 — Read these files before writing anything

```
requirements/0014-program-diet-from-archetype.md   ← requirement format to match exactly
backend/crates/core/src/program/mod.rs             ← GeneratedDiet struct (macros)
backend/crates/api/src/lib.rs                      ← existing routes
backend/crates/api/src/error.rs                    ← ApiError pattern to follow
CLAUDE.md                                          ← engineering constitution
project-specifics.md                               ← stack (Rust + Flutter)
```

---

## Step 2 — Context

The system generates a diet plan per user (macros: protein/carbs/fat/kcal,
approach, meal structure — see `GeneratedDiet`). Users follow this plan but
sometimes a specific food isn't available. This feature lets them ask for a
swap in real time.

**How it works:**
1. Flutter sends `POST /nutrition/substitute` with `{ food: "chicken breast",
   quantity_g: 150 }` (authenticated)
2. Backend fetches the user's active diet plan (macro targets + approach) from
   `user_programs`
3. Backend calls the **Claude API** (`claude-haiku-4-5-20251001` — cost-
   efficient) with a prompt that includes: the food + quantity, the user's macro
   targets, the diet approach (e.g. "high-protein clean bulk")
4. Claude returns 2–3 substitute options with quantities and macro breakdown
5. Backend parses the response and returns structured JSON to the client
6. Flutter shows a simple results card: substitute name, quantity, macros

**Rate limit:** 10 calls per user per day (Postgres counter, resets at midnight
UTC) to control LLM costs.

**Claude model decided:** `claude-haiku-4-5-20251001` — record in decision log.

---

## Step 3 — Write the requirement file

Follow the **exact format** of `requirements/0014-program-diet-from-archetype.md`.

**Metadata:**
```
Status: Accepted
Milestone: M5 (intelligence) — lightweight LLM feature
Created: 2026-06-21
Depends on: R-0014 (Done — GeneratedDiet and user_programs table)
```

**Statement:** A backend endpoint and Flutter UI that answer "what can I eat
instead of X?" by calling the Claude API with the user's active macro targets as
context and returning 2–3 macro-equivalent food substitutes.

**Acceptance criteria (write 10–12)** covering:
- AC1: `POST /nutrition/substitute` (authenticated) accepts
  `{ food: string, quantity_g: number }`, returns
  `{ substitutes: [{ food, quantity_g, protein_g, carbs_g, fat_g, kcal, note }] }`
  with HTTP 200.
- AC2: The endpoint fetches the user's active `UserProgram` diet macros before
  calling Claude. If no active program exists, Claude is called without macro
  context (still useful, just less personalised).
- AC3: The Claude API call uses model `claude-haiku-4-5-20251001`. The
  `ANTHROPIC_API_KEY` is read from an environment variable — never hardcoded.
- AC4: The prompt includes: the food + quantity, the user's daily macro targets
  (protein/carbs/fat/kcal), and the diet approach string. The prompt is
  unit-tested with snapshot testing (input → expected prompt string shape).
- AC5: Claude's response is parsed into the structured substitute list. If Claude
  returns fewer than 2 or more than 3 items, or malformed JSON, the endpoint
  returns 502 with `{ error: "upstream_parse_error" }`.
- AC6: Rate limit: max 10 calls per user per 24-hour window (UTC). Exceeding
  returns 429 `{ error: "rate_limit_exceeded", resets_at: "<ISO timestamp>" }`.
  The counter lives in a `nutrition_substitute_calls` Postgres table (one row
  per user, date, count).
- AC7: If the Claude API is unavailable or times out (5-second timeout), return
  503 `{ error: "service_unavailable", retryable: true }`. No crash.
- AC8: A new database migration creates the `nutrition_substitute_calls` table.
- AC9: Flutter: a "Can't find it?" button on the diet plan screen opens a sheet
  with a text input (food name) + quantity field. Submitting calls the endpoint
  and shows results as a list of cards (food name, quantity, macro breakdown).
- AC10: Flutter handles 429 with a user-visible message: "Daily limit reached.
  Try again tomorrow." Handles 503 with a retry button.
- AC11: Tests — backend: unit test for prompt construction; integration test
  stubs the Claude HTTP client and asserts request shape + response parsing +
  rate-limit enforcement. Flutter: widget test for the substitution sheet
  (input → loading → results; 429 path; 503 path).
- AC12: Scope guard — no fine-tuning, no model training, no stored LLM
  responses, no chat history. Single-shot request per call. No changes to the
  diet plan itself (read-only context).

**Constraints:**
- `ANTHROPIC_API_KEY` in environment only
- No response caching (answers are context-specific)
- No streaming (single JSON response)
- Rate limit is per-user, not global

**Open questions to defer to SPEC-0031:**
- Exact prompt template and few-shot examples
- Rust HTTP client for Anthropic API: `reqwest` with manual JSON vs an SDK
- Whether to use Claude's JSON mode / tool use to enforce structured output
- Whether the Flutter sheet is a bottom sheet or a full screen

**Decision log:** `claude-haiku-4-5-20251001`, ANTHROPIC_API_KEY env var, 10/day
rate limit, 5-second timeout, no caching — all 2026-06-21.

Mark status **Accepted**.

---

## Step 4 — Commit and push

```bash
git checkout main
git checkout -b R-0031-nutrition-substitution
# write the file
git add requirements/0031-nutrition-substitution.md
git commit -m "R-0031: step-1 requirement — nutrition LLM substitution (Accepted)"
git push -u origin R-0031-nutrition-substitution
```
