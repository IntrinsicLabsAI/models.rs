use axum::{extract::State, http::StatusCode, Json};
use std::borrow::BorrowMut;

use crate::state::AppState;

pub mod types {
    use once_cell::sync::Lazy;
    use regex::Regex;
    use serde::{de, Deserialize, Serialize, Serializer};

    /// ModelType corresponds to the category of model. Currently accepted values include
    /// Completion: a completion language model.
    #[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
    pub enum ModelType {
        #[serde(rename = "completion")]
        Completion,
    }

    /// Runtime indicates the runtime the model is built for. Some common examples are "ggml" or "onnx".
    #[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
    pub enum Runtime {
        #[serde(rename = "ggml")]
        Ggml,
    }

    #[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
    pub struct RegisteredModel {
        pub id: uuid::Uuid,
        pub name: String,
        pub model_type: ModelType,
        pub runtime: Runtime,
    }

    #[derive(Serialize, Deserialize)]
    pub struct CompletionInferenceRequest {
        /// Prompt for the inference engine to complete against.
        pub prompt: String,

        /// Number of tokens to generate.
        pub tokens: u32,

        /// Temperature for generation.
        #[serde(default)]
        pub temperature: f32,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct GetRegisteredModelsResponse {
        pub models: Vec<RegisteredModel>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct HealthStatus {
        pub status: String,
    }
}

// How to convert any error we receive to a standard status code?

/// Handler for v1 get_models endpoint
async fn get_models(
    State(app_state): State<AppState>,
) -> Result<Json<types::GetRegisteredModelsResponse>, StatusCode> {
    todo!()
}

#[cfg(test)]
mod test {
    use crate::router::models::types;

    #[test]
    pub fn api_serde() {
        let registered_model = types::RegisteredModel {
            id: uuid::uuid!("6f479fd1-d7eb-4ca0-b15e-e61743e561fd"),
            model_type: types::ModelType::Completion,
            runtime: types::Runtime::Ggml,
            name: "my-model".to_owned(),
        };

        assert_eq!(
            registered_model,
            serde_json::from_str::<types::RegisteredModel>(
                r#"
            {
                "id": "6f479fd1-d7eb-4ca0-b15e-e61743e561fd",
                "name": "my-model",
                "model_type": "completion",
                "runtime": "ggml"
            }
        "#
            )
            .unwrap()
        );

        assert_eq!(
            serde_json::to_string(&registered_model).unwrap(),
            r#"{"id":"6f479fd1-d7eb-4ca0-b15e-e61743e561fd","name":"my-model","model_type":"completion","runtime":"ggml"}"#
        );
    }
}
