# Architectural Decisions Log (ADR)

## ADR-0001: Initial Architecture and Workspace Modularization for Alpha 0.1
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Alpha 0.1 requires a reliable desktop 3D application with a 3D viewport, perspective camera navigation (orbit, pan, zoom), grid/axes, basic lighting, primitive mesh generation (cube, sphere, plane), object selection, and transform manipulation (translation, rotation, scaling).
  Per the master specification, domain logic (geometry, math, scene) must remain strictly independent of renderer/UI dependencies, and no empty crates should be created.
- **Decision:**
  Establish a Cargo workspace with the following active crates for Alpha 0.1:
  1. `forge-core`: Foundational identifiers (`ObjectId`, `Transform`), common error types, traits.
  2. `forge-math`: 3D mathematics leveraging `glam` (Vec3, Quat, Mat4, AABB, Ray, coordinate conversions). Right-handed, Y-up convention.
  3. `forge-geometry`: Pure-CPU mesh topology, procedural primitive generators (cube, UV sphere, plane, grid), vertex/index buffers, normal calculation, triangulation, mesh validation invariants (no NaNs, no degenerates).
  4. `forge-scene`: Scene hierarchy, scene objects, selection state, persistent vs transient editor state, dirty flags.
  5. `forge-gpu`: wgpu device, queue, surface abstraction, runtime GPU adapter discovery and capability logging.
  6. `forge-render`: Viewport rendering pipeline, WGSL shaders, perspective camera uniforms, depth buffer, grid and coordinate axes passes, lighting (directional sun + ambient), selection outlines/visual feedback.
  7. `apps/forge3d-studio`: Native desktop application combining the wgpu viewport and egui dark graphite UI layout (menu bar, tools, viewport, outliner, properties, status bar).
- **Consequences:**
  - Strict decoupling: `forge-geometry` and `forge-scene` can be tested purely on CPU in headless environments.
  - Safe GPU resource management: wgpu handles stay within `forge-gpu` and `forge-render`.
  - Maintainability: Easy to extend in Alpha 0.2 (mesh edit mode) without architectural rework.

---

## ADR-0002: Graphics Framework & UI Integration
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  The development machine has an Intel 3rd Gen Core processor (HD Graphics 4000) running Linux (Wayland / X11) with Mesa drivers (`hasvk` Vulkan and Gallium/Crocus OpenGL).
  We need a cross-platform graphics foundation that reliably supports both modern Vulkan/Metal/DX12 and falls back gracefully to OpenGL/GLES if Vulkan capabilities are restricted.
- **Decision:**
  - Use `wgpu` with `Backends::all()` to discover the best available hardware adapter at runtime (Vulkan primary, GLES fallback).
  - Use `eframe` (0.37) / `egui` for the dark graphite native UI. Custom wgpu rendering will render directly to texture/viewport callback using wgpu render passes.
  - Runtime adapter logging: surface the exact backend, driver name, and limits in the UI status bar.
- **Consequences:**
  - Guarantees windowing and DPI scaling work seamlessly on both Wayland and X11.
  - No decorative fake controls: every UI panel and button corresponds to real scene and viewport actions.

---

## ADR-0003: Viewport Input Processing and Rendering Synchronization
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  In Alpha 0.1, camera and viewport interactions suffered from a 1-frame latency and frame dropping because egui rendering consumed inputs after offscreen wgpu texture dispatch.
- **Decision:**
  Restructure the top-level application loop in `forge3d-studio`:
  1. Capture all pointer events, keyboard shortcuts, and viewport bounds *before* drawing UI panels.
  2. Directly mutate camera and active gizmo state using delta time and mouse delta.
  3. Perform wgpu render pass to the viewport texture.
  4. Present the updated texture inside egui's central panel with zero latency.
- **Consequences:**
  Eliminates input lag, jitter, and frame stalling during orbit, pan, zoom, and gizmo dragging.

---

## ADR-0004: B-Rep `EditableMesh` and Non-Destructive Modifiers Separation
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Mesh modeling requires topology inspection, boundary loop traversal, polygon extrusion, and non-destructive modifier evaluation (Mirror, Subdivision, Solidify, Array).
  Triangulated GPU meshes (`forge_geometry::Mesh`) are inadequate for polygon editing.
- **Decision:**
  - Introduce `EditableMesh` in `crates/forge-geometry/src/edit_mesh.rs` with `EditVertex`, `EditEdge`, and `EditFace` (arbitrary n-gons with winding orders).
  - Implement conversion between `Mesh` and `EditableMesh`.
  - Isolate modifiers into `crates/forge-modifiers`, operating as pure functions `fn apply(&self, mesh: &Mesh) -> Result<Mesh, ModifierError>`.
  - In `SceneObject`, maintain the base mesh as authoritative and expose `evaluated_mesh()` which executes `ModifierStack::evaluate(&self.mesh)`.
- **Consequences:**
  Base mesh topology remains pure and uncorrupted during modifier adjustments; baking is an explicit one-click action.

---

## ADR-0005: Command Pattern and Undo/Redo System
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Professional 3D modeling demands robust undo/redo for object transforms, mesh edits, primitive additions, and deletions.
- **Decision:**
  - Implement `HistoryStack` in `crates/forge-scene/src/history.rs` supporting `Action::Transform`, `Action::MeshEdit`, `Action::AddObject`, and `Action::DeleteObject`.
  - Undo restores preceding transforms and mesh snapshots; Redo reapplies them.
  - Hotkeys `Ctrl+Z` (Undo) and `Ctrl+Shift+Z` / `Ctrl+Y` (Redo) are integrated globally into the studio application.
- **Consequences:**
  Provides confidence to artists during complex modeling workflows; completely decoupled from UI code.
