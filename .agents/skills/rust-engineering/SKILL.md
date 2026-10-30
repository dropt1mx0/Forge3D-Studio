---
name: rust-engineering
description: "Use when designing, implementing, reviewing, or debugging Rust code, Cargo workspaces, APIs, ownership, lifetimes, concurrency, error handling, or dependency choices for Forge3D."
---

# Rust Engineering Skill

## Mission
Produce idiomatic, maintainable, safe Rust for Forge3D. Prefer clear ownership and explicit boundaries over clever abstractions.

## Workflow
1. Inspect the relevant crate, workspace conventions, toolchain, and existing tests before editing.
2. Identify public API and dependency impacts.
3. Write a short implementation plan for non-trivial changes.
4. Implement the smallest coherent change.
5. Run `cargo fmt --check`, targeted tests, `cargo check`, and `cargo clippy` where feasible.
6. Report commands actually run and their real results.

## Engineering rules
- Prefer stable Rust unless a documented feature requires otherwise.
- Avoid `unsafe`. If unavoidable, isolate it, document invariants and safety contracts, and add focused tests.
- Avoid `.unwrap()` and `.expect()` in production paths; propagate typed errors with context.
- Use `thiserror` for library/domain errors and `anyhow` at application boundaries when appropriate.
- Do not clone large buffers or scene data without measuring and explaining why.
- Prefer borrowing, slices, iterators, and explicit ownership transfer.
- Keep modules cohesive; avoid giant files and circular crate dependencies.
- Avoid premature generic frameworks and unnecessary trait indirection.
- Use `Arc`/`Mutex`/`RwLock` only when shared ownership or synchronization is required; document contention risks.
- Never block async executor threads with long CPU work. Use dedicated workers or a CPU pool.
- Do not introduce dependencies without checking maintenance, platform support, license, and compile-time cost.

## Cargo workspace
- Keep crate responsibilities narrow and documented.
- Put shared dependency versions in workspace dependencies where useful.
- Avoid wildcard dependency versions.
- Keep feature flags intentional and test important feature combinations.
- Do not silently change MSRV or toolchain requirements.

## API design
- Prefer explicit types over loosely typed strings and unstructured maps.
- Keep public APIs small and stable.
- Document invariants, units, coordinate conventions, threading expectations, and error cases.
- Avoid exposing renderer-specific types from geometry or scene core crates.

## Completion checklist
- [ ] Existing conventions inspected
- [ ] Ownership and threading reviewed
- [ ] Error paths handled
- [ ] Tests added or updated
- [ ] Formatting and linting checked
- [ ] Dependency/license impact reviewed
- [ ] No unsupported completion claims
