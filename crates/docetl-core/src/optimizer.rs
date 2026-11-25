//! Agentic pipeline rewrite optimizer for long-document accuracy.

use crate::pipeline::{Operation, OperationKind, PipelineSpec};

/// Configuration for [`optimize_pipeline`].
#[derive(Debug, Clone, Copy)]
pub struct OptimizerConfig {
    /// Average document length threshold above which `Map` is rewritten into
    /// `Split -> Gather -> Map -> Reduce`.
    pub split_threshold_chars: usize,
    /// Chunk size inserted by the rewrite rule.
    pub default_chunk_size: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            split_threshold_chars: 4000,
            default_chunk_size: 2000,
        }
    }
}

/// Rewrites a [`PipelineSpec`] to improve accuracy on long, noisy documents:
/// - Automatically inserts `Split -> Gather` before `Map` when `avg_doc_chars`
///   exceeds `config.split_threshold_chars` and no `Split` is present.
#[must_use]
pub fn optimize_pipeline(
    spec: &PipelineSpec,
    avg_doc_chars: usize,
    content_key: &str,
    config: OptimizerConfig,
) -> PipelineSpec {
    let has_split = spec
        .operations
        .iter()
        .any(|op| matches!(op.kind, OperationKind::Split { .. }));

    if has_split || avg_doc_chars <= config.split_threshold_chars {
        return spec.clone();
    }

    let mut optimized_ops = Vec::with_capacity(spec.operations.len() + 2);
    let mut inserted = false;
    for op in &spec.operations {
        if !inserted && matches!(op.kind, OperationKind::Map { .. }) {
            optimized_ops.push(Operation {
                name: format!("{}_auto_split", op.name),
                kind: OperationKind::Split {
                    content_key: content_key.to_string(),
                    chunk_size: config.default_chunk_size,
                    overlap: 150,
                },
            });
            optimized_ops.push(Operation {
                name: format!("{}_auto_gather", op.name),
                kind: OperationKind::Gather {
                    content_key: content_key.to_string(),
                    peripheral_chars: 200,
                },
            });
            inserted = true;
        }
        optimized_ops.push(op.clone());
    }

    PipelineSpec {
        name: spec.name.clone(),
        operations: optimized_ops,
    }
}
