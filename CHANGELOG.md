# Changelog

All notable changes to Forge3D Rust Edition will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.1] - 2026-10-30

### Added
- **Core Engine Architecture**:
  - `forge-core`: Strongly typed, generational `ObjectId` and `AssetId`, domain traits (`Identifiable`, `Named`), and error definitions (`CoreError`).
  - `forge-math`: 3D linear algebra (glam-based), coordinate conventions (Right-Handed, Y-up, distance in meters), affine `Transform` with parent-child matrix composition and Euler angle degrees conversion, `Aabb` bounding boxes, and 3D `Ray` with ray-plane, ray-sphere, and Möller–Trumbore triangle intersection algorithms.
  - `forge-geometry`: Pure-CPU mesh data representation with explicit WGSL-aligned vertex buffer layout (position, normal, uv, color), procedural geometry generators (`generate_cube`, `generate_uv_sphere`, `generate_plane`, `generate_grid_lines`), normal calculation, and mesh validation invariants (no NaNs, no out-of-bounds indices, no degenerate triangles).
  - `forge-scene`: Scene hierarchy management, `SceneObject`, selection state tracking, transient `EditorState`, and `.forge` project serialization schema v1.
  - `forge-gpu`: `wgpu` (v30.0.1) device, queue, and surface wrappers, typed `GpuBuffer` abstractions, and runtime adapter capability discovery (verified on hardware Intel HD Graphics 4000).
  - `forge-render`: Viewport rendering pipeline, perspective `Camera` with Orbit, Pan, and Zoom controls, screen-to-ray unprojection for object picking, WGSL shaders with Blinn-Phong directional and ambient lighting, depth testing (`Depth32Float`), ground grid, and selection highlight feedback.
  - `forge3d-studio`: Native desktop application built on `eframe` (v0.37) with custom dark graphite UI, menu bar (New, Open, Save, Exit, Add, View, Help), left tool palette, central 3D viewport with offscreen WGPU texture integration, scene outliner, transform inspector, and bottom status bar reporting GPU adapter name, FPS, frame time, and geometry metrics.
- **Verification & Quality**:
  - Full test suite passing across all crates (22/22 tests).
  - `cargo fmt --check` cleanly enforced.
  - `cargo clippy --workspace --all-targets -- -D warnings` cleanly passed with zero warnings.
