# docetl-rs

Pure-Rust port of [ucbepic/docetl](https://github.com/ucbepic/docetl) — declarative, agentic map-reduce engine for LLM pipelines over unstructured documents (`Map`, `Filter`, `Split`, `Gather`, `Reduce`, `Resolve`, `Unnest`) with automatic long-document `Split -> Gather` optimization.

Zero Python runtime required.

## Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| `docetl-core` | YAML `PipelineSpec`, `optimize_pipeline` rewrite pass, async parallel `DocRunner`, `DocLlm` trait, and `MockDocLlm` |
| `docetl-cli` | `docetl` CLI (`demo`) |

## Quickstart

```bash
cargo test --workspace
cargo run -p docetl-cli -- demo
```
