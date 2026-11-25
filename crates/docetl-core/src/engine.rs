//! Parallel async DocETL pipeline execution engine.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};
use tokio::task::JoinSet;

use crate::llm::DocLlm;
use crate::pipeline::{OperationKind, PipelineSpec};
use crate::DocEtlError;

/// JSON object representing a single document / record flowing through DocETL.
pub type Document = Map<String, Value>;

/// Executes a [`PipelineSpec`] over a batch of [`Document`]s using `llm`.
pub struct DocRunner {
    llm: Arc<dyn DocLlm>,
}

impl DocRunner {
    /// Creates a new [`DocRunner`] backed by `llm`.
    #[must_use]
    pub fn new(llm: Arc<dyn DocLlm>) -> Self {
        Self { llm }
    }

    /// Runs every operation in `spec` sequentially, executing per-document LLM
    /// operators (`Map`, `Filter`, `Resolve`) concurrently via Tokio tasks.
    ///
    /// # Errors
    /// Returns [`DocEtlError`] if any operator or LLM invocation fails.
    pub async fn run(
        &self,
        spec: &PipelineSpec,
        initial_docs: Vec<Document>,
    ) -> Result<Vec<Document>, DocEtlError> {
        let mut docs = initial_docs;
        for op in &spec.operations {
            docs = match &op.kind {
                OperationKind::Map { prompt } => self.exec_map(prompt, docs).await?,
                OperationKind::Filter { prompt } => self.exec_filter(prompt, docs).await?,
                OperationKind::Split {
                    content_key,
                    chunk_size,
                    overlap,
                } => exec_split(content_key, *chunk_size, *overlap, docs)?,
                OperationKind::Gather {
                    content_key,
                    peripheral_chars,
                } => exec_gather(content_key, *peripheral_chars, docs),
                OperationKind::Reduce { reduce_key, prompt } => {
                    self.exec_reduce(reduce_key, prompt, docs).await?
                }
                OperationKind::Resolve { target_key, prompt } => {
                    self.exec_resolve(target_key, prompt, docs).await?
                }
                OperationKind::Unnest { unnest_key } => exec_unnest(unnest_key, docs),
            };
        }
        Ok(docs)
    }

    async fn exec_map(
        &self,
        prompt: &str,
        docs: Vec<Document>,
    ) -> Result<Vec<Document>, DocEtlError> {
        let mut join_set = JoinSet::new();
        for (idx, doc) in docs.into_iter().enumerate() {
            let llm = Arc::clone(&self.llm);
            let rendered = format!("{prompt}\nDocument: {}", Value::Object(doc.clone()));
            join_set.spawn(async move {
                let extracted = llm.complete_json(&rendered).await?;
                let mut merged = doc;
                if let Value::Object(map) = extracted {
                    for (k, v) in map {
                        merged.insert(k, v);
                    }
                }
                Ok::<(usize, Document), DocEtlError>((idx, merged))
            });
        }
        collect_ordered(join_set).await
    }

    async fn exec_filter(
        &self,
        prompt: &str,
        docs: Vec<Document>,
    ) -> Result<Vec<Document>, DocEtlError> {
        let mut join_set = JoinSet::new();
        for (idx, doc) in docs.into_iter().enumerate() {
            let llm = Arc::clone(&self.llm);
            let rendered = format!("FILTER: {prompt}\nDocument: {}", Value::Object(doc.clone()));
            join_set.spawn(async move {
                let res = llm.complete_json(&rendered).await?;
                let keep = res.get("keep").and_then(Value::as_bool).unwrap_or(false);
                Ok::<(usize, Option<Document>), DocEtlError>((idx, keep.then_some(doc)))
            });
        }
        let mut items = Vec::new();
        while let Some(joined) = join_set.join_next().await {
            let (idx, maybe_doc) =
                joined.map_err(|e| DocEtlError::Operator(format!("task panic: {e}")))??;
            if let Some(d) = maybe_doc {
                items.push((idx, d));
            }
        }
        items.sort_by_key(|(idx, _)| *idx);
        Ok(items.into_iter().map(|(_, d)| d).collect())
    }

    async fn exec_reduce(
        &self,
        reduce_key: &str,
        prompt: &str,
        docs: Vec<Document>,
    ) -> Result<Vec<Document>, DocEtlError> {
        let mut groups: HashMap<String, Vec<Document>> = HashMap::new();
        for doc in docs {
            let key = doc
                .get(reduce_key)
                .and_then(Value::as_str)
                .unwrap_or("_default")
                .to_string();
            groups.entry(key).or_default().push(doc);
        }

        let mut out = Vec::with_capacity(groups.len());
        for (key, group_docs) in groups {
            let rendered = format!(
                "REDUCE: {prompt}\nGroup `{key}` ({} docs): {}",
                group_docs.len(),
                serde_json::to_string(&group_docs)?
            );
            let agg = self.llm.complete_json(&rendered).await?;
            let mut row = Document::new();
            row.insert(reduce_key.to_string(), Value::String(key));
            row.insert("count".to_string(), Value::from(group_docs.len() as u64));
            if let Value::Object(map) = agg {
                for (k, v) in map {
                    row.insert(k, v);
                }
            }
            out.push(row);
        }
        Ok(out)
    }

    async fn exec_resolve(
        &self,
        target_key: &str,
        prompt: &str,
        docs: Vec<Document>,
    ) -> Result<Vec<Document>, DocEtlError> {
        let mut join_set = JoinSet::new();
        for (idx, mut doc) in docs.into_iter().enumerate() {
            let llm = Arc::clone(&self.llm);
            let raw_val = doc
                .get(target_key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let key = target_key.to_string();
            let rendered = format!("RESOLVE: {prompt}\nValue: {raw_val}");
            join_set.spawn(async move {
                let res = llm.complete_json(&rendered).await?;
                if let Some(canon) = res.get("canonical").and_then(Value::as_str) {
                    doc.insert(key, Value::String(canon.to_string()));
                }
                Ok::<(usize, Document), DocEtlError>((idx, doc))
            });
        }
        collect_ordered(join_set).await
    }
}

async fn collect_ordered(
    mut join_set: JoinSet<Result<(usize, Document), DocEtlError>>,
) -> Result<Vec<Document>, DocEtlError> {
    let mut items = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        let pair = joined.map_err(|e| DocEtlError::Operator(format!("task panic: {e}")))??;
        items.push(pair);
    }
    items.sort_by_key(|(idx, _)| *idx);
    Ok(items.into_iter().map(|(_, d)| d).collect())
}

fn exec_split(
    content_key: &str,
    chunk_size: usize,
    overlap: usize,
    docs: Vec<Document>,
) -> Result<Vec<Document>, DocEtlError> {
    let max_len = chunk_size.max(1);
    let step = max_len.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    for doc in docs {
        let text = doc
            .get(content_key)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                DocEtlError::Operator(format!("missing string field `{content_key}`"))
            })?;
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0usize;
        let mut chunk_idx = 0usize;
        while start < chars.len() {
            let end = (start + max_len).min(chars.len());
            let mut chunk_doc = doc.clone();
            let chunk_str: String = chars[start..end].iter().collect();
            chunk_doc.insert(content_key.to_string(), Value::String(chunk_str));
            chunk_doc.insert("_chunk_id".to_string(), Value::from(chunk_idx as u64));
            out.push(chunk_doc);
            chunk_idx += 1;
            if end == chars.len() {
                break;
            }
            start += step;
        }
    }
    Ok(out)
}

fn exec_gather(
    content_key: &str,
    peripheral_chars: usize,
    mut docs: Vec<Document>,
) -> Vec<Document> {
    let prev_tails: Vec<Option<String>> = docs
        .iter()
        .map(|d| {
            d.get(content_key).and_then(Value::as_str).map(|s| {
                let chars: Vec<char> = s.chars().collect();
                let keep = peripheral_chars.min(chars.len());
                chars[chars.len() - keep..].iter().collect()
            })
        })
        .collect();

    for (i, doc) in docs.iter_mut().enumerate() {
        if i > 0 {
            if let (Some(prev), Some(Value::String(cur))) =
                (&prev_tails[i - 1], doc.get_mut(content_key))
            {
                *cur = format!("[prev context: {prev}]\n{cur}");
            }
        }
    }
    docs
}

fn exec_unnest(unnest_key: &str, docs: Vec<Document>) -> Vec<Document> {
    let mut out = Vec::new();
    for doc in docs {
        if let Some(Value::Array(items)) = doc.get(unnest_key) {
            for item in items {
                let mut row = doc.clone();
                row.insert(unnest_key.to_string(), item.clone());
                out.push(row);
            }
        } else {
            out.push(doc);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockDocLlm;
    use crate::optimizer::{optimize_pipeline, OptimizerConfig};

    #[tokio::test]
    async fn transcript_pipeline_filters_maps_resolves_and_reduces() {
        let yaml = r#"
name: call_intake_pipeline
operations:
  - name: drop_spam
    type: filter
    prompt: "Exclude spam or hangups"
  - name: extract_case
    type: map
    prompt: "Extract case_type and sentiment"
  - name: canonicalize_case
    type: resolve
    target_key: case_type
    prompt: "Normalize case_type"
  - name: summarize_by_case
    type: reduce
    reduce_key: case_type
    prompt: "Summarize call volume"
"#;
        let spec = PipelineSpec::from_yaml(yaml).expect("valid yaml");
        let optimized = optimize_pipeline(&spec, 500, "transcript", OptimizerConfig::default());
        assert_eq!(optimized.operations.len(), 4);

        let docs: Vec<Document> = vec![
            serde_json::from_str(
                r#"{"id": 1, "transcript": "I was in a rear-end crash and I am hurt."}"#,
            )
            .expect("doc1"),
            serde_json::from_str(r#"{"id": 2, "transcript": "Automated spam robocall"}"#)
                .expect("doc2"),
        ];
        let runner = DocRunner::new(Arc::new(MockDocLlm));
        let out = runner.run(&optimized, docs).await.expect("run succeeds");
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].get("case_type").and_then(Value::as_str),
            Some("Auto Accident")
        );
        assert_eq!(out[0].get("count").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn optimizer_inserts_split_and_gather_for_long_documents() {
        let spec = PipelineSpec::from_yaml(
            "name: long\noperations:\n  - name: extract\n    type: map\n    prompt: Extract\n",
        )
        .expect("yaml");
        let opt = optimize_pipeline(&spec, 12_000, "transcript", OptimizerConfig::default());
        assert_eq!(opt.operations.len(), 3);
        assert!(matches!(
            opt.operations[0].kind,
            OperationKind::Split { .. }
        ));
        assert!(matches!(
            opt.operations[1].kind,
            OperationKind::Gather { .. }
        ));
    }
}
