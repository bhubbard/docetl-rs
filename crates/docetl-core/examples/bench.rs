//! Release benchmark for `docetl-core` optimization and parallel map-reduce execution.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use docetl_core::{
    optimize_pipeline, DocRunner, Document, MockDocLlm, OptimizerConfig, PipelineSpec,
};

#[tokio::main]
async fn main() {
    let spec = PipelineSpec::from_yaml(
        r#"
name: transcript_triage
operations:
  - name: drop_spam
    type: filter
    prompt: "Exclude spam"
  - name: extract
    type: map
    prompt: "Extract case_type"
  - name: resolve_case
    type: resolve
    target_key: case_type
    prompt: "Canonicalize"
  - name: reduce_case
    type: reduce
    reduce_key: case_type
    prompt: "Summarize"
"#,
    )
    .expect("yaml");
    let opt = optimize_pipeline(&spec, 500, "transcript", OptimizerConfig::default());
    let docs: Vec<Document> = (0..100)
        .map(|i| {
            serde_json::from_str(&format!(
                r#"{{"id": {i}, "transcript": "Rear-end crash on highway, urgent injury."}}"#
            ))
            .expect("doc")
        })
        .collect();
    let runner = DocRunner::new(Arc::new(MockDocLlm));

    let mut times = Vec::with_capacity(15);
    for i in 0..18 {
        let t0 = Instant::now();
        let res = black_box(runner.run(&opt, docs.clone()).await.expect("run"));
        black_box(res.len());
        if i >= 3 {
            times.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "DocRunner(100 transcripts, 4 ops): median={:.4} ms (min={:.4} ms)",
        times[times.len() / 2],
        times[0]
    );
}
