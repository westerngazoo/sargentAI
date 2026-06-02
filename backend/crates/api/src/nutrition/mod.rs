//! Nutrition-log surface: full CRUD under `/nutrition`.

mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nutrition", post(handlers::create).get(handlers::list))
        .route(
            "/nutrition/:id",
            get(handlers::get_one)
                .put(handlers::replace)
                .delete(handlers::delete),
        )
}
