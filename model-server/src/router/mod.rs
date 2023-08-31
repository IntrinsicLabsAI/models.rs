use axum::{
    routing::{get, post, put},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

pub mod generate;
pub mod imports;
pub mod models;

async fn healthz() -> Json<String> {
    Json("healthy".to_string())
}

/// Main router for the application, with all API and health endpoints attached
pub fn app_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(models::get_models))
        .route(
            "/v1/models/:model_name/description",
            get(models::get_model_description),
        )
        .route(
            "/v1/models/:model_name/description",
            put(models::update_model_description),
        )
        .route("/v1/complete", post(generate::generate))
        .route("/v1/imports", post(imports::import_model))
        .route("/v1/imports", get(imports::import_job_status_all))
        .route("/imports/:job_id", get(imports::import_job_status))
        // CORS Allow All
        .layer(CorsLayer::new().allow_origin(Any))
}
