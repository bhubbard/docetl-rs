# Benchmarks & Coverage

Measured on **Apple M3 Pro** (`arm64`, 11 cores, 18 GiB RAM, macOS) with `rustc 1.100.0` (`--release`) vs. `Python 3.15.2` (same host) and upstream `ucbepic/docetl`.

## 1. Code Coverage (`cargo llvm-cov --workspace --summary-only`)

| File | Region Coverage | Function Coverage | Line Coverage |
| :--- | ---: | ---: | ---: |
| `docetl-core/src/llm.rs` | 100.00% | 100.00% | 100.00% |
| `docetl-core/src/optimizer.rs` | 96.88% | 100.00% | 97.83% |
| `docetl-core/src/engine.rs` | 64.87% | 65.52% | 69.85% |
| **Workspace Total** *(incl. CLI)* | **62.54%** | **64.86%** | **60.36%** |

## 2. Footprint, Startup & Microbenchmarks

| Metric / Operation | `docetl-rs` (Rust) | Upstream `ucbepic/docetl` (Python) | Comparison |
| :--- | ---: | ---: | :--- |
| **Standalone Binary Size** | **1.97 MiB** | Python + `litellm` + `pydantic` + `rich` | Single static binary |
| **Cold Startup (5-run median)** | **5.96 ms** | **23.61 ms** (`python3 -c ""` bare stdlib) | **3.96× faster** cold start |
| `DocRunner(100 transcripts, 4 ops)` | **0.5544 ms** | Python `asyncio` + `litellm` dispatch | **~5.5 µs** per transcript |

Reproduce locally:
```bash
cargo run --release -p docetl-core --example bench
cargo llvm-cov --workspace --summary-only
```
