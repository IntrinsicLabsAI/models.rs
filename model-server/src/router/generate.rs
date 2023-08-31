use crate::api_types::{GenerateRequest, GenerateResponse};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};

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
