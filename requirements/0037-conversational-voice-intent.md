# R-0037 — Conversational Voice Intent (multi-turn, tool-calling)

- **Status:** Draft
- **Milestone:** M9 (Voice Assistant & Automation)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-09
- **Depends on:** R-0032 (voice logging — the single-shot path this evolves),
                  R-0004 (workout log), R-0005 (nutrition log)
- **Realized by:** SPEC-0037 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Evolve voice logging from a single-shot parser into a **multi-turn conversation
that holds context and asks for what's missing**. When the user speaks an
incomplete command ("log a meal", "I benched"), the assistant asks a natural
follow-up ("How many grams of protein, carbs, and fat?"), remembers the prior
turns, and only writes the log once it has everything — instead of failing or
guessing. The model decides *when* it has enough via **function/tool calling**,
not regex.

## 2. Rationale

Real speech is incremental and messy. The current one-shot parser (R-0032) either
succeeds on a fully-formed sentence or bounces the user with a canned clarify.
A context-keeping agent that asks targeted questions turns logging into a short
dialogue — higher completion rate, less friction, and it feels like a coach.

## 3. Acceptance criteria

- **AC1. Multi-turn context.** A request carries the prior conversation turns
  (user + assistant) so the model resolves a new utterance *against the running
  dialogue* (e.g. after "log a meal" → "chicken breast, 200 grams" completes the
  earlier intent). Turn history is bounded (a capped window).
- **AC2. Tool calling, not regex.** The LLM is given typed tools —
  `log_workout(exercise, reps, weight_kg?)` and `log_meal(protein_g, carbs_g,
  fat_g)` — and drives the outcome by choosing to call one (enough info) or to
  ask a question (missing info). The keyword parser remains the no-LLM fallback.
- **AC3. Asks for missing fields.** When required arguments are absent, the
  assistant returns a **specific** follow-up question naming what it needs, and
  does **not** write a log that turn.
- **AC4. Commits when complete.** Once the required fields are gathered (across
  one or more turns), the corresponding nutrition/workout row is written, and the
  response confirms what was logged (reusing R-0032's logged-* response shape).
- **AC5. Validation still enforced.** Tool arguments pass through the existing
  `fitai_core` `New*` constructors; invalid values (e.g. negative macros) produce
  a clarify/validation response, never a bad row.
- **AC6. Model-agnostic.** The feature works through the existing provider seam
  (R-0032 / #68): Anthropic, or any OpenAI-compatible endpoint — including
  **Cloudflare Workers AI** (e.g. a Qwen model) and local **Ollama** — selected
  by env, with no code change to swap models.
- **AC7. Graceful degradation.** LLM error/timeout/no-key falls back to the
  keyword parser (single-shot); the user is never left with a crash or a dead
  turn.
- **AC8. Scope guard / safety.** The assistant stays constrained to
  fitness/nutrition logging + navigation — it is not a general chatbot; it must
  not answer off-domain requests. No audio is stored; only transcripts + prior
  turns transit. The mic remains user-initiated (no always-on listening).
- **AC9. Tests.** Unit tests for: tool-call → `ParsedAction` mapping; a missing
  argument → clarify (no write); a completed multi-turn exchange → a write;
  validation rejection. Backend integration test for the endpoint carrying
  history. Provider seam covered by a fake LLM (no live model in CI).
- **AC10. Reminders out of scope.** Proactive missing-log reminders remain
  R-0036; this requirement is only the conversational logging loop.

## 4. Constraints & non-goals

- **Stateless server, client-carried history (v1).** Conversation state travels
  in the request (bounded turn list). A server-side session store — Durable
  Object / Agents SDK edge agent — is an explicit **non-goal for v1**, noted as a
  future migration if streaming/persistence is wanted.
- Not a general assistant; strictly logging + navigation intents.
- No new audio pipeline; on-device STT (R-0032) is unchanged.
- Does not change the mobile voice UI beyond passing/keeping turn history.

## 5. Open questions (deferred to SPEC-0037)

- **OQ-1:** Turn-window size and token budget (how many prior turns to send).
- **OQ-2:** Uniform tool-calling wire format across providers — do we use each
  API's native tool-calling, or a JSON-schema-in-prompt shim for
  OpenAI-compatible servers (Ollama/Workers AI) that vary in tool support?
- **OQ-3:** Default production model + a smaller/cheaper fallback tier
  (right-sizing neurons vs quality).
- **OQ-4:** Where the bounded history lives on the client (per voice session in
  the hub / coach) and how "out"/session-end clears it.
- **OQ-5:** Confirmation policy — auto-commit when confident vs always read back
  before writing.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Client-carried history, stateless server (v1) | Smallest step on the existing `/voice/intent`; defers Durable Object/Agents-SDK complexity until warranted. |
| 2026-07-09 | Tool calling over regex/JSON-mode | The model owns "enough info?" — follow-ups become emergent, not hand-coded. |
| 2026-07-09 | Reuse the #68 provider seam (model-agnostic) | Workers AI is OpenAI-compatible, so Qwen/DeepSeek/Ollama/Anthropic all work by env. |

## Changelog

- _2026-07-09 — created (Draft), pending owner sign-off of acceptance criteria._
