//! Error types for `docetl-core`.

use thiserror::Error;

/// Errors returned by DocETL YAML parsing, optimization, and pipeline execution.
#[derive(Debug, Error)]
pub enum DocEtlError {
    /// YAML pipeline specification error.
    #[error("pipeline YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON serialization or operator output error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// LLM backend or prompt execution error.
    #[error("llm error: {0}")]
    Llm(String),

    /// Invalid operator configuration or missing field.
    #[error("operator error: {0}")]
    Operator(String),
}
