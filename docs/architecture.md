# Architecture

## Scope

OptiMod advises and benchmarks `llama.cpp` inference. It does not own model execution internals. Initial platform focus is Linux, especially CPU-only workstations and small VPS instances. Platform probes remain behind module boundaries so Windows and macOS support can follow.

## Data flow

```text
HostSnapshot ─┐
ModelProfile ─┼─> feasibility filter ─> candidate policy ─> benchmark ─> report
Workload ─────┤
llama.cpp ────┘
```

Static analysis establishes constraints and estimated candidates. Benchmarks establish host-specific evidence. Reports must distinguish `estimated` from `benchmarked` recommendations.

## Module boundaries

### `hardware`

Produces a read-only `HostSnapshot` containing:

- OS and architecture;
- physical and logical CPU topology;
- relevant CPU instruction features;
- RAM and swap totals plus currently available memory;
- accelerator backends and memory where discoverable;
- model-storage capacity and rotational status where discoverable.

Linux implementation should prefer kernel interfaces and stable system commands. Missing information remains `None`; it must not be invented.

### `model`

Reads only enough GGUF metadata to support resource accounting and reporting. Model bytes remain local. Parsing should be bounded and reject malformed lengths before allocation.

### `workload`

Describes requested context, output budget, concurrency, and optimization goal. Recommendation quality depends on workload shape; host data alone is insufficient.

### `llama_cpp`

Locates explicit or PATH-provided executables, records version/build output, and detects supported options from that installation. Command construction must use structured arguments rather than shell interpolation.

### `recommendation`

Uses two stages:

1. hard feasibility constraints;
2. profile-specific ranking among feasible candidates.

Every recommendation includes reasons, warnings, and confidence. Policies must remain deterministic for a frozen input snapshot.

### `benchmark`

Runs bounded subprocesses, captures resource measurements, and separates warm-up from measured runs. Later work should record variance so noisy shared VPS results are not presented as precise.

### `report`

Renders human-readable output and versioned JSON from the same domain object. Rendering must not alter recommendation decisions.

## Resource model

Planned memory accounting includes:

- mapped or resident model weights;
- llama.cpp runtime overhead;
- KV cache for requested context and cache types;
- compute and batch buffers;
- per-request cost under concurrency;
- reserved host safety headroom.

A quantized model fitting on disk does not prove it fits safely in memory. A model fitting in memory does not prove useful latency.

## Safety

Default commands remain observational. Any future action that downloads models, launches a persistent server, changes host settings, or quantizes files must be explicit and separately authorized.

Subprocess rules:

- no shell command construction from paths or model metadata;
- bounded runtime and output capture;
- explicit executable discovery;
- graceful termination followed by forced cleanup when required;
- no model contents, paths, or hardware details sent over a network.

## Testing strategy

- Unit tests: parsing, arithmetic boundaries, feasibility rules, scoring, command construction
- Fixture tests: small synthetic GGUF headers, never full model weights
- Integration tests: fake executables with deterministic stdout/stderr and exit codes
- Host tests: opt-in because hardware and performance vary
- Benchmarks: repeated, versioned, and never used as universal claims
