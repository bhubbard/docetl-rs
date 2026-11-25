//! Async LLM interface for DocETL operators.

use async_trait::async_trait;
use serde_json::Value;

use crate::DocEtlError;

/// Asynchronous LLM interface invoked by `Map`, `Filter`, `Reduce`, and `Resolve`.
#[async_trait]
pub trait DocLlm: Send + Sync {
    /// Executes an extraction or synthesis prompt and returns structured JSON.
    ///
    /// # Errors
    /// Returns [`DocEtlError::Llm`] if inference or JSON parsing fails.
    async fn complete_json(&self, prompt: &str) -> Result<Value, DocEtlError>;
}

/// Deterministic rule-based [`DocLlm`] for offline testing and call-transcript demos.
#[derive(Debug, Default)]
pub struct MockDocLlm;

#[async_trait]
impl DocLlm for MockDocLlm {
    async fn complete_json(&self, prompt: &str) -> Result<Value, DocEtlError> {
        let lower = prompt.to_ascii_lowercase();
        let doc_part = lower.split("\ndocument:").nth(1).unwrap_or(&lower);
        if lower.contains("filter:") {
            let keep = !doc_part.contains("spam") && !doc_part.contains("hangup");
            return Ok(serde_json::json!({ "keep": keep }));
        }
        if lower.contains("resolve:") {
            let canonical = if lower.contains("auto") || lower.contains("collision") {
                "Auto Accident"
            } else {
                "General Inquiry"
            };
            return Ok(serde_json::json!({ "canonical": canonical }));
        }
        if lower.contains("reduce:") {
            return Ok(serde_json::json!({
                "summary": "Aggregated call transcript insights across group."
            }));
        }
        let case_type = if lower.contains("rear-end") || lower.contains("crash") {
            "Auto Accident"
        } else {
            "General Inquiry"
        };
        let sentiment = if lower.contains("urgent") || lower.contains("hurt") {
            "distressed"
        } else {
            "neutral"
        };
        Ok(serde_json::json!({
            "case_type": case_type,
            "sentiment": sentiment
        }))
    }
}
