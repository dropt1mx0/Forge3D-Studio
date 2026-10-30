---
name: graphics-programming
description: "Use when working on graphics APIs, GPU resources, wgpu, Vulkan concepts, WGSL shaders, render passes, buffers, textures, synchronization, or cross-platform rendering in Forge3D."
---

# Graphics Programming Skill

## Mission
Build a portable, testable graphics layer with explicit resource ownership and predictable frame execution.

## Workflow
1. Inspect the current graphics backend and supported target platforms.
2. Confirm API/version details from official documentation before using unfamiliar APIs.
3. Define resource lifetime, ownership, synchronization, and fallback behavior.
4. Implement a minimal vertical slice before generalizing.
5. Validate shader compilation and test on available backends.
6. Record hardware/backend limitations honestly.

## Architecture
- Keep GPU implementation details inside `forge-gpu` / `forge-render`.
- Keep scene and geometry crates independent of wgpu handles.
- Separate CPU scene data from GPU-resident representations.
- Define explicit upload/update paths and invalidation rules.
- Use render passes and resource layouts consistently.
- Make resize, device loss, surface reconfiguration, and shutdown paths explicit.

## Performance
- Minimize unnecessary buffer uploads and GPU/CPU readbacks.
- Batch compatible draws; use instancing where appropriate.
- Reuse buffers and textures when safe.
- Avoid per-frame allocations in hot paths.
- Profile CPU submission time and GPU execution separately.
- Do not assume more threads or larger batches always improve performance.

## WGSL and shaders
- Keep shader interfaces synchronized with Rust-side bind-group layouts.
- Document coordinate spaces, matrix conventions, color spaces, units, and precision assumptions.
- Add small shader validation examples.
- Provide graceful diagnostics for compilation and pipeline creation failures.

## Cross-platform
- Treat wgpu as an abstraction, not a guarantee of identical performance or feature support.
- Query adapter features and limits at runtime.
- Implement fallbacks for optional capabilities.
- Never claim ray tracing, mesh shaders, or a specific GPU feature without checking backend and hardware support.

## Completion checklist
- [ ] Resource lifetimes are explicit
- [ ] Resize and failure paths handled
- [ ] Shader interfaces validated
- [ ] CPU and GPU costs considered
- [ ] Backend limitations documented
- [ ] No renderer types leaked into domain crates
