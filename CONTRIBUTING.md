# Contributing

OptiMod has an early read-only Linux inspection implementation.

## Development loop

1. Open an issue describing one observable behavior.
2. Add a focused failing test and confirm the expected failure.
3. Add the smallest implementation that passes it.
4. Run all quality checks.
5. Keep unrelated refactors in separate changes.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

## Constraints

- Preserve read-only defaults.
- Do not add telemetry or implicit network access.
- Never interpolate user-controlled paths into a shell command.
- Keep estimated and measured recommendations distinguishable.
- Add synthetic metadata fixtures, not model weights.
- Document any report-schema change.
