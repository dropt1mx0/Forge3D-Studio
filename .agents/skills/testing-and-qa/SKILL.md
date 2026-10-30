---
name: testing-and-qa
description: "Use when planning or implementing unit, integration, property-based, regression, serialization, graphics, performance, or cross-platform tests for Forge3D."
---

# Testing and QA Skill

## Mission
Prevent regressions and verify that Forge3D features work beyond the happy path.

## Test layers
- Unit tests for pure logic and geometry.
- Integration tests for crate boundaries and workflows.
- Property-based tests for math and geometry invariants where useful.
- Golden/reference tests for serialization and deterministic outputs.
- GPU tests with capability-aware skips and clear reasons.
- End-to-end smoke tests for application startup and basic editing.
- Benchmarks for performance-critical paths.

## Test design
- Test invalid inputs and failure paths.
- Test empty, minimal, large, and degenerate scenes.
- Test save/load round trips.
- Test version migration.
- Test undo/redo sequences.
- Test asset import failures and unsupported formats.
- Keep fixtures small and deterministic.

## Commands
Use project-supported commands, typically:
- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`

Run narrower commands during iteration and full checks at milestones.

## GPU testing
- Do not assume a GPU exists in CI.
- Separate CPU-testable logic from GPU integration.
- Use software adapters only where supported and clearly label them.
- Never mark skipped GPU tests as passed functionality.

## Reporting
Report exact commands, pass/fail/skip counts, and known limitations. Do not claim tests ran if they did not.

## Completion checklist
- [ ] Happy path covered
- [ ] Failure cases covered
- [ ] Regression test added
- [ ] Serialization compatibility considered
- [ ] GPU skips explained
- [ ] Exact test results reported
