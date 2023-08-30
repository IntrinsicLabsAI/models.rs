pub mod types {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

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

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    #[serde(tag = "type")]
    pub enum Locator {
        #[serde(rename = "hf/v1")]
        HFLocator { repo_id: String, file_path: PathBuf },

        #[serde(rename = "disk/v1")]
        DiskLocator { path: PathBuf },
    }
}

pub mod endpoints {
    use super::types::{self, GetRegisteredModelsResponse};
    use crate::{db::tables, state::AppState};
    use axum::{extract::State, http::StatusCode, Json};

    pub async fn get_models(
        State(app_state): State<AppState>,
    ) -> Result<Json<types::GetRegisteredModelsResponse>, StatusCode> {
        let mut conn = app_state.db.conn.lock().await;
        let tx = conn.transaction().unwrap();
        let stored_models: Vec<tables::Model> = {
            let mut stmt = tx
                .prepare("select id, name, model_type, runtime, description from model order by id")
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let models = stmt
                .query([])
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .mapped(|row| {
                    Ok(tables::Model {
                        id: row.get_ref(0)?.as_str()?.to_owned(),
                        name: row.get_ref_unwrap(1).as_str()?.to_owned(),
                        model_type: row.get_ref_unwrap(2).as_str()?.to_owned(),
                        runtime: row.get_ref_unwrap(3).as_str()?.to_owned(),
                        description: row.get_ref_unwrap(4).as_str()?.to_owned(),
                    })
                })
                .filter(|res| res.is_ok())
                .map(|res| res.unwrap());

            models.collect()
        };

        let api_models: Vec<types::RegisteredModel> =
            stored_models.iter().map(|m| m.into()).collect();

        let result = GetRegisteredModelsResponse { models: api_models };

        Ok(Json(result))
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

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

    #[test]
    pub fn import_locator_serde() {
        let locator = types::Locator::HFLocator {
            repo_id: "meta-llm/llama".to_owned(),
            file_path: PathBuf::from("consolidated.00.pth"),
        };
        assert_eq!(
            r#"{"type":"hf/v1","repo_id":"meta-llm/llama","file_path":"consolidated.00.pth"}"#,
            serde_json::to_string(&locator).unwrap()
        );

        assert_eq!(
            serde_json::from_str::<types::Locator>(
                r#"{"type":"hf/v1","repo_id":"meta-llm/llama","file_path":"consolidated.00.pth"}"#,
            )
            .unwrap(),
            locator
        );
    }
}
