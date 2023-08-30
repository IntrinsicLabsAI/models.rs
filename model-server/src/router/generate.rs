use crate::state::AppState;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub model_id: String,
    pub prompt: String,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub model_id: String,
    pub completion: String,
}

#[axum::debug_handler]
pub async fn generate(
    State(app_state): State<AppState>,
    Json(params): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, StatusCode> {
    let model = app_state.model;
    let completion = {
        let mut model = model.model.lock().await;
        model.generate(&params.prompt)
    };

    let res = GenerateResponse {
        model_id: params.model_id.clone(),
        completion,
    };

    Ok(Json(res))
}
