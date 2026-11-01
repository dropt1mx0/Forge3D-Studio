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

---

## ADR-0006: Scene Camera as Pure Domain Structure with Procedural 3D Marker
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  A 3D application requires persistent scene cameras that can be positioned, rotated, parented, and calibrated with realistic optics (focal length, sensor size, clipping, depth of field), distinct from the working viewport navigation camera.
- **Decision:**
  - Add `PrimitiveType::Camera` and `CameraData` to `forge-scene::object`.
  - Generate a procedural 3D camera marker mesh (`generate_camera_marker()` in `forge-geometry::primitives`) consisting of an equipment body, a tapered wire frustum lens, and an orange upward-pointing triangle.
  - Calculate camera viewing matrices through pure-CPU `CameraViewParameters` in `forge-scene`, avoiding any dependencies on graphics crates.
  - Render an on-screen film gate / safe area overlay in the viewport matching the camera's aspect ratio and resolution settings.
- **Consequences:**
  Clean-room implementation with zero asset dependencies. Camera objects participate seamlessly in scene graphs, transforms, gizmos, and `.forge` project serialization.

---

## ADR-0007: Modal Transformation Finite State Machine with Precision Numeric Input
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Professional 3D software (such as Blender) relies heavily on fast modal keyboard transformations (`G` Grab, `R` Rotate, `S` Scale) with axis constraints (`X`, `Y`, `Z`), plane constraints (`Shift+X/Y/Z`), and typed numbers (`G X 5 Enter`).
- **Decision:**
  - Implement `ModalTransformState` in `forge-scene::editor_state` tracking transform mode, axis constraint, space (global vs. local), start pointer position, pivot point, and a typed numeric string buffer.
  - Multi-select transform: Apply offsets to all selected objects around configured Pivot Points (Median Point, Active Element, 3D Cursor, Individual Origins).
  - Toggling: Pressing the same axis key a second time toggles constraint between Global and Local coordinates; a third press clears the constraint.
  - Atomic confirmation / cancellation: Enter or Left Click commits the transform to `HistoryStack`; Escape or Right Click reverts all objects to their initial transforms.
- **Consequences:**
  High-speed keyboard workflow matching professional expectations with zero ambiguity and full cancellation safety.

---

## ADR-0008: Centralized Event Routing Priority and Text Input Isolation
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Single-key hotkeys (such as `G`, `R`, `S`, `X`, `F`, `H`) easily collide with typing text in inspector fields, outliner renames, or fly navigation inputs.
- **Decision:**
  - Establish a strict priority chain:
    1. UI Text Focus (`egui_wants_keyboard_input`): Blocks all viewport shortcut processing.
    2. Fly / Walk Navigation: WASDQE and mouse look take precedence over object transforms.
    3. Modal Transformation: Number keys, axis keys, Enter, and Escape are consumed by the modal state machine.
    4. 3D Gizmo Dragging: Consumes mouse clicks, preventing accidental viewport orbiting.
    5. Viewport Navigation: MMB / Alt-drag, Shift-drag, and wheel scroll.
    6. Global Shortcuts: Fall-through when all above are idle.
- **Consequences:**
  Completely eliminates inadvertent object modifications while typing text, and prevents navigation conflicts during transforms.

---

## ADR-0009: Decoupling Pointer Hover vs Drag State and Confirmation Click Consumption
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  `ui.input(|i| i.pointer.interact_pos())` was used for sampling mouse coordinates in the viewport. Because egui only sets `interact_pos` when a mouse button is actively held down, hover highlights, scroll zooming, and keyboard-driven modal transformations received `None` on every frame where the mouse was not pressed. Additionally, when a user clicked LMB to confirm a modal transform, `modal.active` transitioned to `false` on that frame, causing downstream object selection logic to treat the click as an object picking event.
- **Decision:**
  - Sample mouse position via `ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()))`.
  - Introduce an explicit `modal_consumed_click = true` flag upon modal confirmation or cancellation via mouse click, preventing raycast selection on that frame.
- **Consequences:**
  Ensures smooth hover highlights, reliable mouse scroll wheel zooming, and guarantees that confirming an operation never unexpectedly deselects or reselects objects in the scene.

---

## ADR-0010: Inflation of Degenerate Slabs in AABB Raycasting
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Flat 2D geometry (such as the Ground Plane where $min.y = max.y = 0.0$) produced ray intersection failures. The standard slab formula $t_1 = (min.y - orig.y) / dir.y$ and $t_2 = (max.y - orig.y) / dir.y$ resulted in $t_{min} > t_{max}$ due to floating-point rounding errors, and division by zero ($NaN$) for rays parallel to the plane.
- **Decision:**
  Enforce a minimum half-thickness $\epsilon = 0.005$ along any degenerate dimension ($max_i - min_i < \epsilon$) in `Aabb::intersect_ray`, accompanied by an explicit parallel ray containment check.
- **Consequences:**
  Eliminates selection glitches on planar geometry, ground grids, and thin panels, ensuring fast and robust raycasting broadphase tests.

---

## ADR-0011: LMB Orbit Viewport Navigation and Pointer Interaction State Machine
- **Status:** Accepted
- **Date:** 2026-10-30
- **Context:**
  Users expect natural 3D viewport navigation where dragging LMB over empty space rotates (orbits) the camera around the active pivot point. However, this must never interfere with existing interactions: UI panel clicks, active gizmo drags, object raycast picking, or box selection. Furthermore, a normal click on empty space must continue to clear the current selection.
- **Decision:**
  1. Define `PointerInteractionState` in `forge-scene`: `Idle`, `ObjectPressed`, `GizmoDragging`, `BoxSelecting`, `ViewportOrbiting`, and `UIInteraction`.
  2. Implement an input priority hierarchy on pointer press:
     - UI panels / widgets (`egui_wants_pointer_input()`) -> `UIInteraction`.
     - Gizmo handles -> `GizmoDragging`.
     - Box Select tool -> `BoxSelecting`.
     - Scene object raycast hit -> `ObjectPressed` (selects object).
     - Empty space -> `ViewportOrbiting`.
  3. Reuse existing camera math (`Camera::orbit(yaw, pitch)`) ensuring pivot point and distance are preserved, and vertical pitch is clamped between $-89.4^\circ$ and $+89.4^\circ$.
  4. Disambiguate empty-space clicking and dragging: record `pointer_press_pos`; on pointer release, if Euclidean displacement is $< 4.0$ px, clear selection; if $\ge 4.0$ px, keep selection and complete orbit.
  5. Add safety reset: on focus loss or release outside the window, reset `pointer_state` to `Idle`.
- **Consequences:**
  Provides an intuitive and responsive camera orbit interaction over empty viewport space while preserving all existing gizmo transformations, object selection, box selection, MMB orbit, and Shift+MMB pan.

---

## ADR-0012: Master Project Audit & Strategic Roadmap Alignment
- **Status:** Accepted
- **Date:** 2026-10-31
- **Context:**
  A comprehensive technical audit was conducted against the master specification (`FORGE3D_MASTER_SPEC.md` and the 31 production domains). The project has successfully established a high-performance, rock-solid Phase 1 foundation (71/71 tests passing, <600ms startup, ~110MB idle RAM, 60 FPS viewport on Intel HD 4000 legacy hardware). We must define the architectural strategy for progressing into Phase 2 (Polygonal Modeling) and Phase 4 (Materials, Lighting, and Asset I/O).
- **Decision:**
  1. **Preserve and extend existing crates**: Retain `forge-core`, `forge-math`, `forge-geometry`, `forge-scene`, `forge-gpu`, `forge-render`, `forge-modifiers`, and `apps/forge3d-studio`.
  2. **Introduce `crates/forge-io`**: Dedicated crate for standard 3D asset interchange (Wavefront OBJ, glTF 2.0 / GLB, STL). It depends on `forge-core`, `forge-math`, `forge-geometry`, and `forge-scene`, keeping file parsing isolated from rendering and UI.
  3. **Establish clean-room asset and parsing boundaries**: No proprietary libraries; use standard open specifications and pure Rust parsers with comprehensive round-trip tests.
  4. **Next Implementation Cycle Selection**: Execute the immediate core modeling enhancement (Loop Cut & Slide and Bevel foundation in `forge-geometry`) and Asset I/O foundation (`forge-io`) to enable importing and refining real-world assets without external DCC tools.
- **Consequences:**
  Maintains clean crate boundaries, guarantees zero regression in existing tested interactions, and systematically addresses P0/P1 gaps identified in `docs/FEATURE_MATRIX.md`.

## ADR-0013: Unified Interaction, Delta-Based Multi-Object Gizmo, and Resizable Workspace UX (Alpha 0.2)
- **Status:** Accepted
- **Date:** 2026-10-31
- **Context:**
  In Alpha 0.1, object manipulation required manually selecting tools on the left toolbar before interacting with objects. Users needed to select an object and immediately begin manipulating it with a unified transform gizmo or keyboard hotkeys (G, R, S) without toolbar friction. Furthermore, panels were fixed in width, lacking resizability, collapsibility, and persistent workspace settings.
- **Decision:**
  1. **Automatic Gizmo Activation & Rebinding:** When any object is selected (or multi-selection exists), the transform gizmo is automatically displayed in the viewport. Switching selection immediately rebinds the gizmo to the new target.
  2. **Unified Gizmo Delta Interaction (`GizmoInteractionResult`):** The gizmo system calculates 3D transform deltas (`GizmoDelta::Translation`, `GizmoDelta::Rotation`, `GizmoDelta::Scale`) and emits lifecycle events (`drag_started`, `drag_ended`). Multi-object transformations apply deltas to all selected objects according to active `PivotPoint` (Median, Active, Cursor, Individual) and `TransformOrientation` (Global, Local), recording compound history undo actions (`Action::MultiTransform`).
  3. **Modal Transform State Machine Integration:** Direct G/R/S modal transforms integrate seamlessly with gizmo mode, supporting X/Y/Z single-axis locks, Shift+X/Y/Z planar locks, precision numeric keyboard typing, and window focus loss safety. Hotkeys are strictly disabled when egui text inputs have focus.
  4. **Resizable Workspace & Splitter Architecture:** Left Toolbar, Right Sidebar, Outliner/Inspector vertical ratio, and Bottom Timeline panel feature draggable splitters with `#f09020` accent lines and cursor feedback (`ResizeHorizontal`/`ResizeVertical`). Toolbar supports `IconOnly`, `IconAndLabel`, and collapse. Panel sizes are clamped and saved across sessions to `forge3d_settings.json`.
  5. **Clean-room Primitive Expansion:** Added `Capsule` and `Monkey` (Suzanne procedural mascot) with outward normals and UV coordinates, validated through mesh topology checks.
- **Consequences:**
  Eliminates friction in object interaction. Users can select and manipulate immediately. Workspace layout is customizable and persists across launches. All 80 automated unit and integration tests pass cleanly with zero warnings under clippy.

## ADR-0014: Free Cursor Object Dragging via Camera-Aligned Ray-Plane Intersection and Grab Offset Retention
- **Status:** Accepted
- **Date:** 2026-11-01
- **Context:**
  Previously in Forge3D, translating objects required interacting with the Gizmo's 1D axis arrows or 2D planar quads, or pressing `G` for modal movement. Direct, unconstrained click-and-drag translation directly on object geometry was missing. Users require a free drag interaction mode where clicking and holding on any object geometry translates the object freely across the viewport in world space, tracking the cursor naturally without axis restriction, without jumping, and with support for multi-object groups and unified Undo/Redo.
- **Decision:**
  1. **Camera-Aligned World Working Plane:**
     On mouse press on geometry (`hit_point` via raycast), establish a working plane passing through `hit_point` whose normal opposes the camera's forward vector (`plane_normal = -camera.forward().normalize()`). This plane is parallel to the camera sensor / viewport plane, providing intuitive 1:1 screen-to-world mapping without perspective skewing across the field of view.
  2. **Grab Point Offset Retention (Zero-Jump Guarantee):**
     At drag start, the initial ray through the cursor intersects the working plane at exactly `initial_hit_point`. The initial offset between the hit point and each object's translation is recorded: `initial_offset = obj.local_transform.translation - hit_point`. On subsequent mouse frames, the current camera ray intersects the plane at `current_hit`, computing `delta = current_hit - initial_hit_point`. Object translations are updated as `obj.translation = initial_translation + delta = current_hit + initial_offset`. When drag starts, `delta == 0`, ensuring zero jump.
  3. **Multi-Object Synchronous Drag:**
     When multiple objects are selected, dragging any selected object applies the exact same world-space `delta` to all selected objects simultaneously, maintaining their exact relative distances and configuration. Upon mouse release, the group translation is recorded as a single atomic `Action::MultiTransform` in the undo history stack.
  4. **Input Priority Hierarchy Protection:**
     Enforce strict input priority: (1) UI Interaction (`egui_wants_pointer_input()`), (2) Gizmo Interaction (`gizmo_consumed`), (3) Object Drag, (4) Object Selection, (5) Box Selection, (6) Viewport Navigation. When the pointer is over UI or gizmo handles, object drag is completely inhibited.
  5. **Edge Case & Numeric Robustness:**
     Implement `intersect_ray_plane` and `calculate_drag_delta` with comprehensive safety checks:
     - Finite checks on ray origin, ray direction, plane point, and normal.
     - Rejection of rays nearly parallel to the plane (`|denom| < 1e-4`).
     - Rejection of rays directed backwards or behind the camera (`t <= 0.0`).
     - Range bounds checking (`t <= 10000.0`, `|delta| <= 5000.0`) preventing coordinate explosion near viewport horizons.
     - Window focus loss and `Escape` key cancellation restoring initial object transforms cleanly.
- **Consequences:**
  Provides a fluid, responsive DCC-standard direct manipulation experience. Preserves existing Gizmo, modal transform, and camera navigation workflows without regressions. Verified by 12 dedicated automated tests and 92/92 total passing tests across the workspace.
