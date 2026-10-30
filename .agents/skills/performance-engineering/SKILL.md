---
name: performance-engineering
description: "Use when profiling or optimizing Forge3D CPU, memory, GPU, startup, scene loading, geometry operations, frame latency, or large-scene performance."
---

# Performance Engineering Skill

## Mission
Improve measured performance without sacrificing correctness, portability, or maintainability.

## Golden rule
Never claim a speedup without a reproducible benchmark. Rust does not automatically make a program faster than C, C++, Java, or an existing optimized implementation.

## Workflow
1. Define the user-visible performance problem.
2. Establish a baseline on documented hardware and software.
3. Choose representative workloads.
4. Profile to locate the bottleneck.
5. Form one measurable hypothesis.
6. Change one major variable at a time.
7. Re-run correctness tests and benchmarks.
8. Report results, variance, and limitations.

## Metrics
- Cold and warm startup time
- Idle and peak RSS
- CPU utilization
- GPU utilization and memory where measurable
- Frame time and frame-time percentiles
- Throughput
- P50/P95/P99 latency
- Scene load/save duration
- Allocation count/volume where tooling permits

## Benchmark hygiene
- Record CPU, GPU, RAM, OS, driver, compiler, build profile, feature flags, and scene fixture.
- Compare equivalent quality settings and workloads.
- Separate debug and release builds.
- Warm up where appropriate.
- Repeat runs and report variance.
- Do not compare unrelated tasks or cherry-pick results.

## Optimization candidates
- Data layout and cache locality
- Allocation reduction
- Incremental updates
- Parallel task scheduling
- GPU batching and instancing
- Culling
- Asset streaming
- Serialization and decompression
- Lock contention

## Safety
Do not trade memory safety or data integrity for speculative speed. Any unsafe optimization requires documented invariants, focused tests, and measured benefit.

## Completion checklist
- [ ] Baseline recorded
- [ ] Bottleneck identified by evidence
- [ ] Workload reproducible
- [ ] Correctness preserved
- [ ] Before/after results recorded
- [ ] Trade-offs documented
