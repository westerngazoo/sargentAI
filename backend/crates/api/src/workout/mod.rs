//! Workout-log surface: full CRUD under `/workouts`.

mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workouts", post(handlers::create).get(handlers::list))
        .route(
            "/workouts/:id",
            get(handlers::get_one)
                .put(handlers::replace)
                .delete(handlers::delete),
        )
}
