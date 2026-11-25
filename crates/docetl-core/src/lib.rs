//! Pure-Rust port of UC Berkeley EPIC Lab's DocETL declarative LLM map-reduce engine.
//!
//! Provides:
//! - [`pipeline`]: YAML/JSON declarative [`PipelineSpec`] with `Map`, `Filter`,
//!   `Reduce`, `Resolve`, `Split`, `Gather`, and `Unnest` operations.
//! - [`optimizer`]: Automatic long-document `Split -> Gather -> Map -> Reduce`
//!   rewrite optimizer.
//! - [`engine`]: Async parallel [`DocRunner`] executing pipelines over [`Document`]s.
//! - [`llm`]: `DocLlm` trait and deterministic `MockDocLlm`.

pub mod engine;
pub mod error;
pub mod llm;
pub mod optimizer;
pub mod pipeline;

pub use engine::{DocRunner, Document};
pub use error::DocEtlError;
pub use llm::{DocLlm, MockDocLlm};
pub use optimizer::{optimize_pipeline, OptimizerConfig};
pub use pipeline::{Operation, OperationKind, PipelineSpec};
