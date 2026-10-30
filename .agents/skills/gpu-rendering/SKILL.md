---
name: gpu-rendering
description: "Use when working specifically on GPU compute, WGSL, shader development, buffer layouts, bind groups, texture formats, GPU synchronization, compute pipelines, or GPU debugging."
---

# GPU Rendering and Compute Skill

## Mission
Implement GPU work with explicit data layouts, feature checks, resource lifetime rules, and shader validation.

## WGSL discipline
- Keep shader source organized and version-controlled.
- Document Rust/WGSL struct layout assumptions.
- Validate alignment and padding.
- Keep binding indices and visibility synchronized.
- Prefer small shaders and isolated validation examples.
- Surface compilation errors with useful context.

## Compute
Before adding a compute pipeline, define:
- Input/output buffers
- Workgroup dimensions
- Dispatch dimensions
- Bounds checks
- Synchronization requirements
- Readback strategy
- CPU fallback if required

Never assume workgroup sizes or optional features are universal.

## Resource management
- Avoid unnecessary map/readback operations.
- Reuse allocations where safe.
- Track resource ownership and destruction.
- Handle device loss and reinitialization where supported.
- Avoid creating pipelines or buffers repeatedly in frame loops.

## Debugging
- Enable validation/debug labels in development builds.
- Use GPU captures/profilers available on the target platform.
- Reduce failing shaders to minimal reproductions.
- Test at least one fallback path where feasible.

## Completion checklist
- [ ] Buffer layout verified
- [ ] Bounds checked
- [ ] Feature limits queried
- [ ] Shader errors surfaced
- [ ] Resource lifecycle reviewed
- [ ] GPU/CPU synchronization minimized
