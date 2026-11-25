//! Declarative DocETL pipeline and operator specifications.

use serde::{Deserialize, Serialize};

use crate::DocEtlError;

/// Supported DocETL declarative operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationKind {
    /// Applies an LLM extraction prompt per document.
    Map {
        /// Prompt template referencing document fields.
        prompt: String,
    },
    /// Keeps documents where the LLM predicate returns `{"keep": true}`.
    Filter {
        /// Predicate prompt.
        prompt: String,
    },
    /// Splits `content_key` into overlapping character chunks.
    Split {
        /// Source text field name.
        content_key: String,
        /// Target chunk size in characters.
        chunk_size: usize,
        /// Overlap in characters between adjacent chunks.
        #[serde(default)]
        overlap: usize,
    },
    /// Attaches neighboring chunk context (`context_window` chars) to each split chunk.
    Gather {
        /// Chunk field name produced by `Split`.
        content_key: String,
        /// Surrounding character window to include.
        #[serde(default = "default_gather_window")]
        peripheral_chars: usize,
    },
    /// Groups documents by `reduce_key` and synthesizes per group.
    Reduce {
        /// Grouping key field name.
        reduce_key: String,
        /// Aggregation prompt.
        prompt: String,
    },
    /// Canonicalizes entity/category values in `target_key`.
    Resolve {
        /// Field name to canonicalize.
        target_key: String,
        /// Resolution prompt.
        prompt: String,
    },
    /// Explodes an array field into one document per element.
    Unnest {
        /// Array field name.
        unnest_key: String,
    },
}

const fn default_gather_window() -> usize {
    200
}

/// A named step in a [`PipelineSpec`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    /// Unique step name.
    pub name: String,
    /// Operator definition.
    #[serde(flatten)]
    pub kind: OperationKind,
}

/// Top-level declarative DocETL pipeline specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Pipeline name.
    pub name: String,
    /// Ordered list of operations.
    pub operations: Vec<Operation>,
}

impl PipelineSpec {
    /// Parses a [`PipelineSpec`] from YAML.
    ///
    /// # Errors
    /// Returns [`DocEtlError::Yaml`] if the YAML is malformed.
    pub fn from_yaml(yaml: &str) -> Result<Self, DocEtlError> {
        Ok(serde_yaml::from_str(yaml)?)
    }
}
