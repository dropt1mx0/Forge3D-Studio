# Agent Guidance for Forge3D Rust Edition

This repository is governed by the principles in `FORGE3D_MASTER_SPEC.md` and the Engineering Skill Pack in `.agents/skills/`.

## Key Directives:
1. **Source of Truth**: `FORGE3D_MASTER_SPEC.md` defines requirements, roadmap, and boundaries.
2. **Autonomous Execution**: Make sound engineering choices independently, document them in `docs/DECISIONS.md`, and maintain state in `docs/SESSION_STATE.md`.
3. **Clean-Room**: Strict independent implementation. No copied code or assets.
4. **Safety Boundaries**: No `sudo`, no network publishing/pushing, no destructive actions outside the workspace.
5. **Quality Gates**: Always run `cargo fmt`, `cargo check`, `cargo test`, and `cargo clippy` and record real results.
