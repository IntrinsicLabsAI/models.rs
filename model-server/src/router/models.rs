use axum::{Json, http::StatusCode};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{self, de, Deserialize, Serialize, Serializer};

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

/// The semantic version specialized type that indicates the model version.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct ModelVersion(u32, u32, u32);

/// Custom serializer implementation
impl Serialize for ModelVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let triple = format!("{}.{}.{}", self.0, self.1, self.2);
        serializer.serialize_str(&triple)
    }
}

struct ModelVersionVisitor;

static PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"([0-9]+).([0-9+]).([0-9]+)").unwrap());

impl<'de> serde::de::Visitor<'de> for ModelVersionVisitor {
    type Value = ModelVersion;

    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("a SemVer version 2.0.0 triple as a string, e.g. \"2.0.0\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some((_, [major, minor, patch])) = PATTERN.captures(&v).map(|c| c.extract()) {
            if let (Ok(major), Ok(minor), Ok(patch)) = (
                major.parse::<u32>(),
                minor.parse::<u32>(),
                patch.parse::<u32>(),
            ) {
                return Ok(ModelVersion(major, minor, patch));
            } else {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Other("value exceeded u32 bounds"),
                    &"major, minor, and patch must fit in u32",
                ));
            }
        } else {
            return Err(de::Error::invalid_value(
                de::Unexpected::Other("string did not match regex"),
                &"A SemVer dot-separate triple ex. \"2.0.0\"",
            ));
        }
    }
}

impl<'de> Deserialize<'de> for ModelVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ModelVersionVisitor)
    }
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

/// Handler for v1 get_models endpoint
async fn get_models(
    // Have access to the DB stuff    
) -> Result<Json<GetRegisteredModelsResponse>, StatusCode> {
    todo!()
}

#[cfg(test)]
mod test {
    use crate::router::models::{ModelType, Runtime};

    use super::{ModelVersion, RegisteredModel};

    #[test]
    pub fn model_version_serde() {
        assert_eq!(
            serde_json::to_string(&ModelVersion(101, 202, 303)).unwrap(),
            r#""101.202.303""#
        );
        assert_eq!(
            serde_json::from_str::<ModelVersion>(r#""1.2.3""#).unwrap(),
            ModelVersion(1, 2, 3)
        );
    }

    #[test]
    pub fn api_serde() {
        let registered_model = RegisteredModel {
            id: uuid::uuid!("6f479fd1-d7eb-4ca0-b15e-e61743e561fd"),
            model_type: ModelType::Completion,
            runtime: Runtime::Ggml,
            name: "my-model".to_owned(),
        };

        assert_eq!(
            registered_model,
            serde_json::from_str::<RegisteredModel>(
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
