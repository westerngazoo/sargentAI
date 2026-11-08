# R-0065 — Your squat against the model, from your own video

- **Status:** Draft
- **Milestone:** M6 (the lift model, shown to the lifter)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-11-08
- **Depends on:** R-0045 (the squat solve and its moments, slices A and B1,
  merged), R-0059 (the squat check this opens from, Met). R-0064 (the
  sticking point) only to add capacity later; it is not a blocker. R-0062
  (deadlift) and R-0063 (bench press) only to extend this to those lifts.
- **Realized by:** SPEC-0065 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A lifter films their own squat from the side with the phone. On a few
**key frames** they choose (the top, halfway down, the bottom, halfway up,
the top again) they mark where their joints and the bar are. The video
never leaves the phone: only those marked points travel to the server.

The server fits the marked points to the lifter's own segment lengths,
fills the motion between the key frames, and runs the R-0045 model on every
frame. It also finds, for the same lifter, load and depth, the side-view
posture that asks the least of each joint. The app plays the lifter's
video with the model drawn over it, and the lifter can move through it like
a small game: where the demand peaks, how far it is from the model's
minimum, and which change (trunk lean, how far the knee travels, where the
bar sits over the foot) closes the gap, in degrees and centimetres.

## 2. Rationale

- **The check shows the model's squat; the lifter wants their own.** R-0059
  draws the squat the model computes from the lifter's measurements. The
  next question is "and mine?" (owner, 2026-11-02). Marking key frames over
  their own video puts the lifter's real posture into the same physics.
- **Marking beats guessing.** Automatic pose estimation on a phone video is
  noisy in exactly the joints that matter (hip, bar), and R-0044's server
  path needs the video uploaded. A lifter marking five frames carefully is
  slower but checkable, and it keeps the video on the phone (owner,
  2026-11-08).
- **"How efficient is my squat" needs a physical answer.** A score of good
  or bad would break R-0059 AC6 and the product's promise. Comparing the
  lifter's peak moments against the minimum the same model finds for the
  same body, load and depth is a number anyone can check (owner,
  2026-11-08).

## 3. Acceptance criteria

### Marking, on the phone

- **AC1. Entry.** The physics check of a modelled exercise (R-0061 AC9,
  R-0059) offers *Compárate con tu video*. An exercise that reads *Pronto*
  does not offer it.
- **AC2. The video stays on the phone.** The lifter records or picks a video
  on the phone. It is never uploaded, never sent in any request and never
  stored by the server. A test asserts the request body carries no video
  bytes and no file reference.
- **AC3. Key frames.** The app asks for the side view and guides the lifter
  to mark the key frames in order (top, halfway down, bottom, halfway up,
  top; see OQ-1). On each one the lifter places the ankle, knee, hip,
  shoulder and bar, and can move any point before sending. The app shows
  which key frames are still missing and does not send until all are marked.
- **AC4. What is sent.** Per key frame: its time in the video and the marked
  points in the frame's own coordinates. Also the R-0059 inputs: height,
  body mass, optional segment lengths, dorsiflexion and the bar load the
  lifter enters.

### The model, on the server

- **AC5. Fit to the lifter.** The marked points are fitted to the lifter's
  segment lengths (measured, or from the table as in R-0059). The result
  states, per key frame, how far the marked points sit from the fitted
  posture.
- **AC6. Refuse rather than guess** (as R-0044 AC12). When the marks cannot
  be a side-view squat of this lifter, the answer is a typed refusal that
  names the key frame and the reason, never a figure drawn anyway. Examples:
  segments that are too long or too short for the lifter beyond a stated
  tolerance, or key frames out of order.
- **AC7. The frames in between.** The server fills the motion between key
  frames by a stated method. Every frame says whether it was marked or
  filled. Each frame carries the R-0045 moments with their uncertainty, and
  the balance over the mid-foot.
- **AC8. The model's minimum.** For the same lifter, load and depth, the
  server finds the side-view posture through the rep that asks the least of
  each joint, varying only what a side view shows: trunk lean, knee travel
  and the bar's place over the foot, always in balance. The result reports:
  - per joint, the lifter's peak moment, the minimum's peak and the
    difference in N·m and %;
  - where in the rep the gap is largest;
  - the change the minimum used there, in degrees and centimetres.

### The screen

- **AC9. The video with the model on it.** The app plays the lifter's video
  with the fitted figure drawn over it. Playback, pause and scrubbing work
  as in R-0059. The per-frame readouts and moment bars are R-0059's.
- **AC10. The game.** The lifter can show the model's minimum posture next
  to their own at any frame, jump to the point of the largest gap, and see
  the suggested change there. Every number on the screen comes from the
  server's answer.
- **AC11. The caveats, on the screen:**
  - R-0059's: assumptions, table-derived lengths, the arms;
  - that only the marked frames were measured, and the frames in between
    are filled;
  - how far each marked frame sat from the fit (AC5);
  - that a side view sees only the side plane, so stance width and foot
    angle are not measured.
- **AC12. No injury or safety claims** (R-0059 AC6, R-0045 AC14). A gap is a
  number of N·m, never good or bad, safe or dangerous.

### Boundaries

- **AC13. Thin client.** The app computes no posture, fit or moment. It may
  interpolate between returned frames for smooth playback, display only.
- **AC14. Privacy.** The marked points and body measurements are processed,
  not stored, and never logged. R-0024's inventory records them as
  "processed, not stored". The video is not in the inventory because it
  never reaches the server.
- **AC15. Tests.**
  - Backend: a valid marking; each invalid field; no session (401); each
    refusal of AC6; a marking that matches the model's minimum, which shows
    no gap; a known posture whose moments match R-0045 at the marked frames.
  - Flutter: marking and correcting points, sending blocked until every key
    frame is marked, the request carries no video, the overlay over
    playback, the minimum toggle, the jump to the largest gap, and a
    refusal's message.
  - The e2e test is owned by the `qa` agent.

## 4. Constraints & non-goals

- **The high-bar squat first.** The request names the lift, so deadlift and
  bench join once R-0062 and R-0063 give them a model, each with its own key
  frames and points.
- **The side plane only.** Stance width, foot angle and knee path in the
  frontal plane need a front view and a 3D model. They are a later
  requirement, not part of this one.
- **No automatic pose estimation.** The lifter marks the points. R-0044 is the
  automatic, uploaded path; this requirement does not use it.
- **Demand only.** Where the lifter is closest to their muscles' limit is
  R-0064. Once R-0064 lands, its sticking point can be drawn on this screen.
- **Nothing saved in v1** (see OQ-2).
- **Any new mobile dependency** (video playback, frame grabbing) is named in
  SPEC-0065 together with the alternative it rejected.

## 5. Open questions

- **OQ-1. How many key frames.** Five fixed frames (top, halfway down,
  bottom, halfway up, top) or the lifter's choice within a range (3 to 9)?
  Proposal: five fixed frames in v1. They match the phases of the rep and
  keep the fit comparable between lifters.
- **OQ-2. Saving.** Save the marked points (never the video) so the lifter
  can compare squats over time? Proposal: not in v1. A later slice once
  R-0024's privacy inventory covers it.
- **OQ-3. R-0044 and the video on the phone.** R-0044 (Accepted) uploads the
  clip for server-side pose estimation. With the owner's 2026-11-08 call
  that the video stays on the phone, does R-0044's upload path still stand,
  or does it move to on-device extraction, or is it closed? This does not
  block R-0065.
- **OQ-4. One number for the game?** AC8 reports per joint. Should the screen
  also show one summary (for example the largest gap of any joint, in %)?
  Proposal: per joint only in v1, so no joint's trade-off is hidden behind
  another.
- **OQ-5. Is the minimum well defined?** Whether R-0045's model has a unique
  least-demand posture over trunk lean, knee travel and bar position, and
  how the joints are weighed when lowering one raises another, is for
  SPEC-0065 to show with the architect. If it is not unique, AC8 is amended
  before acceptance.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-02 | A separate section to see the model against the lifter's own video; the joints the lifter marks adjust the preloaded model. | Owner. The log lists exercises by name only (59-c2c dropped); the comparison is its own place. |
| 2026-11-08 | Key frames marked by the lifter, the motion between them filled by the server; the result is a small game showing how efficient the squat is and where to change an angle. | Owner. |
| 2026-11-08 | The side plane only in v1; stance width is a later requirement. | A side view cannot measure width, and the model is sagittal (SPEC-0059 `sagittal_plane`). Owner agreed. |
| 2026-11-08 | "Efficient" means the lifter's peak moments against the model's minimum for the same body, load and depth; never good or bad. | R-0059 AC6; a checkable number. Owner agreed. |
| 2026-11-08 | The video stays on the phone; only the marked points travel. | Owner. Health data: no video stored, nothing to hold before M8's privacy review. |
| 2026-11-08 | The entry is *Compárate con tu video* in each exercise's physics check. | Owner agreed. |
| 2026-11-08 | Squat first; the other basic lifts join through R-0062 and R-0063. | Owner: "necesitamos meter más ejercicios básicos". Each lift needs its own model first. |

## Changelog

- 2026-11-08 — created (Draft).
