---
name: code-review
description: "Use when reviewing Forge3D changes for correctness, Rust safety, architecture, performance, security, maintainability, licensing, and test coverage."
---

# Code Review Skill

## Mission
Find consequential defects before changes are accepted. Be specific, evidence-based, and constructive.

## Review order
1. Correctness and data loss
2. Memory safety and concurrency
3. Security and trust boundaries
4. Architecture and API compatibility
5. Performance and resource usage
6. Error handling and observability
7. Tests and documentation
8. Style and maintainability

## Rust-specific checks
- Ownership and lifetime assumptions
- `unsafe` blocks and invariants
- Lock ordering and deadlocks
- Async cancellation and blocking work
- Panic paths
- Unbounded queues or allocations
- Incorrect Send/Sync assumptions
- Resource cleanup

## Graphics-specific checks
- GPU resource lifetime
- Buffer layout/alignment
- Shader binding mismatch
- Out-of-bounds indexing
- Device feature assumptions
- Unnecessary readbacks or uploads
- Incorrect coordinate/color-space assumptions

## Review output
Classify findings:
- Critical: data loss, security breach, memory unsafety, severe corruption
- High: major functional failure or crash
- Medium: incorrect behavior or significant performance regression
- Low: maintainability or minor issue

For every finding include:
- File and location
- Why it is a problem
- Reproduction or reasoning
- Recommended correction

Do not invent findings. If no material issues are found, state the scope reviewed and remaining uncertainty.

## Completion checklist
- [ ] Diff fully inspected
- [ ] Tests and claims verified
- [ ] Risky paths reviewed
- [ ] Findings prioritized
- [ ] Scope and limitations stated
