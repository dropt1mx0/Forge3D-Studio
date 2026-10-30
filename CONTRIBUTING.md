# Contributing to Forge3D Rust Edition

Thank you for your interest in contributing to Forge3D!

## Principles
1. **Clean-Room Implementation**: Forge3D is an independent implementation. Do not copy code, assets, or text from Blender or other closed-source/GPL applications.
2. **Correctness Before Speed**: All geometry and math algorithms must have unit tests verifying invariants and edge cases (degenerate polygons, zero vectors, non-finite values).
3. **Rust Quality Gates**: All contributions must pass:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets`
4. **No `.unwrap()` or `.expect()` in Production Code**: Handle errors gracefully and surface diagnostics to the user.
5. **No Renderer Leakage**: `forge-core`, `forge-math`, `forge-geometry`, and `forge-scene` must never depend on `wgpu` or UI libraries.
