# Roadmap

## Phase 0 — Scaffold

- [x] Define CLI surface
- [x] Define serializable domain contracts
- [x] Add architecture and safety boundaries
- [x] Add formatting, lint, test, build, and MSRV checks
- [x] Implement first runtime behavior

## Phase 1 — Read-only inspection

- [x] Linux CPU topology and instruction features
- [x] Host, cgroup-aware RAM, and swap snapshot
- [x] Known accelerator-vendor discovery without vendor SDKs
- [x] Storage characteristics for a selected path
- [ ] Installed `llama.cpp` executable, version, and supported-option discovery
- [x] Versioned JSON host report

## Phase 2 — GGUF and feasibility

- [ ] Bounded GGUF metadata reader
- [ ] Model size and architecture report
- [ ] KV-cache and runtime memory estimator
- [ ] Explicit host safety headroom
- [ ] Hard rejection reasons for configurations that cannot fit

## Phase 3 — Recommendation policy

- [ ] Memory-saver profile
- [ ] Balanced profile
- [ ] Quality-first profile
- [ ] Candidate command preview
- [ ] Explanation and confidence for every recommendation

## Phase 4 — Measured optimization

- [ ] `llama-bench` adapter
- [ ] Warm-up and repeated measured runs
- [ ] Cold-load versus warm-load metrics
- [ ] Peak resident memory and I/O metrics
- [ ] Variance and noisy-host warning
- [ ] Fastest-measured profile

## Later

- Optional perplexity or task-specific quality evaluation
- Additional llama.cpp backends and platforms
- Explicit, opt-in quantization workflow
- Signed release binaries

Each behavior should arrive through a failing test, minimal implementation, and documented report-schema change where applicable.
