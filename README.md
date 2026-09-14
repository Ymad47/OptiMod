# OptiMod

OptiMod is a resource-aware inference advisor for `llama.cpp` and GGUF models running on constrained computers.

> Status: early implementation. Read-only Linux host inspection works. GGUF parsing, recommendations, and benchmarking remain under development.

## Goal

Given a host, model, workload, and installed `llama.cpp` build, OptiMod will:

1. inspect CPU, memory, accelerator, and model-storage constraints;
2. read relevant GGUF metadata;
3. reject configurations that cannot fit safely;
4. build candidates for several optimization goals;
5. benchmark candidates through installed `llama.cpp` tools;
6. report measured trade-offs instead of claiming one universal optimum.

Initial manual test target: Qwen 28B GGUF inference through `llama.cpp`. Design remains model-agnostic.

## Planned profiles

- `memory-saver` — minimize resident memory while retaining explicit safety headroom;
- `balanced` — balance fit, latency, throughput, and expected quality;
- `quality-first` — prefer less aggressive quantization when resources allow;
- `fastest-measured` — select from configurations actually benchmarked on this host.

## Non-goals for the first release

- Reimplementing an inference engine
- Replacing `llama.cpp`
- Silently downloading or deleting models
- Mutating swap, sysctls, affinity, or system services
- Claiming exact quality from a quantization label
- Sending hardware or model information to a remote service

## CLI

```text
optimod inspect [--path <PATH>] [--json]       available
optimod model inspect <MODEL.gguf>
optimod recommend <MODEL.gguf> --profile balanced
optimod benchmark <MODEL.gguf> --runs 3
optimod report --json
```

Unfinished commands exit with a clear `not implemented` error. They never emit fabricated recommendations or benchmark results.

Linux inspection reads kernel and system interfaces without changing host state:

```bash
cargo run -- inspect
cargo run -- inspect --json
cargo run -- inspect --path /path/to/models
```

It reports visible CPU topology and inference features, host and cgroup-aware memory, swap, known GPU vendors exposed through DRM, and filesystem capacity for selected path. Unknown display-only DRM devices are not presented as inference accelerators.

## Build

Requirements:

- Rust 1.85 or newer
- Cargo

```bash
git clone https://github.com/Ymad47/OptiMod.git
cd OptiMod
cargo build
cargo run -- --help
cargo test
```

Quality checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --release --locked
```

## Structure

```text
src/
├── benchmark/       repeatable benchmark result contract
├── hardware/        Linux host inspection and resource snapshots
├── llama_cpp/       installed runtime and capability detection
├── metrics/         latency, throughput, memory, and I/O measurements
├── model/           GGUF metadata contract
├── recommendation/  candidate and recommendation contracts
├── report/          stable report schema
├── cli.rs           CLI surface
├── lib.rs           library module boundaries
├── main.rs          process entry point
└── workload.rs      workload constraints and optimization goals
```

See [`docs/architecture.md`](docs/architecture.md) for boundaries and [`docs/roadmap.md`](docs/roadmap.md) for implementation phases.

## Design rules

- Inspection and reporting are read-only by default.
- Measured results outrank static heuristics.
- Memory accounting includes model data, runtime overhead, KV cache, concurrency, and host safety headroom.
- Cold-load I/O and steady-state inference are measured separately.
- Supported options come from the installed `llama.cpp` build; flags are not assumed permanent.
- JSON output carries a schema version for downstream tooling.

## License

Licensed under either Apache License 2.0 or MIT License, at your option.
