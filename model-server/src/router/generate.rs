pub mod types {
    use serde::{Deserialize, Serialize};

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
}

pub mod endpoints {
    use super::types;
    use crate::state::AppState;
    use axum::{extract::State, http::StatusCode, Json};

    #[axum::debug_handler]
    pub async fn generate(
        State(app_state): State<AppState>,
        Json(params): Json<types::GenerateRequest>,
    ) -> Result<Json<types::GenerateResponse>, StatusCode> {
        let model = app_state.model;
        let completion = {
            let mut model = model.model.lock().await;
            model.generate(&params.prompt)
        };

        let res = types::GenerateResponse {
            model_id: params.model_id.clone(),
            completion,
        };

        Ok(Json(res))
    }
}
