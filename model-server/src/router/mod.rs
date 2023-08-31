use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub mod generate;
pub mod imports;
pub mod models;

pub fn app_router() -> Router<AppState> {
    Router::new()
        .route("/models", get(models::endpoints::get_models))
        .route("/complete", post(generate::endpoints::generate))
        .route("/imports", post(imports::endpoints::import_model))
        .route("/imports", get(imports::endpoints::import_job_status_all))
        .route("/imports/:job_id", get(imports::endpoints::import_job_status))
}
