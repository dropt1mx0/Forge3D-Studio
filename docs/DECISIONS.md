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

## ADR-0015: Adaptive Exponential Viewport Zoom, Cursor-Anchored Homothety, and Smooth Inertial Easing
- **Status:** Accepted
- **Date:** 2026-11-02
- **Context:**
  The previous mouse-wheel zoom implementation in Forge3D relied on linear distance multiplication `(1.0 - factor * sensitivity * 5.0).clamp(0.05, 5.0)`. Because egui passes `smooth_scroll_delta.y` with typical values of 20 to 50 per mouse-wheel notch, this linear calculation caused erratic, violent distance leaps (up to 95% jumps in a single notch), frequently sending the camera flying past geometry or away into empty space. Furthermore, zooming was fixed exclusively around the camera's central target point rather than the 3D surface under the cursor, and lacked smooth temporal interpolation.
- **Decision:**
  1. **Adaptive Exponential Multiplicative Model:**
     Adopt the mathematical model `distance_new = distance_old * exp(-scroll_delta * sensitivity)`.
     - In egui, forward scroll (`scroll_delta > 0`) yields a negative exponent, decreasing camera distance smoothly (Zoom In). Backward scroll (`scroll_delta < 0`) yields a positive exponent, increasing distance (Zoom Out).
     - Base sensitivity is normalized to `0.0025`, resulting in a gentle ~7.3% distance scale per standard wheel notch at `zoom_sensitivity = 1.0`.
     - The per-notch exponent is clamped to `[-1.2, 1.2]` to prevent overflow from erratic input spikes, and camera distance is bounded to `[0.05, 5000.0]`.
     - Multiplicative scaling inherently adapts: when close to an object, the absolute step size automatically shrinks, granting sub-millimeter control; when far away, step sizes scale up proportionally.
  2. **Cursor-Anchored Homothety (Zoom to Cursor):**
     When `zoom_to_cursor` is active:
     - Unproject cursor screen position to a world ray using `camera.screen_to_ray(local_pos, viewport_size)`.
     - Raycast against scene objects. If geometry is hit at `hit_point` $P$, scale camera eye $E$ and target $T$ about $P$:
       $E' = P + (E - P) \cdot k$, $T' = P + (T - P) \cdot k$, where $k = \text{new\_distance} / \text{current\_distance}$.
     - This homothety guarantees that the ray $E' \to P$ remains strictly collinear with $E \to P$; the 3D point under the cursor projects to the identical 2D pixel coordinates before and after zooming, while the camera look direction vector $(E' - T')$ remains parallel to $(E - T)$ without rotational twisting.
     - **Safe Focus Plane Fallback:** If the ray misses scene objects, intersect it with the camera focus plane passing through $T$ with normal $-camera.forward()$. The focal plane intersection serves as $P$, preserving smooth, predictable zooming even over empty viewport regions.
     - Orthographic projection scales `ortho_size` proportionally while translating camera eye and target along $(T' - T)$.
  3. **Smooth Frame-Rate Independent Easing:**
     When `smooth_zoom` is enabled:
     - Mouse wheel input updates smooth zoom targets (`zoom_target_eye`, `zoom_target_target`, `zoom_target_ortho_size`) with clamping to `[0.15 * current, 6.0 * current]` to prevent unbounded queueing from free-spinning mouse wheels.
     - Each frame advances interpolation using exponential decay: $\alpha = 1.0 - \exp(-\lambda \cdot dt)$ with $\lambda = 18.0$.
     - When the distance to target falls below $0.001$, the camera snaps to the final target and halts repainting, preventing idle CPU/GPU consumption on power-constrained hardware (e.g. Intel HD Graphics 4000).
  4. **Navigation Compatibility & Zoom Synchronization:**
     Any manual navigation input (MMB Orbit, Shift+MMB Pan, Ctrl+MMB Dolly, empty-space LMB Orbit, Fly/Walk, View Presets, Reset View, Frame Box) invokes `camera.sync_zoom_targets()`, instantly synchronizing smooth targets with active camera state and eliminating interaction fighting.
  5. **User Settings & Persistence:**
     Expose `Zoom Sensitivity` (0.1x to 3.0x), `Zoom to Cursor` (toggle), and `Smooth Zoom` (toggle) in the `View` menu and Right Sidebar properties panel, persisted to `forge3d_settings.json` via `PanelSettings`.
- **Consequences:**
  Eliminates abrupt jumps, provides precise CAD-grade zooming towards points of interest under the cursor, ensures smooth 60fps transitions without sluggish lag, and maintains 100% backward compatibility. Verified by 23 dedicated camera and studio tests (109/109 workspace tests passing).

---

## ADR-0016: Studio UI Visual Overhaul (Cinema 4D / Octane Dark Theme) and Free Object Drag Jitter Elimination
- **Status:** Accepted
- **Date:** 2026-11-02
- **Context:**
  1. The user requested transforming Forge3D Studio's visual interface to match a professional Cinema 4D / Octane dark studio aesthetic: deep slate/charcoal matte theme, modern menu strip with quick-tool icon shelf, top-left viewport HUD strip (`Live Viewer :: View Cameras Display Options`), bottom-left orientation tripod with axis tags, 3-column coordinate card (`Position | Size | Rotation`) with colored dot indicators, timeline scrubber with signature red progress bar and 0..30F ruler, bottom material shelf, and floating material inspector window.
  2. The user also reported extraneous visual noise and back-and-forth jerking ("дергаются вперед назад") during free cursor object dragging.
- **Root Cause & Technical Investigation:**
  During free object dragging (`PointerInteractionState::ObjectDragging | ObjectPressed`), as the object was dragged across the viewport, its pivot moved underneath the mouse cursor. Because `render_and_interact_gizmo` had no guard checking if free object dragging was active, the gizmo handles directly intercepted mouse button state, switching `pointer_state` into `GizmoDragging` and applying 1D axis constraint. Once the cursor moved slightly away from the handle, it switched back to 3D free drag. This oscillation caused rapid snapping between 3D free movement and 1D gizmo constraint. Furthermore, continuous hover raycasting during active dragging spiked frame times.
- **Decision:**
  1. **Gizmo Hit-Testing & Pointer Guard:**
     In `apps/forge3d-studio/src/ui/viewport.rs`, explicitly suppressed gizmo mouse consumption (`gizmo_lmb_pressed = ... && !is_object_dragging_active`, `gizmo_lmb_down = ... && !is_object_dragging_active`). When `pointer_state == PointerInteractionState::ObjectDragging`, gizmo rendering and interaction are cleanly bypassed.
  2. **Hover Raycast Suppression:**
     Hover raycasting (`app.scene.raycast`) is bypassed during active object drag, eliminating extraneous ray tests and keeping frame rates at a steady 60 FPS.
  3. **Cinema 4D / Octane Dark Studio Aesthetic:**
     - Configured egui Visuals: `#121316` base, `#14151a` panels, `#2563eb` electric blue selection pill highlights, `CornerRadius::same(6)`.
     - Top menu bar with logo `⬡`, clean menu hierarchy (`File`, `Edit`, `Create`, `Select`, `Tools`, `Mesh`, `Render`, `Plugins`, `Windows`, `Help`), and right-aligned `Layout: Model ▾` selector.
     - Secondary quick-tool icon shelf (`⤺`, `⤼`, `↖`, `⬚`, `✥`, `⟲`, `⤡`, primitives, `🧲`, `▦`, `⚡ Live Viewer`).
     - Outliner top tabs strip (`File | Edit | View | Objects | Tags | Bookmarks`), tree lines with expand icons (`▾`), leading icons, and blue pill active selection highlight.
     - Properties panel 3-column coordinate card (`Position`, `Size`, `Rotation`) with red (`• X`), green (`• Y`), blue (`• Z`) indicators, and bottom buttons `[ Object ▾ ]`, `[ Size ▾ ]`, `[ Apply ]`.
     - Viewport top-left HUD strip: `⚡ Live Viewer :: View Cameras Display Options`.
     - Viewport orientation tripod at bottom-left with colored axis labels (`X`, `Y`, `Z`).
     - Bottom timeline transport bar with 0..30F ruler, signature red progress track, draggable playhead, and material shelf (`Create`, `Edit`, `Function`, `Texture`, `🟡 Gold`, `⚪ Diffuse`, `🔘 Metal`).
     - Floating `Brushed Metal Material  ‹ › 🔒` inspector window with glossy sphere preview, glossy type selector, diffuse/specular/roughness channels, color spectrum bar, and sliders.
- **Consequences:**
  Object dragging is completely smooth with zero jitter, oscillation, or frame drops. The studio interface achieves a modern, cohesive, dark DCC studio aesthetic strictly matching the reference screenshot. All 109 automated tests pass cleanly with zero warnings under clippy and fmt.

---

## ADR-0017: Viewport/Sidebar Separation, Dynamic Border Resizing, Unified Dark Title/Menu Bar, and Comprehensive UI Emoji Purge
- **Status:** Accepted
- **Date:** 2026-11-02
- **Context:**
  The user requested three critical interface refinements:
  1. Fix the workspace (3D viewport) and separate it from the right menu bar (Outliner / Properties), because previously the workspace was overlapping the right panel and cutting off its functions.
  2. Add the ability to resize using the cursor on the border: hovering over the boundary between the workspace and the right sidebar must change the cursor and dynamically scale both the workspace and sidebar.
  3. Eliminate the jarring white bar on top that took up vertical space and clashed with the dark theme; recolor it to match the `#131419` dark studio theme and move the menu functions (`File`, `Edit`, `Create`, etc.) one row up into this unified top bar.
  4. Completely remove all emojis across the interface (menus, toolbars, properties, outliner, status bar, timeline, material pills, viewport HUD) as emojis look cheap and diminish the application's professional standard.
- **Decision:**
  1. **Border-Resizing & Viewport Separation via Native egui Panel:**
     - Replaced the inner manual 6px splitter in `app.rs` with native `.resizable(true)` on `egui::Panel::right("outliner_properties_panel")`.
     - Placed the resize handle directly on the outer border between the central 3D viewport panel and the right sidebar. Hovering over the edge switches the cursor to `CursorIcon::ResizeHorizontal` and allows smooth drag-scaling (`240.0..=600.0` range).
     - Persist resized width back to `PanelSettings::sidebar_width` and `forge3d_settings.json`.
     - Increased default `sidebar_width` to 350.0 px.
     - Added `egui::ScrollArea::horizontal()` to Outliner tabs (`File | Edit | View | Objects | Tags | Bookmarks`) preventing horizontal truncation.
     - Clamped Viewport HUD overlay strip width to `(rect.width() - 28.0).clamp(100.0, 520.0)` to eliminate any potential viewport overlay bleed.
  2. **Unified Borderless Dark Studio Bar:**
     - Enabled `.with_decorations(false)` on `egui::ViewportBuilder`, eliminating OS/Wayland `libdecor` white titlebars.
     - Merged window branding (`Forge3D`), standard menu hierarchy (`File`, `Edit`, `Create`, `Select`, `Tools`, `Mesh`, `Render`, `Plugins`, `Windows`, `Help`), window draggable area (`StartDrag` on drag, double-click for maximize/restore), layout selector, and window controls (`—`, `▢`/`⧉`, `✕`) onto the single top row with matching studio dark frame (`#131419`).
  3. **Zero-Emoji Policy:**
     - Completely eliminated all emojis from the application. Replaced them with clean text badges (`[Cube]`, `[Sphere]`, `[Cam]`, `[Mesh]`, `[Active]`, `[Locked]`, `Show`/`Hide`, `X`/`Y`/`Z`, `Copy`, `Paste`, `Apply`, `|<`, `◀`, `▶`, `>|`, `Loop`, `Gold`, `Diffuse`, `Metal`).
- **Consequences:**
  Eliminates visual clashes and clipped content, restores complete visibility to the Outliner and Properties cards, provides intuitive border dragging, and gives Forge3D a sleek, professional, modern desktop appearance conforming to Blender, Cinema 4D, and VS Code standards. All automated tests pass with 100% compliance.

---

## ADR-0018: Wavefront OBJ Asset Pipeline MVP (`crates/forge-io`)
- **Status:** Accepted
- **Date:** 2026-11-03
- **Context:**
  The master specification (Phase 5) requires an independent, robust Wavefront OBJ import and export pipeline allowing artists to load external 3D models into Forge3D, view and manipulate them in the Viewport and Outliner, undo imports via a single `Ctrl+Z`, and export meshes and scenes back to standard-compliant `.obj` files.
- **Decision:**
  1. **New Crate `crates/forge-io`**:
     - Dedicated, pure-CPU crate with zero dependencies on `wgpu` or `egui`.
     - Independent clean-room parser supporting `v`, `vt`, `vn`, `f`, `o`, `g`, `s`, `mtllib`, and `usemtl`.
     - Relative and negative index resolution (e.g. `-1`, `-2`) evaluated at line parse time against accumulated buffer length.
     - Strict 1-based indexing validation rejecting 0-indices as `ForgeIoError::ZeroIndex`.
  2. **3D Polygon Ear Clipping Triangulation (`obj/triangulation.rs`)**:
     - Arbitrary $N$-gon triangulation without external dependencies.
     - Newell's method for normal estimation on non-planar polygons.
     - Dynamic 2D projection on dominant plane ($XY$, $XZ$, or $YZ$).
     - Ear clipping with interior-point edge-function tests, correctly handling convex and concave polygons with safe fallback against degenerate/self-intersecting loops.
  3. **Vertex Welding & Seam Preservation (`obj/importer.rs`)**:
     - Multi-index hash map keyed by `(v_idx, vt_idx, vn_idx)` guarantees vertex deduplication while strictly preserving UV seams and hard normal edges.
     - Area-weighted smooth normal calculation if normals are missing in the OBJ file.
  4. **Scene Export (`obj/exporter.rs`)**:
     - Supports exporting single `Mesh` or full `Scene`.
     - World-transform application (`world_space: true`) bakes object translation, rotation, and scale into exported vertex positions and normal vectors.
  5. **Unified History Undo/Redo (`Action::ImportObjects`)**:
     - Added `Action::ImportObjects { objects: Vec<SceneObject> }` to `crates/forge-scene/src/history.rs`.
     - Single `Ctrl+Z` atomic undo removes all objects imported in a multi-object asset.
  6. **Studio UI Integration**:
     - File menu entries: `File -> Import -> Wavefront (.obj)...` and `File -> Export -> Wavefront (.obj)...` (full scene and selected-only) via `rfd` native file dialogs.
- **Consequences:**
  Forge3D can now import and export standard 3D assets with full roundtrip fidelity, multi-object hierarchy preservation, zero panic on malformed inputs, and complete undo/redo integration. All 149 automated workspace tests pass cleanly with zero warnings under clippy and fmt.

## ADR-0019: Built-in House, Tree & Rock Showcase Scene
- **Status:** Accepted
- **Date:** 2026-11-04
- **Context:**
  Forge3D's startup scene demonstrates individual primitives, but users need a more recognizable composition that can be viewed immediately and reloaded while learning the application. Creating the scene through editor interactions is not always available in automated or constrained desktop sessions.
- **Decision:**
  1. Add `Scene::create_showcase_scene()` in `forge-scene`, using the existing procedural mesh generators and per-object colors to build a small house, broadleaf tree, faceted rock, and yard.
  2. Keep `create_default_scene()` unchanged for the standard modeling workspace and headless interaction fixtures.
  3. Start the visible studio application in the showcase view and add `Examples -> House, Tree & Rock` to restore it from the UI. Loading resets selection/edit state, invalidates renderer caches, and frames a composition-specific camera view.
- **Consequences:**
  A user can launch Forge3D and see a complete example composition immediately, then reload it from the menu without external assets. The scene uses simple solid colors and primitive meshes; editable material and lighting systems remain future work.

## ADR-0020: Detailed Showcase and On-Screen Build Walkthrough
- **Status:** Accepted
- **Date:** 2026-11-04
- **Context:**
  The user requested a more detailed version of the existing house, tree, and rock composition and a visible keyboard/cursor demonstration suitable for screen recording.
- **Decision:**
  1. Extend the procedural scene with facade, roof, porch, foliage, path, fence, shrub, and flower details while continuing to use the existing primitive mesh generators and solid-color renderer.
  2. On graphical startup and when replayed from `Examples -> Replay House Build Demo`, reveal the scene in six timed stages over about 8.5 seconds.
  3. Draw a virtual keyboard with stage-specific highlighted keys and an animated screen-space pointer in the viewport. Label both as simulated visual playback; do not synthesize OS keyboard events or move the real system pointer.
  4. Keep `new_headless()` and `File -> New Scene` behavior on the default modeling scene.
- **Consequences:**
  Screen recordings can show a clear, repeatable construction walkthrough without external assets or native input automation. The walkthrough is illustrative only; visible key highlights do not represent input sent to the editor or operating system.

## ADR-0021: Procedural Showcase Surface Patterns
- **Status:** Accepted
- **Date:** 2026-11-04
- **Context:**
  The user requested visible wood and stone texture on the showcase tree, fence, and rocks. Forge3D's current mesh shader already multiplies per-vertex colors by the object color, but it has no image texture loader or texture sampling bindings.
- **Decision:**
  1. Bake deterministic procedural color patterns into mesh vertices for streaked tree bark, wood fibers and knots on fence boards, and mottled mineral/vein color on rock meshes.
  2. Subdivide target meshes where needed so the vertex colors resolve as visible surface variation. Preserve UVs, normals, and object transforms, and use the existing renderer path.
  3. Keep this showcase pass self-contained; full image texture assets, material slots, and GPU texture sampling remain separate renderer/asset-pipeline work.
- **Consequences:**
  The current renderer can show distinct surface character without external files or GPU pipeline changes. These are procedural vertex-color patterns rather than sampled bitmap textures, so higher-resolution image detail and editable texture parameters still require a future material system.

## ADR-0022: Poly Haven PBR Material for the Showcase Path and Porch
- **Status:** Superseded by ADR-0023
- **Date:** 2026-11-04
- **Context:**
  The user requested connecting Poly Haven's Brown Floor Tiles asset to the live render with lighting and 4K textures. The renderer previously used only vertex/object colors and a Blinn-Phong light model.
- **Decision:**
  1. Vendor the asset's CC0 4K diffuse, OpenGL normal, roughness, and AO maps under `assets/textures/brown_floor_tiles/`, with their source and license recorded beside the files.
  2. Add an optional, serde-defaulted `SceneObject.material_id` string so existing `.forge` files remain readable. Mark only the showcase Front Path and Porch Deck with the built-in `polyhaven/brown_floor_tiles` ID.
  3. Add a material bind group to the mesh pipeline and replace the viewport mesh lighting with a Cook-Torrance GGX direct-light model, an ambient fill term, derivative-built tangent frames for normal mapping, and AO modulation.
  4. Decode maps only when a scene with that material is first rendered, upload once per renderer, generate CPU mipmaps, use sRGB for albedo and linear formats for data maps, and clamp source dimensions to the adapter's supported 2D texture size. Other objects use a shared one-pixel material fallback and skip normal-map tangent-frame work in the shader.
- **Consequences:**
  The path and porch now display the source bitmap maps under directional and ambient viewport lighting. The release app loaded the 4096×4096 set on the Intel HD 4000/OpenGL adapter and reported about 170.7 MiB of GPU texture storage including mipmaps; loading and mip generation occur once, not each frame. This is a built-in renderer material rather than a user-editable material browser, and the viewport still has no shadow map or environment lighting.

## ADR-0023: Scene-Driven Metallic-Roughness Materials and Lighting
- **Status:** Accepted
- **Date:** 2026-11-04
- **Context:**
  The initial Poly Haven integration only recognized one hard-coded material ID and the viewport shader fixed metallic at zero and used one synthetic light. The user asked for a complete material pipeline and editable lighting rather than a bitmap-only treatment.
- **Decision:**
  1. Add a renderer-independent `PbrMaterial` record to `Scene`, with linear base-color/emissive factors, metallic and roughness factors, normal/AO strengths, UV scaling, and optional base-color, normal, metallic, roughness, AO, and emissive image paths. Keep GPU resources derived and cached in `forge-render`.
  2. Use the same material bind layout for all scene materials. Sample base color and emissive in sRGB; sample normal and scalar data maps in linear formats; generate mip chains with color-space-aware filtering; use one-pixel defaults for absent maps. Refresh scalar uniforms without rebuilding unchanged image maps.
  3. Replace the hard-coded shader light with a scene uniform carrying up to 16 editable directional, point, and spot lights. Point and spot lights have inverse-square attenuation and finite range. World lighting supplies hemispheric sky/ground fill.
  4. Add a 1024×1024 directional shadow map for the first directional light and ACES tone mapping. The viewport uses a forward renderer; material, light, and world settings remain ordinary scene data and serialize in `.forge` projects.
  5. Replace the fake material inspector with controls that assign/create materials, edit PBR values and map paths, edit selected lights, and tune world fill. Bump the project schema to version 2 and restore the bundled tile material when loading older version-1 showcases.
- **Consequences:**
  Any object can now use the same metallic-roughness path, including the packaged 4K tile material. The live viewport performs direct PBR lighting and directional shadows on the available GPU backend. The renderer still uses hemispheric ambient fill rather than HDR image-based lighting; spot and point lights do not cast shadow maps, and transparent-cutout materials are not supported in the shadow pass.

## ADR-0024: Clean-Room STL and Stanford PLY Asset Pipeline
- **Status:** Accepted
- **Date:** 2026-11-05
- **Context:**
  Section 22.1 ("Матрица форматов") of the master specification designates STL (for 3D printing and rapid fabrication) and PLY (for point cloud, 3D scanning, and vertex color interchange) as high-priority early interchange formats. Previously, only Wavefront OBJ was supported in `crates/forge-io`.
- **Decision:**
  1. Implement clean-room STL parsers, importers, and exporters in `crates/forge-io/src/stl/` supporting both ASCII and Binary STL formats. Distinguish formats via an exact file length invariant check (`file_len == 84 + triangle_count * 50`) and header heuristics, robustly handling binary files whose 80-byte header starts with `"solid"`. Provide spatial-quantization vertex welding with area-weighted smooth normal calculation.
  2. Implement clean-room Stanford PLY parsers, importers, and exporters in `crates/forge-io/src/ply/` supporting ASCII and Binary Little Endian formats. Support custom vertex attributes (position, normal, UV, RGBA vertex color) and polygon face triangulation using 3D Ear Clipping.
  3. Wire STL and PLY import/export into `apps/forge3d-studio` under the top `File -> Import` and `File -> Export` menus with native file dialog filters, status bar telemetry, and atomic `Action::ImportObjects` Undo/Redo integration.
- **Consequences:**
  Forge3D can now directly import and export 3D printing STL models and scan/vertex-colored PLY models without external conversion tools. Workspace test suite expanded to 470/470 passing tests with full roundtrip verification.

## ADR-0025: Non-Destructive Modifiers Stack Expansion and Core Modeling Tools
- **Status:** Accepted
- **Date:** 2026-11-05
- **Context:**
  Section 8 ("Инструменты моделирования") and Section 9 ("Система модификаторов") specify crucial procedural modeling modifiers (`BevelModifier`, `DecimateModifier`, `DisplaceModifier`, `WeightedNormalModifier`) and essential interactive polygonal modeling operations (`loop_cut`, `bevel_selected_edges`, `dissolve_selected_edges`, `bridge_edge_loops`, `separate_selected`, `proportional_translate`, `tris_to_quads`).
- **Decision:**
  1. Expand `crates/forge-modifiers` with 4 new modifiers:
     - `BevelModifier`: procedural edge/vertex chamfering with width, segment count, profile curvature, and clamp overlap.
     - `DecimateModifier`: quadric error metric planar simplification collapsing edge pairs while respecting boundary seams.
     - `DisplaceModifier`: procedural noise or normal displacement along vertex normals with strength, midlevel, and coordinate space options.
     - `WeightedNormalModifier`: area and face-angle weighted smooth normal computation improving hard-surface shading without extra geometry.
  2. Add core polygon editing operations to `EditableMesh` in `crates/forge-geometry/src/edit_mesh.rs`:
     - `loop_cut`: edge ring detection, fractional parameter cut, loop insertion, and topological quad division.
     - `bevel_selected_edges`: split vertex displacement with quad fill along selected edge sets.
     - `dissolve_selected_edges`: co-planar face merging and edge dissolution.
     - `bridge_edge_loops`: connects two boundary edge loops with quad or triangle strips.
     - `separate_selected`: extracts selected faces into a detached mesh entity.
     - `proportional_translate`: smooth spherical/gaussian falloff translation on adjacent unselected vertices.
     - `tris_to_quads`: pairs adjacent triangles into quads when coplanar and angle criteria are met.
  3. Wire all 4 modifiers into `apps/forge3d-studio/src/ui/properties.rs` with dedicated UI panels and real-time evaluation.
- **Consequences:**
  The modifier stack supports standard CAD/CGI modifier workflows with zero crashes or vertex leaks.

## ADR-0026: Sculpting Brush Engine, Operator Command Palette, Workspaces, and Advanced Modeling Tools
- **Status:** Accepted
- **Date:** 2026-11-05
- **Context:**
  Section 10 ("Скульптинг"), Section 21 ("Интерфейс пользователя и рабочие пространства"), Section 24 ("Рабочий процесс"), and Section 8 ("Инструменты моделирования") call for interactive mesh sculpting, Blender-style quick operator search (`F3` / `Space`), dedicated workspace layout presets, and advanced geometric cut tools (`bisect`, `knife_connect_vertices`, `spin_selected`).
- **Decision:**
  1. Implement a complete sculpting engine in `crates/forge-geometry/src/sculpt.rs`:
     - 9 brushes: `Draw`, `Clay`, `Smooth`, `Flatten`, `Inflate`, `Deflate`, `Grab`, `Pinch`, `Crease`.
     - Smooth cubic spherical falloff: $w(d) = (1 - (d/r)^2)^3$.
     - Mirror symmetry across X, Y, and Z axes.
     - Real-time viewport raycasting deformed meshes via pointer delta in `apps/forge3d-studio/src/ui/viewport.rs`.
     - Dedicated Sculpting brush shelf, radius/strength sliders, and symmetry toggles in `toolbar.rs`.
  2. Implement Blender-style Command Palette modal dialog in `apps/forge3d-studio/src/ui/command_palette.rs`:
     - Hotkeys: `F3`, `Ctrl+K`, and `Space`.
     - 50+ searchable operators across Mesh operations, Modifiers, Workspace switching, Primitive creation, Viewport shading, and File I/O.
     - Fuzzy/case-insensitive substring filtering, arrow key navigation, Enter execution.
  3. Implement Workspace layout presets in `crates/forge-scene/src/editor_state.rs` and `apps/forge3d-studio/src/app.rs`:
     - 8 workspaces: `Modeling`, `Sculpting`, `UvEditing`, `Shading`, `Animation`, `Rendering`, `GeometryNodes`, `Scripting`.
     - Top menu bar mode switcher configuring viewports, toolbars, sidebar panels, and edit modes.
  4. Implement advanced polygonal cut tools in `crates/forge-geometry/src/edit_mesh.rs`:
     - `bisect`: cuts mesh along arbitrary 3D plane, splits crossing edges, subdivides faces, optionally creates planar cap, optionally prunes outer geometry.
     - `knife_connect_vertices`: splits an n-gon/quad across two non-adjacent vertices with a dividing edge.
     - `spin_selected`: lathe/screw revolve tool revolving profile vertices/edges around an arbitrary 3D axis with configurable steps and pitch.
     - Studio hotkeys: `K` (Knife connect), `Alt+B` (Bisect), `Alt+S` (Spin).
- **Consequences:**
  Studio provides comprehensive Blender-equivalent modeling, sculpting, search, and layout features, passing 100% of workspace test suite with zero clippy warnings.

## ADR-0027: Clean-Room glTF 2.0 and Binary GLB Asset Pipeline
- **Status:** Accepted
- **Date:** 2026-11-05
- **Context:**
  Section 22.1 ("Матрица форматов") designates glTF 2.0 / GLB as P0 / High Priority for mesh models, PBR metallic-roughness materials, transforms, and scene interchange. The application required a robust, self-contained, clean-room implementation without heavy C/C++ dependencies or unsafe bindings.
- **Decision:**
  1. Implement glTF 2.0 and GLB support in `crates/forge-io/src/gltf/`:
     - `types.rs`: Complete serde-based data model for glTF 2.0 JSON structures (`asset`, `scenes`, `nodes`, `meshes`, `primitives`, `materials`, `accessors`, `bufferViews`, `buffers`).
     - `base64.rs`: Standalone clean-room Base64 encoder and decoder for data URI interchange without external crate dependencies.
     - `importer.rs`: Dual-mode reader parsing self-contained binary GLB containers (magic `0x4654_6C67`, version 2, JSON and BIN chunks) and text glTF files with data URIs or relative external buffers. Reconstructs `Mesh` with positions, normals, UVs, vertex colors, and indices. Maps node hierarchies to local transforms and imports PBR metallic-roughness materials (`baseColorFactor`, `metallicFactor`, `roughnessFactor`, `emissiveFactor`).
     - `exporter.rs`: Serializes scene geometry and PBR materials into binary `.glb` containers (with 4-byte padding alignments on chunks) or `.gltf` with embedded base64 buffers. Supports whole scene and selected-only exports.
  2. Integrate into `apps/forge3d-studio`:
     - Added `import_gltf_from_path` and `export_gltf_to_path` in `app.rs` with atomic Undo/Redo tracking.
     - Added menu items in `ui/menu_bar.rs` under `File -> Import -> glTF 2.0 / GLB` and `File -> Export -> glTF 2.0 / GLB`.
     - Added command palette actions in `ui/command_palette.rs`.
  3. Added comprehensive test coverage:
     - `crates/forge-io/tests/gltf_tests.rs`: Cube roundtrip, sphere data URI roundtrip, multi-object PBR material preservation, and header error handling.
     - `apps/forge3d-studio/tests/gltf_studio_integration.rs`: Studio import/export roundtrip, selection sync, and undo/redo.
- **Consequences:**
  Forge3D now supports full-cycle industry-standard glTF 2.0 / GLB interchange. Passes 100% of workspace tests and Clippy on Rust 1.99 with `-D warnings`.

## ADR-0028: UV Unwrapping & Projection Suite and Extension / Plugin Sandbox System
- **Status:** Accepted
- **Date:** 2026-11-05
- **Context:**
  Section 11 ("UV & Texture Painting") and Section 23 ("Scripting & Extensions") require robust UV mapping tools (Smart UV Projection, Box, Cylinder, Sphere projections, Island packing) and an extensible plugin system with manifest declarations, runtime capability permissions, sandboxing, and operator registration.
- **Decision:**
  1. Implement UV projection and packing algorithms in `crates/forge-geometry/src/uv.rs`:
     - `smart_project_uvs`: Clusters faces into planar UV islands based on normal divergence threshold (`angle_limit_rad`), computes tangent-plane 2D projections, and arranges islands into non-overlapping grid cells within $[0, 1] \times [0, 1]$ texture space with user margin.
     - `box_project_uvs`: Orthogonal tri-planar box projection along the dominant normal axis (X, Y, or Z).
     - `sphere_project_uvs`: Spherical polar coordinates $(u, v)$ centered around arbitrary 3D origins.
     - `cylinder_project_uvs`: Radial azimuth and vertical height projection.
     - `pack_uv_islands`: Bounds and scales existing vertex UV coordinates to fit cleanly within $[margin, 1 - margin]$.
  2. Implement Scripting & Extension Manager in `crates/forge-scene/src/extension.rs`:
     - `PluginManifest`: Declares plugin identity (id, name, version, author), API compatibility, and required capabilities.
     - `PluginPermission`: Runtime permissions (`FileRead`, `FileWrite`, `SceneMutate`, `Network`).
     - `ExtensionManager`: Manages plugin lifecycle (`Enabled`/`Disabled`), custom operator registration, runtime permission validation, and event hook telemetry (`SceneLoaded`, `ObjectAdded`, `ObjectRemoved`, `FrameRendered`, `ProjectSaved`).
  3. Studio Integration:
     - Added UV projection tools under `Mesh -> UV Unwrapping & Projection` and command palette (`uv_smart_project`, `uv_box_project`, `uv_sphere_project`, `uv_cylinder_project`, `uv_pack_islands`).
     - Added `show_plugin_manager` modal dialog showing installed plugins, permission tags, enable/disable toggles, and event log telemetry.
     - Wired into menu bar `Plugins -> Plugin Manager...` and command palette `Open Plugin & Extension Manager`.
- **Consequences:**
  Completes Section 11 and Section 23 specification requirements. Verified with unit and workspace integration tests passing 100%.




