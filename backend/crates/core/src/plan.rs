//! R-0061 / SPEC-0061 §2.2 — the plan projection.
//!
//! A program is an ordered cycle of [`PlanDay`]s that starts on the day the
//! program was chosen. [`project`] replays the user's logs over the cycle and
//! emits one [`DayState`] per calendar day in the requested [`DateRange`]: the
//! logged past as *done*, and the days from today onward as the plan the user
//! is assumed to follow. Pure: no I/O, no clock. `today` is an argument.
//!
//! What is stored and what is derived (SPEC-0061 finding 3): a plan session
//! stores the cycle index it was seeded with — that is a fact — and the
//! *cursor* (which day is next) is derived here by replay, so a late log can
//! never rewrite a day that was actually trained.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::workout::MuscleGroup;

/// One day of a program's cycle (SPEC-0061 §2.1).
///
/// A training day names its `focus` with the closed [`MuscleGroup`]
/// vocabulary — the client builds the day's name from it in the user's
/// language, so the server sends no prose — and lists its exercises.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanDay {
    Training {
        focus: Vec<MuscleGroup>,
        exercises: Vec<String>,
    },
    Rest,
}

impl PlanDay {
    fn is_rest(&self) -> bool {
        matches!(self, PlanDay::Rest)
    }
}

/// The inclusive span of calendar days a projection is asked for.
///
/// Built only through [`DateRange::new`], so an inverted, over-long or
/// far-ahead range cannot exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateRange {
    from: NaiveDate,
    to: NaiveDate,
}

impl DateRange {
    /// The most calendar days a range may span, inclusive of both ends. Two
    /// consecutive months are at most 62 days.
    pub const MAX_DAYS: i64 = 62;

    /// How far past `today` a range may end. Bounding `to` bounds the work
    /// of any request, since the replay walks every day up to it (N9).
    pub const MAX_DAYS_AHEAD: u64 = 366;

    /// Validate a range.
    ///
    /// # Preconditions
    /// - `from <= to`;
    /// - the range spans at most [`Self::MAX_DAYS`] days, both ends counted;
    /// - `to` is no later than `today + ` [`Self::MAX_DAYS_AHEAD`] days.
    ///
    /// # Errors
    /// - [`PlanError::RangeInverted`] when `from > to`;
    /// - [`PlanError::RangeTooLong`] when the span exceeds `MAX_DAYS`;
    /// - [`PlanError::RangeTooFarAhead`] when `to` is past the reach.
    pub fn new(from: NaiveDate, to: NaiveDate, today: NaiveDate) -> Result<Self, PlanError> {
        if from > to {
            return Err(PlanError::RangeInverted);
        }
        if to.signed_duration_since(from).num_days() + 1 > Self::MAX_DAYS {
            return Err(PlanError::RangeTooLong);
        }
        // `checked_add_days` is `None` only at the end of the calendar, where
        // no `to` can lie beyond it.
        let reach = today.checked_add_days(Days::new(Self::MAX_DAYS_AHEAD));
        if reach.is_some_and(|limit| to > limit) {
            return Err(PlanError::RangeTooFarAhead);
        }
        Ok(Self { from, to })
    }

    fn contains(self, day: NaiveDate) -> bool {
        self.from <= day && day <= self.to
    }

    fn days(self) -> impl Iterator<Item = NaiveDate> {
        self.from.iter_days().take_while(move |d| *d <= self.to)
    }
}

/// What the user logged on one day.
///
/// `plan` holds the `plan_day_index` of each plan session of the **current**
/// program that day, in `created_at` order; sessions of an earlier program
/// are free workouts here (SPEC-0061 finding 11). `any_session` is `true`
/// when any workout at all was logged, plan or free — so it is `true`
/// whenever `plan` is non-empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DayLog {
    pub plan: Vec<u32>,
    pub any_session: bool,
}

/// What the calendar shows for one day. Indices point into the cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DayState {
    /// A workout was logged. `completed` holds the cycle indices completed
    /// that day (empty: free workout(s) only); `pending` is the training day
    /// still pending when only free workouts were logged (N1).
    Done {
        completed: Vec<u32>,
        pending: Option<u32>,
    },
    /// A training day projected onto today or later.
    Planned { cycle_index: u32 },
    /// A rest day projected onto today or later.
    Rest { cycle_index: u32 },
    /// A past day with nothing logged. `pending` is the training day a late
    /// log would complete, if any.
    Nothing { pending: Option<u32> },
}

/// Why a projection could not be computed — never a panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum PlanError {
    /// The cycle has no `Training` day. Unreachable after §2.1 validation,
    /// kept so [`project`] is total.
    #[error("the cycle has no training day")]
    NoTrainingDay,
    /// `to` is earlier than `from`.
    #[error("the range ends before it starts")]
    RangeInverted,
    /// More than [`DateRange::MAX_DAYS`] days, counted inclusively.
    #[error("the range spans more than {} days", DateRange::MAX_DAYS)]
    RangeTooLong,
    /// `to` is more than [`DateRange::MAX_DAYS_AHEAD`] days after today.
    #[error(
        "the range ends more than {} days after today",
        DateRange::MAX_DAYS_AHEAD
    )]
    RangeTooFarAhead,
}

/// Project the plan onto every day of `range` (SPEC-0061 §2.2).
///
/// The cursor starts at index 0 on `starts_on` and walks every day from
/// `starts_on` to the end of `range`, and no further, so nothing after the
/// range moves it. For each day, in this order:
///
/// 1. days before `starts_on` never touch the cursor: `Done` with nothing
///    completed when any session was logged, else `Nothing` with nothing
///    pending;
/// 2. a day with plan sessions, whatever its tense, is `Done` with those
///    indices and the cursor moves past the last one;
/// 3. a day with only free sessions, whatever its tense, is `Done` with
///    nothing completed: a rest day is consumed by time, a training day
///    stays pending and is reported as such (N1);
/// 4. a past day with nothing logged: before the latest plan log it offers
///    nothing, since a later plan session already completed what was pending
///    (N2); otherwise a training day **shifts** and is offered as pending,
///    and a rest day is consumed;
/// 5. today or later with nothing logged is `Planned` or `Rest`, and the
///    cursor moves: the user is assumed to follow the plan.
///
/// Only dates inside `range` are emitted, in ascending order, one per day.
/// A stored index past the end of the cycle (a shortened cycle) is read
/// modulo its length. Cost is linear in the days from `starts_on` to the end
/// of `range`.
///
/// # Errors
/// [`PlanError::NoTrainingDay`] when `cycle` has no `Training` day. Range
/// errors are raised by [`DateRange::new`], so none arise here.
pub fn project(
    cycle: &[PlanDay],
    starts_on: NaiveDate,
    logs: &BTreeMap<NaiveDate, DayLog>,
    today: NaiveDate,
    range: DateRange,
) -> Result<Vec<(NaiveDate, DayState)>, PlanError> {
    let mut cursor = Cursor::new(cycle)?;
    let mut out = Vec::new();

    for day in range.days().take_while(|d| *d < starts_on) {
        let any = logs.get(&day).is_some_and(|l| l.any_session);
        out.push((day, if any { done_free(None) } else { nothing(None) }));
    }

    let last_plan = logs
        .iter()
        .rev()
        .find(|(_, l)| !l.plan.is_empty())
        .map(|(d, _)| *d);

    for day in starts_on.iter_days().take_while(|d| *d <= range.to) {
        let state = match logs.get(&day) {
            Some(l) if !l.plan.is_empty() => {
                cursor.complete(&l.plan);
                DayState::Done {
                    // Normalised like the cursor reads them, so a stored
                    // index from a since-shortened cycle still names a day
                    // the client can look up (the cycle is not frozen per
                    // user, §4).
                    completed: l.plan.iter().map(|i| i % cursor.len).collect(),
                    pending: None,
                }
            }
            Some(l) if l.any_session => done_free(cursor.pass_free()),
            _ if day < today => {
                let before_last_plan = last_plan.is_some_and(|lp| day < lp);
                cursor.pass_unlogged(before_last_plan)
            }
            _ => cursor.project_one(),
        };
        if range.contains(day) {
            out.push((day, state));
        }
    }
    Ok(out)
}

fn done_free(pending: Option<u32>) -> DayState {
    DayState::Done {
        completed: Vec::new(),
        pending,
    }
}

fn nothing(pending: Option<u32>) -> DayState {
    DayState::Nothing { pending }
}

/// The replay's position in the cycle; wraps modulo its length.
struct Cursor<'a> {
    cycle: &'a [PlanDay],
    len: u32,
    at: u32,
}

impl<'a> Cursor<'a> {
    fn new(cycle: &'a [PlanDay]) -> Result<Self, PlanError> {
        if cycle.iter().all(PlanDay::is_rest) {
            return Err(PlanError::NoTrainingDay);
        }
        // §2.1 bounds a cycle at 14 days; the clamp only keeps the
        // conversion total.
        let len = u32::try_from(cycle.len()).unwrap_or(u32::MAX);
        Ok(Self { cycle, len, at: 0 })
    }

    fn on_rest(&self) -> bool {
        self.cycle
            .get(self.at as usize)
            .is_some_and(PlanDay::is_rest)
    }

    fn advance(&mut self) {
        self.at = (self.at + 1) % self.len;
    }

    /// Rule 2: the cursor moves to just after the last completed index.
    fn complete(&mut self, plan: &[u32]) {
        if let Some(last) = plan.last() {
            self.at = (last % self.len + 1) % self.len;
        }
    }

    /// Rule 3: a rest day is consumed; a training day stays pending.
    fn pass_free(&mut self) -> Option<u32> {
        if self.on_rest() {
            self.advance();
            None
        } else {
            Some(self.at)
        }
    }

    /// Rule 4: a training day shifts, a rest day is consumed. Before the
    /// latest plan log nothing is offered, but the cursor moves the same way
    /// so every later day replays identically (N2).
    fn pass_unlogged(&mut self, before_last_plan: bool) -> DayState {
        let pending = self.pass_free();
        nothing(if before_last_plan { None } else { pending })
    }

    /// Rule 5: the day is projected and the cursor moves.
    fn project_one(&mut self) -> DayState {
        let cycle_index = self.at;
        let state = if self.on_rest() {
            DayState::Rest { cycle_index }
        } else {
            DayState::Planned { cycle_index }
        };
        self.advance();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::technique::Lift;
    use DayState::{Nothing, Planned, Rest};

    /// A day of October 2026. The 19th is a Monday.
    fn oct(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn training(group: MuscleGroup) -> PlanDay {
        PlanDay::training(vec![group], vec!["Some lift".into()])
    }

    /// Legs, Push, Pull, Rest.
    fn four_day_cycle() -> Vec<PlanDay> {
        vec![
            training(MuscleGroup::Legs),
            training(MuscleGroup::Chest),
            training(MuscleGroup::Back),
            PlanDay::Rest,
        ]
    }

    fn plan(indices: &[u32]) -> DayLog {
        DayLog {
            plan: indices.to_vec(),
            any_session: true,
        }
    }

    fn free() -> DayLog {
        DayLog {
            plan: vec![],
            any_session: true,
        }
    }

    fn done(completed: &[u32], pending: Option<u32>) -> DayState {
        DayState::Done {
            completed: completed.to_vec(),
            pending,
        }
    }

    fn logs(entries: &[(NaiveDate, DayLog)]) -> BTreeMap<NaiveDate, DayLog> {
        entries.iter().cloned().collect()
    }

    /// Project `cycle` over `from..=to` and return the states in date order.
    fn states(
        cycle: &[PlanDay],
        starts_on: NaiveDate,
        logs: &BTreeMap<NaiveDate, DayLog>,
        today: NaiveDate,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Vec<DayState> {
        let range = DateRange::new(from, to, today).unwrap();
        let out = project(cycle, starts_on, logs, today, range).unwrap();
        let dates: Vec<NaiveDate> = out.iter().map(|(d, _)| *d).collect();
        let expected: Vec<NaiveDate> = range.days().collect();
        assert_eq!(dates, expected, "one entry per day of the range, in order");
        out.into_iter().map(|(_, s)| s).collect()
    }

    // --- rule 1 ----------------------------------------------------------

    #[test]
    fn days_before_starts_on_leave_the_cursor_alone() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), free())]);
        let s = states(&cycle, oct(21), &logs, oct(21), oct(19), oct(23));
        assert_eq!(
            s,
            vec![
                done(&[], None),
                Nothing { pending: None },
                Planned { cycle_index: 0 },
                Planned { cycle_index: 1 },
                Planned { cycle_index: 2 },
            ]
        );
    }

    #[test]
    fn a_future_starts_on_projects_from_that_day() {
        let cycle = four_day_cycle();
        let s = states(&cycle, oct(21), &BTreeMap::new(), oct(19), oct(19), oct(23));
        assert_eq!(
            s,
            vec![
                Nothing { pending: None },
                Nothing { pending: None },
                Planned { cycle_index: 0 },
                Planned { cycle_index: 1 },
                Planned { cycle_index: 2 },
            ]
        );
    }

    #[test]
    fn a_range_wholly_before_starts_on_shows_logs_only() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(20), free())]);
        let s = states(&cycle, oct(26), &logs, oct(23), oct(19), oct(23));
        assert_eq!(
            s,
            vec![
                Nothing { pending: None },
                done(&[], None),
                Nothing { pending: None },
                Nothing { pending: None },
                Nothing { pending: None },
            ]
        );
    }

    // --- rule 2 ----------------------------------------------------------

    #[test]
    fn a_plan_session_completes_its_day_and_moves_the_cursor_past_it() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), plan(&[0])), (oct(20), plan(&[1]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(23));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                done(&[1], None),
                Planned { cycle_index: 2 },
                Rest { cycle_index: 3 },
                Planned { cycle_index: 0 },
            ]
        );
    }

    #[test]
    fn a_plan_session_today_resumes_tomorrow_at_the_next_day_even_a_rest() {
        let cycle = vec![training(MuscleGroup::Legs), PlanDay::Rest];
        let logs = logs(&[(oct(19), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                Rest { cycle_index: 1 },
                Planned { cycle_index: 0 },
            ]
        );
    }

    #[test]
    fn two_plan_sessions_on_one_day_complete_both_and_the_cursor_follows_the_last() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), plan(&[0, 1]))]);
        let s = states(&cycle, oct(19), &logs, oct(20), oct(19), oct(20));
        assert_eq!(s, vec![done(&[0, 1], None), Planned { cycle_index: 2 }]);
    }

    #[test]
    fn a_plan_session_on_a_projected_rest_day_skips_the_rest() {
        let cycle = vec![
            training(MuscleGroup::Legs),
            PlanDay::Rest,
            training(MuscleGroup::Chest),
        ];
        let logs = logs(&[(oct(19), plan(&[0])), (oct(20), plan(&[2]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                done(&[2], None),
                Planned { cycle_index: 0 }
            ]
        );
    }

    #[test]
    fn a_late_log_keeps_both_stored_indices_and_rewrites_neither() {
        // Finding 3: legs was done on Tuesday; Monday is logged late as legs
        // too. Tuesday must not become Push.
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), plan(&[0])), (oct(20), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                done(&[0], None),
                Planned { cycle_index: 1 }
            ]
        );
    }

    #[test]
    fn a_future_dated_plan_log_shows_as_done() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(21), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(22));
        assert_eq!(
            s,
            vec![
                Planned { cycle_index: 0 },
                Planned { cycle_index: 1 },
                done(&[0], None),
                Planned { cycle_index: 1 },
            ]
        );
    }

    // --- rule 3 ----------------------------------------------------------

    #[test]
    fn a_free_workout_on_a_training_day_leaves_it_pending() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), free())]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(20));
        assert_eq!(s, vec![done(&[], Some(0)), Planned { cycle_index: 0 }]);
    }

    #[test]
    fn a_free_workout_consumes_a_rest_day() {
        let cycle = vec![training(MuscleGroup::Legs), PlanDay::Rest];
        let logs = logs(&[(oct(19), plan(&[0])), (oct(20), free())]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                done(&[], None),
                Planned { cycle_index: 0 }
            ]
        );
    }

    #[test]
    fn a_future_dated_free_workout_shows_as_done_with_its_pending_day() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(20), free())]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                Planned { cycle_index: 0 },
                done(&[], Some(1)),
                Planned { cycle_index: 1 },
            ]
        );
    }

    #[test]
    fn after_a_program_change_old_plan_sessions_count_as_free() {
        // Finding 11: the caller passes an earlier program's sessions with an
        // empty `plan`, so they show on the calendar without advancing.
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), free()), (oct(21), free())]);
        let s = states(&cycle, oct(21), &logs, oct(22), oct(19), oct(22));
        assert_eq!(
            s,
            vec![
                done(&[], None),
                Nothing { pending: None },
                done(&[], Some(0)),
                Planned { cycle_index: 0 },
            ]
        );
    }

    // --- rule 4 ----------------------------------------------------------

    #[test]
    fn an_unlogged_training_day_shifts_to_the_next_day() {
        let cycle = four_day_cycle();
        let s = states(&cycle, oct(19), &BTreeMap::new(), oct(20), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                Nothing { pending: Some(0) },
                Planned { cycle_index: 0 },
                Planned { cycle_index: 1 },
            ]
        );
    }

    #[test]
    fn an_unlogged_rest_day_is_consumed_by_time() {
        let cycle = vec![
            training(MuscleGroup::Legs),
            PlanDay::Rest,
            training(MuscleGroup::Chest),
        ];
        let logs = logs(&[(oct(19), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                Nothing { pending: None },
                Planned { cycle_index: 2 },
            ]
        );
    }

    #[test]
    fn an_unlogged_day_before_a_later_plan_log_offers_no_pending_day() {
        // N2: [Legs, Push]; Monday unlogged, Tuesday logged as Legs. Monday
        // must not offer Legs again.
        let cycle = vec![training(MuscleGroup::Legs), training(MuscleGroup::Chest)];
        let logs = logs(&[(oct(20), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(21));
        assert_eq!(
            s,
            vec![
                Nothing { pending: None },
                done(&[0], None),
                Planned { cycle_index: 1 },
            ]
        );
    }

    #[test]
    fn before_the_last_plan_log_the_cursor_still_consumes_rest_days() {
        // Rest first: Monday consumes it, Tuesday's training day stays and
        // is not offered, Wednesday's log sets the cursor, Thursday rests.
        let cycle = vec![PlanDay::Rest, training(MuscleGroup::Legs)];
        let logs = logs(&[(oct(21), plan(&[1]))]);
        let s = states(&cycle, oct(19), &logs, oct(22), oct(19), oct(22));
        assert_eq!(
            s,
            vec![
                Nothing { pending: None },
                Nothing { pending: None },
                done(&[1], None),
                Rest { cycle_index: 0 },
            ]
        );
    }

    // --- rule 5 ----------------------------------------------------------

    #[test]
    fn days_from_today_on_follow_the_cycle_in_order() {
        let cycle = four_day_cycle();
        let s = states(&cycle, oct(19), &BTreeMap::new(), oct(19), oct(19), oct(23));
        assert_eq!(
            s,
            vec![
                Planned { cycle_index: 0 },
                Planned { cycle_index: 1 },
                Planned { cycle_index: 2 },
                Rest { cycle_index: 3 },
                Planned { cycle_index: 0 },
            ]
        );
    }

    #[test]
    fn a_one_training_day_cycle_projects_it_every_day_and_shifts_the_same_way() {
        let cycle = vec![training(MuscleGroup::Legs)];
        let logs = logs(&[(oct(19), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(21), oct(19), oct(22));
        assert_eq!(
            s,
            vec![
                done(&[0], None),
                Nothing { pending: Some(0) },
                Planned { cycle_index: 0 },
                Planned { cycle_index: 0 },
            ]
        );
    }

    // --- the walk and the range -----------------------------------------

    #[test]
    fn only_dates_inside_the_range_are_emitted_but_the_walk_starts_at_starts_on() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(22), oct(21), oct(22));
        assert_eq!(
            s,
            vec![Nothing { pending: Some(1) }, Planned { cycle_index: 1 }]
        );
    }

    #[test]
    fn a_log_after_the_range_does_not_move_the_cursor_inside_it() {
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(23), plan(&[2]))]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(20));
        assert_eq!(
            s,
            vec![Planned { cycle_index: 0 }, Planned { cycle_index: 1 }]
        );
    }

    #[test]
    fn a_plan_log_after_the_range_still_clears_pending_on_earlier_days() {
        // N2 over N9: the walk stops at range.to, but a later completion is
        // a fact about the past, so unlogged days before it offer nothing.
        let cycle = vec![training(MuscleGroup::Legs), training(MuscleGroup::Chest)];
        let logs = logs(&[(oct(22), plan(&[0]))]);
        let s = states(&cycle, oct(19), &logs, oct(23), oct(19), oct(20));
        assert_eq!(
            s,
            vec![Nothing { pending: None }, Nothing { pending: None }]
        );
    }

    #[test]
    fn days_before_starts_on_do_not_consume_a_leading_rest_day() {
        // A rest-first cycle: if rules 3/4 ran before starts_on, the rest at
        // index 0 would be consumed and starts_on would show index 1.
        let cycle = vec![PlanDay::Rest, training(MuscleGroup::Legs)];
        let logs = logs(&[(oct(19), free()), (oct(20), free())]);
        let s = states(&cycle, oct(21), &logs, oct(21), oct(19), oct(22));
        assert_eq!(
            s,
            vec![
                done(&[], None),
                done(&[], None),
                DayState::Rest { cycle_index: 0 },
                Planned { cycle_index: 1 },
            ]
        );
    }

    #[test]
    fn a_stored_index_past_the_cycle_length_is_read_modulo_it() {
        // The cycle was shortened after the session was stored (§4: the
        // cycle is not frozen per user). Index 5 in a 4-day cycle is day 1.
        let cycle = four_day_cycle();
        let logs = logs(&[(oct(19), plan(&[5]))]);
        let s = states(&cycle, oct(19), &logs, oct(19), oct(19), oct(20));
        assert_eq!(s[0], done(&[1], None));
        assert_eq!(s[1], Planned { cycle_index: 2 });
    }

    // --- errors ----------------------------------------------------------

    #[test]
    fn an_inverted_range_is_rejected() {
        assert_eq!(
            DateRange::new(oct(20), oct(19), oct(19)),
            Err(PlanError::RangeInverted)
        );
    }

    #[test]
    fn a_range_longer_than_62_days_is_rejected() {
        let from = oct(1);
        let today = oct(1);
        assert!(DateRange::new(from, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(), today).is_ok());
        assert_eq!(
            DateRange::new(from, NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(), today),
            Err(PlanError::RangeTooLong)
        );
    }

    #[test]
    fn a_range_ending_more_than_366_days_ahead_is_rejected() {
        let today = oct(19);
        let limit = NaiveDate::from_ymd_opt(2027, 10, 20).unwrap();
        assert!(DateRange::new(limit, limit, today).is_ok());
        assert_eq!(
            DateRange::new(limit, limit.succ_opt().unwrap(), today),
            Err(PlanError::RangeTooFarAhead)
        );
    }

    #[test]
    fn a_cycle_without_a_training_day_is_rejected() {
        let range = DateRange::new(oct(19), oct(19), oct(19)).unwrap();
        for cycle in [vec![], vec![PlanDay::Rest, PlanDay::Rest]] {
            assert_eq!(
                project(&cycle, oct(19), &BTreeMap::new(), oct(19), range),
                Err(PlanError::NoTrainingDay)
            );
        }
    }

    // --- physics (SPEC-0059 §2.5) ---------------------------------------

    #[test]
    fn the_library_back_squat_has_the_squat_model() {
        for name in ["Back Squat", "back squat", "  BACK SQUAT "] {
            assert_eq!(physics_for(name), Some(Lift::Squat), "{name:?}");
        }
    }

    #[test]
    fn exercises_without_a_model_have_no_physics() {
        // Other squats are other mechanics (the model is the back squat);
        // the deadlift and bench wait for R-0045's garust work.
        for name in [
            "Front Squat",
            "Hack Squat",
            "Low Bar Squat",
            "Deadlift",
            "Romanian Deadlift",
            "Bench Press",
            "Close-Grip Bench Press",
            "Leg Press",
            "",
        ] {
            assert_eq!(physics_for(name), None, "{name:?}");
        }
    }

    #[test]
    fn a_training_day_has_one_physics_entry_per_exercise_in_order() {
        let day = PlanDay::training(
            vec![MuscleGroup::Legs],
            vec!["Leg Press".into(), "Back Squat".into()],
        );
        let PlanDay::Training { physics, .. } = day else {
            panic!("a training day");
        };
        assert_eq!(physics, vec![None, Some(Lift::Squat)]);
    }

    // --- wire shape (§2.4; slice 2c serializes these) ------------------

    #[test]
    fn plan_day_serializes_with_a_kind_tag() {
        let day = PlanDay::training(
            vec![MuscleGroup::Legs],
            vec!["Back Squat".into(), "Leg Curl".into()],
        );
        assert_eq!(
            serde_json::to_value(&day).unwrap(),
            serde_json::json!({
                "kind": "training",
                "focus": ["legs"],
                "exercises": ["Back Squat", "Leg Curl"],
                "physics": ["squat", null],
            })
        );
        assert_eq!(
            serde_json::to_value(PlanDay::Rest).unwrap(),
            serde_json::json!({ "kind": "rest" })
        );
    }

    #[test]
    fn day_state_serializes_with_a_state_tag() {
        assert_eq!(
            serde_json::to_value(done(&[], Some(0))).unwrap(),
            serde_json::json!({ "state": "done", "completed": [], "pending": 0 })
        );
        assert_eq!(
            serde_json::to_value(Nothing { pending: None }).unwrap(),
            serde_json::json!({ "state": "nothing", "pending": null })
        );
        assert_eq!(
            serde_json::to_value(Planned { cycle_index: 0 }).unwrap(),
            serde_json::json!({ "state": "planned", "cycle_index": 0 })
        );
        assert_eq!(
            serde_json::to_value(Rest { cycle_index: 3 }).unwrap(),
            serde_json::json!({ "state": "rest", "cycle_index": 3 })
        );
    }
}
