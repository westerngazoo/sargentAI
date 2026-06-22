# R-0031 — Nutrition LLM Substitution

- **Status:** Accepted
- **Milestone:** M5 (intelligence) — lightweight LLM feature
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-21
- **Depends on:** R-0014 (Done — GeneratedDiet and user_programs table)
- **Realized by:** SPEC-0031 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A backend endpoint and Flutter UI that answer "what can I eat
instead of X?" by calling the Claude API with the user's active macro targets as
context and returning 2–3 macro-equivalent food substitutes.

## 2. Rationale

The system generates a diet plan per user (macros: protein/carbs/fat/kcal,
approach, meal structure — see `GeneratedDiet`). Users follow this plan but
sometimes a specific food isn't available. This feature lets them ask for a
swap in real time.

## 3. Acceptance criteria

- **AC1.** `POST /nutrition/substitute` (authenticated) accepts
  `{ food: string, quantity_g: number }`, returns
  `{ substitutes: [{ food, quantity_g, protein_g, carbs_g, fat_g, kcal, note }] }`
  with HTTP 200.
- **AC2.** The endpoint fetches the user's active `UserProgram` diet macros before
  calling Claude. If no active program exists, Claude is called without macro
  context (still useful, just less personalised).
- **AC3.** The Claude API call uses model `claude-haiku-4-5-20251001`. The
  `ANTHROPIC_API_KEY` is read from an environment variable — never hardcoded.
- **AC4.** The prompt includes: the food + quantity, the user's daily macro targets
  (protein/carbs/fat/kcal), and the diet approach string. The prompt is
  unit-tested with snapshot testing (input → expected prompt string shape).
- **AC5.** Claude's response is parsed into the structured substitute list. If Claude
  returns fewer than 2 or more than 3 items, or malformed JSON, the endpoint
  returns 502 with `{ error: "upstream_parse_error" }`.
- **AC6.** Rate limit: max 10 calls per user per 24-hour window (UTC). Exceeding
  returns 429 `{ error: "rate_limit_exceeded", resets_at: "<ISO timestamp>" }`.
  The counter lives in a `nutrition_substitute_calls` Postgres table (one row
  per user, date, count).
- **AC7.** If the Claude API is unavailable or times out (5-second timeout), return
  503 `{ error: "service_unavailable", retryable: true }`. No crash.
- **AC8.** A new database migration creates the `nutrition_substitute_calls` table.
- **AC9.** Flutter: a "Can't find it?" button on the diet plan screen opens a sheet
  with a text input (food name) + quantity field. Submitting calls the endpoint
  and shows results as a list of cards (food name, quantity, macro breakdown).
- **AC10.** Flutter handles 429 with a user-visible message: "Daily limit reached.
  Try again tomorrow." Handles 503 with a retry button.
- **AC11.** Tests — backend: unit test for prompt construction; integration test
  stubs the Claude HTTP client and asserts request shape + response parsing +
  rate-limit enforcement. Flutter: widget test for the substitution sheet
  (input → loading → results; 429 path; 503 path).
- **AC12.** Scope guard — no fine-tuning, no model training, no stored LLM
  responses, no chat history. Single-shot request per call. No changes to the
  diet plan itself (read-only context).

## 4. Constraints & non-goals

- `ANTHROPIC_API_KEY` in environment only
- No response caching (answers are context-specific)
- No streaming (single JSON response)
- Rate limit is per-user, not global

## 5. Open questions

Deferred to SPEC-0031:

- Exact prompt template and few-shot examples
- Rust HTTP client for Anthropic API: `reqwest` with manual JSON vs an SDK
- Whether to use Claude's JSON mode / tool use to enforce structured output
- Whether the Flutter sheet is a bottom sheet or a full screen

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-21 | **`claude-haiku-4-5-20251001` model** | Cost-efficient and fast enough for this simple substitution task. |
| 2026-06-21 | **ANTHROPIC_API_KEY env var** | Secure handling of credentials. |
| 2026-06-21 | **10/day rate limit per user** | Prevents abuse and controls costs. |
| 2026-06-21 | **5-second timeout** | Ensures the UI remains responsive. |
| 2026-06-21 | **No caching** | Substitutions are highly contextual depending on user's macros and specific food items. |

## Changelog

- _2026-06-21 — created and **Accepted**._
