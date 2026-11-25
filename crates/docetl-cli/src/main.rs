//! CLI for running and optimizing DocETL declarative LLM pipelines.

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use docetl_core::{
    optimize_pipeline, DocRunner, Document, MockDocLlm, OptimizerConfig, PipelineSpec,
};

#[derive(Parser)]
#[command(
    name = "docetl",
    about = "Pure-Rust DocETL declarative LLM map-reduce CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a call-transcript filter -> map -> resolve -> reduce pipeline demo.
    Demo,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Demo => {
            let spec = PipelineSpec::from_yaml(
                r#"
name: transcript_triage
operations:
  - name: drop_spam
    type: filter
    prompt: "Exclude spam calls"
  - name: extract
    type: map
    prompt: "Extract case_type and sentiment"
  - name: resolve_case
    type: resolve
    target_key: case_type
    prompt: "Canonicalize case_type"
  - name: group_summary
    type: reduce
    reduce_key: case_type
    prompt: "Summarize calls per case type"
"#,
            )?;
            let opt = optimize_pipeline(&spec, 500, "transcript", OptimizerConfig::default());
            let docs: Vec<Document> = vec![
                serde_json::from_str(
                    r#"{"id": 1, "transcript": "Rear-end crash on I-405, urgent pain."}"#,
                )?,
                serde_json::from_str(r#"{"id": 2, "transcript": "Spam warranty call"}"#)?,
            ];
            let runner = DocRunner::new(Arc::new(MockDocLlm));
            let result = runner.run(&opt, docs).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}
