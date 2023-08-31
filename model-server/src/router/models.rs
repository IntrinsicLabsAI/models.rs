use crate::{api_types::GetRegisteredModelsResponse, state::AppState};
use anyhow::Context;
use axum::{
    body::HttpBody,
    extract::{Path, RawBody, State},
    http::StatusCode,
    Json,
};

pub async fn get_models(
    State(AppState {
        db,
        model: _,
        importer: _,
    }): State<AppState>,
) -> Result<Json<GetRegisteredModelsResponse>, StatusCode> {
    // TODO(aduffy): use central error type in the BE that can map back to StatusCode easily
    let result = db
        .get_models()
        .await
        .with_context(|| "failed to execute get_models")
        .unwrap();

    Ok(Json(GetRegisteredModelsResponse { models: result }))
}

pub async fn get_model_description(
    State(AppState {
        db,
        model: _,
        importer: _,
    }): State<AppState>,
    Path(model_name): Path<String>,
) -> Result<Json<String>, StatusCode> {
    // TODO(aduffy): actually make this return the right error type
    let desc = db.get_model_description(&model_name).await.unwrap();

    Ok(Json(desc))
}

pub async fn update_model_description(
    State(AppState {
        db,
        model: _,
        importer: _,
    }): State<AppState>,
    Path(model_name): Path<String>,
    RawBody(mut updated_desc): RawBody,
) -> StatusCode {
    let data = updated_desc.data().await.unwrap().unwrap();
    let desc = String::from_utf8(data.to_vec()).unwrap();

    // NOTE: this will fail at runtime which is bad
    db.update_model_description(&model_name, &desc)
        .await
        .unwrap();

    StatusCode::NO_CONTENT
}

pub async fn rename_model(
    State(AppState {
        db,
        model: _,
        importer: _,
    }): State<AppState>,
    Path(model_name): Path<String>,
    RawBody(mut new_name): RawBody,
) -> StatusCode {
    let data = new_name.data().await.unwrap().unwrap();
    let new_name = String::from_utf8(data.to_vec()).unwrap();
    db.rename_model(&model_name, &new_name)
        .await
        .context("DB::rename_model failed")
        .unwrap();

    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::api_types::{HFLocator, Locator, ModelType, RegisteredModel, Runtime};

    #[test]
    pub fn api_serde() {
        let registered_model = RegisteredModel {
            id: uuid::uuid!("6f479fd1-d7eb-4ca0-b15e-e61743e561fd"),
            model_type: ModelType::Completion,
            runtime: Runtime::Ggml,
            name: "my-model".to_owned(),
            versions: vec![],
        };

        assert_eq!(
            registered_model,
            serde_json::from_str::<RegisteredModel>(
                r#"
            {
                "id": "6f479fd1-d7eb-4ca0-b15e-e61743e561fd",
                "name": "my-model",
                "model_type": "completion",
                "runtime": "ggml",
                "versions": []
            }
        "#
            )
            .unwrap()
        );

        assert_eq!(
            serde_json::to_string(&registered_model).unwrap(),
            r#"{"id":"6f479fd1-d7eb-4ca0-b15e-e61743e561fd","name":"my-model","model_type":"completion","runtime":"ggml","versions":[]}"#
        );
    }

    #[test]
    pub fn import_locator_serde() {
        let locator = Locator::HF(HFLocator {
            repo: "meta-llm/llama".to_owned(),
            file: PathBuf::from("consolidated.00.pth"),
        });
        assert_eq!(
            r#"{"type":"locatorv1/hf","repo":"meta-llm/llama","file":"consolidated.00.pth"}"#,
            serde_json::to_string(&locator).unwrap()
        );

        assert_eq!(
            serde_json::from_str::<Locator>(
                r#"{"type":"locatorv1/hf","repo":"meta-llm/llama","file":"consolidated.00.pth"}"#,
            )
            .unwrap(),
            locator
        );
    }
}
