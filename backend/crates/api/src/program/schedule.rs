//! Pure helpers behind `GET /programs/me/schedule` and the plan link on
//! workouts (R-0061, SPEC-0061 §2.4): turning session rows into the
//! [`DayLog`]s the projection consumes, the logs-only calendar of a user with
//! no plan, and the two date checks. No I/O, no clock — testable without a
//! database.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use uuid::Uuid;

use fitai_core::plan::{DayLog, DayState, PlanDay};

use crate::{
    db::SessionPlanRow,
    error::{ApiError, ApiResult},
};

/// A client's local date must lie within a day of the server's UTC date:
/// every real local date does. Returns the date or a 422 carrying `reason`.
///
/// # Errors
/// [`ApiError::Unprocessable`] with `reason` when `date` is two or more days
/// from `server_today`.
pub(crate) fn check_local_date(
    date: NaiveDate,
    server_today: NaiveDate,
    reason: &'static str,
) -> ApiResult<NaiveDate> {
    if date.signed_duration_since(server_today).num_days().abs() <= 1 {
        Ok(date)
    } else {
        Err(ApiError::Unprocessable { reason })
    }
}

/// Whether `index` names a `Training` day of `cycle`. An index past the end
/// names nothing.
pub(crate) fn is_training_day(cycle: &[PlanDay], index: u32) -> bool {
    usize::try_from(index)
        .ok()
        .and_then(|i| cycle.get(i))
        .is_some_and(|day| matches!(day, PlanDay::Training { .. }))
}

/// Group the caller's sessions by day: the [`DayLog`] each day feeds the
/// projection, and the ids logged that day for the response's `session_ids`.
///
/// `plan` holds the indices of sessions linked to `current` (the current
/// program), in the rows' order, which is `created_at` order within a day;
/// sessions of any other program are free workouts here (finding 11).
/// `any_session` is set for every day with a row.
pub(crate) fn day_logs(
    rows: &[SessionPlanRow],
    current: Option<Uuid>,
) -> (BTreeMap<NaiveDate, DayLog>, BTreeMap<NaiveDate, Vec<Uuid>>) {
    let mut logs: BTreeMap<NaiveDate, DayLog> = BTreeMap::new();
    let mut ids: BTreeMap<NaiveDate, Vec<Uuid>> = BTreeMap::new();
    for row in rows {
        let log = logs.entry(row.performed_on).or_default();
        log.any_session = true;
        if current.is_some_and(|c| row.user_program_id == Some(c)) {
            // A negative stored index cannot pass the handler's validation;
            // if one ever appears, the session counts as free rather than
            // breaking the whole calendar.
            if let Some(index) = row.plan_day_index.and_then(|i| u32::try_from(i).ok()) {
                log.plan.push(index);
            }
        }
        ids.entry(row.performed_on).or_default().push(row.id);
    }
    (logs, ids)
}

/// The calendar of a user with no plan: every day of `from..=to` is `Done`
/// with nothing completed when a session was logged, else `Nothing` with
/// nothing pending — rule 1 of §2.2 applied to every day.
pub(crate) fn logs_only(
    from: NaiveDate,
    to: NaiveDate,
    logs: &BTreeMap<NaiveDate, DayLog>,
) -> Vec<(NaiveDate, DayState)> {
    from.iter_days()
        .take_while(|d| *d <= to)
        .map(|day| {
            let state = if logs.get(&day).is_some_and(|l| l.any_session) {
                DayState::Done {
                    completed: Vec::new(),
                    pending: None,
                }
            } else {
                DayState::Nothing { pending: None }
            };
            (day, state)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use fitai_core::plan::PlanExercise;
    use fitai_core::MuscleGroup;

    fn oct(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn row(day: u32, program: Option<Uuid>, index: Option<i16>) -> SessionPlanRow {
        SessionPlanRow {
            id: Uuid::new_v4(),
            performed_on: oct(day),
            user_program_id: program,
            plan_day_index: index,
        }
    }

    #[test]
    fn a_local_date_within_a_day_of_the_server_passes() {
        for day in [19, 20, 21] {
            assert_eq!(check_local_date(oct(day), oct(20), "x").unwrap(), oct(day));
        }
    }

    #[test]
    fn a_local_date_two_days_from_the_server_is_unprocessable_with_the_reason() {
        for day in [18, 22] {
            let err = check_local_date(oct(day), oct(20), "today_out_of_bounds").unwrap_err();
            assert!(matches!(
                err,
                ApiError::Unprocessable {
                    reason: "today_out_of_bounds"
                }
            ));
        }
    }

    #[test]
    fn only_a_training_index_inside_the_cycle_is_a_training_day() {
        let cycle = vec![
            PlanDay::Training {
                focus: vec![MuscleGroup::Legs],
                exercises: vec![PlanExercise::new("Back Squat")],
            },
            PlanDay::Rest,
        ];
        assert!(is_training_day(&cycle, 0));
        assert!(!is_training_day(&cycle, 1), "a rest day");
        assert!(!is_training_day(&cycle, 2), "past the end");
    }

    #[test]
    fn plan_indices_count_only_for_the_current_program_in_row_order() {
        let current = Uuid::new_v4();
        let old = Uuid::new_v4();
        let rows = vec![
            row(19, Some(current), Some(3)),
            row(19, Some(current), Some(0)),
            row(20, Some(old), Some(1)),
            row(21, None, None),
        ];
        let (logs, ids) = day_logs(&rows, Some(current));

        assert_eq!(
            logs[&oct(19)],
            DayLog {
                plan: vec![3, 0],
                any_session: true
            }
        );
        assert_eq!(
            logs[&oct(20)],
            DayLog {
                plan: vec![],
                any_session: true
            },
            "an earlier program's session is a free workout"
        );
        assert_eq!(
            logs[&oct(21)],
            DayLog {
                plan: vec![],
                any_session: true
            }
        );
        assert_eq!(ids[&oct(19)], vec![rows[0].id, rows[1].id]);
        assert_eq!(ids[&oct(20)], vec![rows[2].id]);
    }

    #[test]
    fn without_a_current_program_every_session_is_free() {
        let rows = vec![row(19, Some(Uuid::new_v4()), Some(0))];
        let (logs, _) = day_logs(&rows, None);
        assert!(logs[&oct(19)].plan.is_empty());
        assert!(logs[&oct(19)].any_session);
    }

    #[test]
    fn a_negative_stored_index_counts_as_free() {
        let current = Uuid::new_v4();
        let (logs, _) = day_logs(&[row(19, Some(current), Some(-1))], Some(current));
        assert!(logs[&oct(19)].plan.is_empty());
        assert!(logs[&oct(19)].any_session);
    }

    #[test]
    fn logs_only_marks_logged_days_done_and_the_rest_nothing() {
        let logs: BTreeMap<NaiveDate, DayLog> = [(
            oct(20),
            DayLog {
                plan: vec![],
                any_session: true,
            },
        )]
        .into_iter()
        .collect();
        assert_eq!(
            logs_only(oct(19), oct(21), &logs),
            vec![
                (oct(19), DayState::Nothing { pending: None }),
                (
                    oct(20),
                    DayState::Done {
                        completed: vec![],
                        pending: None
                    }
                ),
                (oct(21), DayState::Nothing { pending: None }),
            ]
        );
    }
}
