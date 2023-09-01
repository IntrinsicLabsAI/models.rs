use axum::{
    routing::{delete, get, post, put},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

pub mod generate;
pub mod hfhub;
pub mod imports;
pub mod models;

async fn healthz() -> Json<String> {
    Json("healthy".to_string())
}

/// Main router for the application, with all API and health endpoints attached
pub fn app_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        //
        // CRUD operations on models and versions
        //
        .route("/v1/models", get(models::get_models))
        .route(
            "/v1/models/:model_name/description",
            get(models::get_model_description),
        )
        .route(
            "/v1/models/:model_name/description",
            put(models::update_model_description),
        )
        .route("/v1/models/:model_name/name", post(models::rename_model))
        .route("/v1/models/:model_name", delete(models::delete_model))
        .route(
            "/v1/models/:model_name/versions/:version",
            delete(models::delete_model_version),
        )
        //
        // ML model execution
        //
        .route("/v1/complete", post(generate::generate))
        //
        // Import flow
        //
        .route("/v1/imports", post(imports::import_model))
        .route("/v1/imports", get(imports::import_job_status_all))
        .route("/v1/imports/:job_id", get(imports::import_job_status))
        //
        // HF Browser endpoint for import flow
        //
        .route("/hf/ls/:community/:repo_name", get(hfhub::ls_repo_files))
        //
        // Enable all of the CORS flags
        //
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
}
