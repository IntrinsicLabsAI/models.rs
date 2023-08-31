pub use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub runtime: String,
    pub description: String,
}

/// A specific version of a [RegisteredModel]
#[derive(Debug, Clone)]
pub struct ModelVersion {
    pub version: String,
    pub import_metadata: ImportMetadata,
}

/// Metadata acquired from importing a model from a remote system, e.g. HuggingFace Hub or disk
#[derive(Debug, Clone)]
pub struct ImportMetadata {
    pub model_id: String,
    pub model_version: String,
    pub source: String,
    pub imported_at: OffsetDateTime,
}
