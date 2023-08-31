//! Types needed by the API of our project.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use time::OffsetDateTime;

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
/// ModelType corresponds to the category of model. Currently accepted values include
/// Completion: a completion language model.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    #[serde(rename = "completion")]
    Completion,
}

/// Runtime indicates the runtime the model is built for. Some common examples are "ggml" or "onnx".
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    #[serde(rename = "ggml")]
    Ggml,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RegisteredModel {
    pub id: uuid::Uuid,
    pub name: String,
    pub model_type: ModelType,
    pub runtime: Runtime,
    pub versions: Vec<ModelVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ModelVersion {
    pub version: semver::Version,
    pub import_metadata: ImportMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterModelRequest {
    pub model: String,
    pub version: semver::Version,
    pub model_type: ModelType,
    pub runtime: Runtime,
    pub import_metadata: ImportMetadata,
    pub internal_params: ModelParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ModelParams {
    #[serde(rename = "paramsv1/completion")]
    COMPLETION(CompletionModelParams),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletionModelParams {
    pub model_path: PathBuf,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ImportMetadata {
    pub imported_at: OffsetDateTime,
    pub source: ImportSource,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HFLocator {
    pub repo: String,
    pub file: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct DiskLocator {
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum Locator {
    #[serde(rename = "locatorv1/hf")]
    HF(HFLocator),

    #[serde(rename = "locatorv1/disk")]
    DISK(DiskLocator),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ImportSource {
    #[serde(rename = "importv1/hf")]
    HF(HFLocator),

    #[serde(rename = "importv1/disk")]
    DISK(DiskLocator),
}

#[derive(Serialize)]
pub struct GetAllJobStatusResponse {
    pub import_jobs: HashMap<ImportJobId, ImportJobStatus>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ImportJob {
    // Depending on the task, we want to include the subtypes of the locator here as well instead...fuck
    HF { locator: HFLocator },
    DISK { locator: DiskLocator },
}

// Have it enqueue a task, and return an ID
pub type ImportJobId = uuid::Uuid;

/// Status of an import job.
/// Import jobs can be in one of three different states at a given point in time
/// - **[Queued]** - for imports that are taking too long
/// - **[InProgress]** - for imports that are actively being worked on
/// - **[Completed]** - for imports that are complete and cached locally on disk
/// - **[Failed]** - for import jobs that failed with an error
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ImportJobStatus {
    #[serde(rename = "queued")]
    Queued,

    #[serde(rename = "in-progress")]
    InProgress {
        /// A numberic progress indicator between 0 (0%) and 1.0 (100%)
        progress: f32,
    },

    #[serde(rename = "completed")]
    Completed { info: Option<String> },

    #[serde(rename = "finished")]
    Failed {
        // We need to keep track of an error, so that it's sendable, and so that we can log it for later.
        error: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ListHFFiles {
    pub repo: String,
    pub subfolder: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HFFile {
    pub filename: String,
    pub subfolder: Option<String>,
    pub size_bytes: usize,
    pub committed_at: OffsetDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ListHFFilesResponse {
    pub repo: String,
    pub files: Vec<HFFile>,
}
