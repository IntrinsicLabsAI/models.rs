use crate::{
    api_types::{GetAllJobStatusResponse, ImportJob, ImportJobId, ImportJobStatus, Locator},
    state::AppState,
};
use anyhow::Context;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[axum::debug_handler]
pub async fn import_model(
    State(app_state): State<AppState>,
    Json(locator): Json<Locator>,
) -> Result<Json<ImportJobId>, StatusCode> {
    let import_job = match locator {
        Locator::DISK(disk_locator) => ImportJob::DISK {
            locator: disk_locator,
        },
        Locator::HF(hf_locator) => ImportJob::HF {
            locator: hf_locator,
        },
    };

    let result = {
        let importer = app_state.importer;
        importer.start_import(import_job).await
    };

    let job_id = result.unwrap();
    // .context("failed to start import")
    // .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(job_id))
}

#[axum::debug_handler]
pub async fn import_job_status(
    Path(job_id): Path<ImportJobId>,
    State(app_state): State<AppState>,
) -> Result<Json<ImportJobStatus>, StatusCode> {
    let task_status = app_state
        .importer
        .get_import_status(&job_id)
        .await
        .context("failed to retrieve import job status")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_status))
}

pub async fn import_job_status_all(
    State(app_state): State<AppState>,
) -> Result<Json<GetAllJobStatusResponse>, StatusCode> {
    let import_jobs = app_state
        .importer
        .get_all_job_status()
        .await
        .context("failed to retrieve import job status")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GetAllJobStatusResponse { import_jobs }))
}
