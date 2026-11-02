# Changelog

All notable changes to Forge3D Rust Edition will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3-alpha.5] - 2026-11-02

### Added
- **Studio Visual Redesign (Cinema 4D / Octane Dark Aesthetic)**:
  - Theme: Custom dark studio palette (`#121316` background, `#14151a` panels, `#22252e` widgets, `#2563eb` electric blue selection highlights, `CornerRadius::same(6)`).
  - Menu Bar: Top strip with logo `⬡`, standard hierarchy (`File`, `Edit`, `Create`, `Select`, `Tools`, `Mesh`, `Render`, `Plugins`, `Windows`, `Help`), and right-aligned `Layout: Model ▾` selector.
  - Quick-Tool Shelf: Secondary horizontal icon bar (`⤺`, `⤼`, `↖`, `⬚`, `✥`, `⟲`, `⤡`, primitive additions, `🧲`, `▦`, `⚡ Live Viewer`).
  - Outliner Tabs & Tree: Category tabs (`File | Edit | View | Objects | Tags | Bookmarks`), tree lines with expand icons (`▾`), leading icons, and rounded blue pill selection highlight (`#2563eb`).
  - Cinema 4D Coordinate Card: 3-column `Position`, `Size`, `Rotation` block with red (`• X`), green (`• Y`), blue (`• Z`) axis dots, and bottom buttons `[ Object ▾ ]`, `[ Size ▾ ]`, `[ Apply ]`.
  - Viewport HUD: Top-left overlay bar `⚡ Live Viewer :: View Cameras Display Options`.
  - Viewport Orientation Tripod: Positioned in bottom-left with colored letters `X`, `Y`, `Z`.
  - Timeline & Scrubbing: 0..30F ruler with green active frame marker, signature red progress track with draggable playhead, transport controls (`⏮`, `◀`, `▶`, `⏭`, `🔁`), and bottom material shelf (`Create`, `Edit`, `Function`, `Texture`, `🟡 Gold`, `⚪ Diffuse`, `🔘 Metal`).
  - Floating Material Inspector Window: `Brushed Metal Material  ‹ › 🔒` dialog with glossy sphere preview, glossy type selector, diffuse/specular/roughness channels, rainbow hue spectrum bar, and sliders.

### Fixed
- **Object Drag Jitter & Oscillation ("дергаются вперед назад")**:
  - Identified root cause: during active free cursor dragging, as the object's pivot moved under the mouse cursor, gizmo handles intercepted mouse input and took over into `GizmoDragging`, applying 1D axis constraint. Moving away from the handle switched back to 3D free drag, causing rapid jittering and oscillation.
  - Suppressed gizmo hit-testing and pointer consumption whenever `PointerInteractionState::ObjectDragging` or `ObjectPressed` is active.
  - Completely bypassed gizmo rendering during free dragging.
  - Suppressed hover raycasts during active dragging, eliminating CPU frame time spikes and ensuring silky 60 FPS movement.

## [0.5.2-alpha.4] - 2026-11-02

### Added
- **Adaptive Smooth Viewport Zoom**:
  - Replaced erratic linear zoom with exponential multiplicative model: `distance_new = distance_old * exp(-scroll_delta * sensitivity)`.
  - Scaled base sensitivity factor (`BASE_ZOOM_SENSITIVITY = 0.0025`), eliminating violent camera jumps and providing ~7.3% distance change per standard wheel notch.
  - Automatic adaptive step scaling: millimeter-fine sensitivity up close, fast navigation when far away.
  - Safe distance bounds clamping (`[0.05, 5000.0]`) and per-notch exponent clamping (`[-1.2, 1.2]`), preventing penetration through geometry, camera flipping, or NaN/infinity divergence.
- **Zoom to Cursor (Cursor-Anchored Homothety)**:
  - Cursor position in screen space unprojects to a 3D ray (`camera.screen_to_ray`).
  - Raycasts against scene geometry; if hit at point $P$, camera eye and target scale around $P$: $E' = P + (E - P) \cdot k$, $T' = P + (T - P) \cdot k$.
  - Collinear ray invariant preserves exact 2D screen coordinates of the point under cursor before and after zoom without camera rolling or yaw shifts.
  - Safe fallback: if raycast misses scene objects, intersects the camera focus plane passing through target (`plane_normal = -camera.forward()`), ensuring smooth navigation across empty viewport regions.
  - Full support for both Perspective and Orthographic projection modes.
- **Smooth Inertial Zoom**:
  - Time-based exponential interpolation ($\alpha = 1.0 - \exp(-18.0 \cdot dt)$) with frame delta awareness.
  - Anti-queueing guard clamps target distances to `[0.15 * current, 6.0 * current]`, preventing accumulation from free-spinning mouse wheels.
  - Snaps to target when delta $< 0.001$, halting repaints and conserving GPU/CPU on power-constrained hardware (Intel HD Graphics 4000).
- **Navigation Compatibility & Synchronization**:
  - Manual navigation (MMB Orbit, Shift+MMB Pan, Ctrl+MMB Dolly, empty-space LMB Orbit, Fly/Walk, View Presets, Reset View, Frame Box) instantly invokes `camera.sync_zoom_targets()`, ensuring zero conflict.
- **User Settings & Persistence**:
  - Added configurable `Zoom Sensitivity` slider (0.1x to 3.0x), `Zoom to Cursor` checkbox, and `Smooth Zoom` checkbox in the `View` menu and Properties panel.
  - Settings persist across application launches in `forge3d_settings.json`.
- **Testing & Verification**:
  - 15 new unit and integration tests (6 in `forge-render::camera`, 9 in `adaptive_smooth_zoom.rs`).
  - 109/109 automated tests passing across the workspace.

## [0.5.1-alpha.3] - 2026-11-01

### Added
- **Free Cursor Object Dragging (Free Drag)**:
  - Direct LMB click-and-hold on 3D geometry initiates free world-space dragging following the mouse cursor without requiring axis selection.
  - Camera-aligned working plane dynamically constructed through the initial grab point with normal opposing camera forward (`-camera.forward().normalize()`), delivering intuitive 1:1 screen-to-world mapping.
  - Grab point offset retention (`initial_offset = obj.translation - hit_point`) ensuring zero object jumps or teleportation upon drag initiation.
  - Multi-object synchronous drag: dragging any selected object applies the exact same delta to all selected objects, preserving group configuration.
  - Single atomic undo command (`Action::MultiTransform`) committed to the history stack upon mouse release (`Ctrl+Z` reverts full drag in one step).
  - Strict input priority protection: UI Interaction (`egui_wants_pointer_input`) > Gizmo Handles (`gizmo_consumed`) > Object Drag > Selection > Box Selection > Viewport Orbit. No accidental drag on UI or Gizmo clicks.
  - Robust edge case handling: safe parallel ray rejection (`|denom| < 1e-4`), distance clamp ($t \le 10000.0$), delta displacement clamp ($|\Delta| \le 5000.0$), focus loss restoration, and `Escape` key rollback.
  - 12 automated unit and integration tests in `apps/forge3d-studio/tests/free_cursor_drag.rs` (92/92 workspace tests passing).

## [0.5.0-alpha.2] - 2026-10-31

### Added
- **Automatic Object Interaction & Unified Gizmo System**:
  - Automatically display Transform Gizmo immediately upon selecting any object or multiple objects.
  - Multi-axis & multi-plane gizmo support: 3 translation/scale axes, 3 planar handles (XY, XZ, YZ), and 3 rotation rings.
  - Automatic rebinding when clicking another object.
  - Pivot Point support: Median Center, Active Element, 3D Cursor, Individual Origins.
  - Transform Orientation support: Global and Local coordinates.
  - Multi-selection transform with compound undo action (`Action::MultiTransform`).
- **Keyboard Transform System**:
  - Direct G (Move), R (Rotate), S (Scale) modal transform activation.
  - X, Y, Z axis constraints and Shift+X, Shift+Y, Shift+Z planar constraints.
  - Real-time precision numeric input with backspace and decimal point support.
  - Enter / LMB to confirm, Esc / RMB to cancel, Ctrl+Z Undo, Ctrl+Shift+Z Redo.
  - Automatic cancellation on window focus loss.
  - Full hotkey prioritization: typing in UI text fields suppresses viewport modal hotkeys.
- **Resizable Workspace & Splitters**:
  - Draggable splitters with hover/drag visual feedback (orange accent line `#f09020`, horizontal and vertical resize cursors).
  - Left Toolbar: resizable (48–240 px), Icon Only vs Icon + Label modes, and collapse toggle.
  - Right Sidebar: resizable (220–600 px) with adjustable Outliner vs Properties vertical split ratio (0.15–0.85).
  - Bottom Timeline panel: resizable (60–350 px) with transport controls and visibility toggle.
  - Clamped limits and session persistence via `forge3d_settings.json`.
- **Categorized Toolbar**:
  - Grouped into Selection, Transform, Modeling, Add Mesh, Utilities.
  - Tooltips, icons, and keyboard shortcuts for all active tools.
- **Add Object System Expansion**:
  - Added procedural clean-room Capsule primitive (`generate_capsule`) and Monkey / Suzanne mascot head (`generate_monkey`) with validated outward normals and UV coordinates.
  - Searchable Add Menu with category groupings (`Mesh Primitives`, `Scene Objects`) and dynamic query filtering.
- **Automated Integration Test Suite**:
  - Added `tests/alpha_0_2_ux_interaction.rs` with 7 end-to-end integration tests covering gizmo auto-display, multi-object transformation, modal constraints with numeric input, focus loss safety, panel persistence, and primitive generation (80/80 tests passing workspace-wide).

## [0.4.2-alpha.6] - 2026-10-31

### Added
- **Master Project Audit & Traceability Matrix**:
  - Authored comprehensive `docs/FEATURE_MATRIX.md` mapping all 31 specification sections to the codebase with granular status, priority, dependencies, acceptance criteria, test coverage, and known limitations.
  - Formulated the 10-phase master development roadmap in `docs/ROADMAP.md` covering P0–P4 priority tiers and clear phase exit criteria.
  - Documented ADR-0012 in `docs/DECISIONS.md` establishing architectural crate boundaries, clean-room standards, and next implementation cycles.
  - Measured and published real-hardware performance baselines in `docs/BENCHMARKS.md` on Intel Core i5 Ivy Bridge + Intel HD Graphics 4000 under Linux (~603 ms startup, 110 MB idle RAM, 60 FPS viewport, 71/71 tests passing).

## [0.4.1-alpha.5] - 2026-10-30

### Added
- **Left Mouse Viewport Navigation (LMB Orbit Navigation)**:
  - Users can now orbit the 3D viewport camera by pressing and dragging LMB over empty viewport space.
  - Reuses existing camera orbit mathematics without code duplication, preserving orbit pivot target and camera distance while enforcing $-89.4^\circ$ to $+89.4^\circ$ pitch limits to prevent camera flipping.
  - Supports both Perspective and Orthographic projection modes.
  - Existing Middle Mouse Button (MMB) Orbit, Shift+MMB Pan, Scroll Zoom, Alt+LMB Orbit, and Fly Navigation remain completely unaffected.
- **Pointer Interaction State Machine**:
  - Implemented `PointerInteractionState` (`Idle`, `ObjectPressed`, `GizmoDragging`, `BoxSelecting`, `ViewportOrbiting`, `UIInteraction`).
  - Strict input priority hierarchy: UI Panels > Active Gizmos > Object Selection > Box Select > Empty Viewport Navigation.
  - Disambiguated empty-space clicking and dragging: static clicks ($< 4.0$ px displacement) clear active selection, whereas drags ($\ge 4.0$ px) orbit the camera without clearing selection.
  - Safety checks for pointer release outside the viewport and application focus loss to prevent stuck dragging/orbiting states.
- **Viewport Debug Telemetry**:
  - Integrated floating monospace telemetry card displaying Pointer Interaction State, Pointer Position, Viewport Bounds, Camera Mode, Orbit Target Pivot, and Camera Distance.
  - Added "Show Viewport Debug Telemetry" toggle checkbox in the View menu.
- **Dedicated Automated Integration Tests**:
  - Added `apps/forge3d-studio/tests/lmb_orbit_interaction.rs` testing all 8 required pointer interaction scenarios (71/71 tests passing workspace-wide).

## [0.4.0-alpha.4] - 2026-10-30

### Fixed
- **Critical Interaction Pipeline Disconnect**:
  - Fixed pointer sampling in `apps/forge3d-studio/src/ui/viewport.rs` by replacing `interact_pos()` with `hover_pos().or(latest_pos())`, restoring cursor tracking, gizmo highlighting, mouse-look, and scroll zoom when mouse buttons are not held down.
  - Resolved modal transform confirmation click collision where confirming a Grab/Rotate/Scale via LMB caused an immediate deselection or object reassignment in the same frame.
  - Fixed zero-thickness AABB raycast failure in `crates/forge-math/src/bounds.rs` with minimum slab thickness padding ($\epsilon = 0.005$) and parallel ray containment checks, making flat planes and 2D ground meshes reliably selectable.
  - Fixed gizmo single-frame undo bug in `apps/forge3d-studio/src/ui/gizmo.rs` by caching the drag start transform upon initial mouse capture and committing the full drag delta to the undo history stack upon release.

### Added
- **Selection Tools & Box Selection**:
  - Rubber-band 2D Box Selection (выделение рамкой) in Select tool mode (`B` shortcut) with live visual bounding rectangle.
  - Global selection commands: Select All (`A`), Deselect All (`Alt+A`), Invert Selection (`Ctrl+I`).
- **Automated Interaction Flow Test Suite**:
  - Added headless testing harness in `apps/forge3d-studio` (`Forge3dStudioApp::new_headless()`).
  - Added 8 end-to-end integration tests simulating real user event sequences:
    - Object picking and raycast selection.
    - Modal move (`G` -> `X` -> `5` -> `Enter` -> `Undo` -> `Redo`).
    - Modal rotate (`R` -> `Y` -> `90` -> `Enter` -> `Undo`).
    - Modal scale (`S` -> `2` -> `Enter` -> `Undo`).
    - Camera navigation (Orbit, Pan, Zoom, Presets, Ortho toggle, Reset).
    - Selection tools (Select All, Deselect All, Invert, Box Select).
    - Scene camera object manipulation and view alignment.
    - Project serialization / persistence roundtrip.

## [0.3.0-alpha.3] - 2026-10-30

### Added
- **Full Viewport Camera Navigation System**:
  - Orbit, Pan, and Dolly Zoom with pitch clamping and configurable sensitivity.
  - Fly and Walk Navigation (`Shift+F` or View menu): WASD translation, Q/E vertical translation, Shift boost, Ctrl slow-down, mouse look, and walk height ground lock.
  - Standard Numpad views: Front (1), Back (Ctrl+1), Right (3), Left (Ctrl+3), Top (7), Bottom (Ctrl+7), Perspective/Orthographic toggle (5), Camera View (0), 15° step rotation (2/4/6/8), and Emulate Numpad option for compact keyboards.
  - Framing: Frame Selected (`F` / Numpad `.`) and Frame All (`Home`).
- **First-Class Scene Camera Objects**:
  - `PrimitiveType::Camera` object in scene hierarchy with procedural 3D camera marker mesh (body, tapered lens frustum, upward indicator).
  - Physical optical properties: Perspective & Orthographic projection, synchronized Focal Length (mm) and FOV (deg), Sensor Size (36×24mm default), near/far clipping, ortho scale, Depth of Field settings, and resolution presets (1080p, 4K, 1:1).
  - Camera View overlay with film gate / safe area letterboxing.
  - Align Active Camera to View (`Ctrl+Alt+Num0` and inspector button).
  - Active camera switching via Outliner, Viewport, and Properties Inspector.
- **Modal Transformations & Keyboard Workflow**:
  - Modal Grab/Move (`G`), Rotate (`R`), Scale (`S`) finite state machine.
  - Axis locks (`X`, `Y`, `Z`) and plane locks (`Shift+X`, `Shift+Y`, `Shift+Z`).
  - Repeat axis press toggles Local vs Global coordinate constraint.
  - Exact real-time numeric input (digits, `-`, `.`, Backspace) with live viewport HUD status readout.
  - Confirm with `Enter` / `LMB`, cancel with `Escape` / `RMB` (restores original transforms).
  - Multi-object transformation support with Pivot Points: Median Point, Active Element, 3D Cursor, Individual Origins.
  - Object operations: Duplicate (`Shift+D`), Delete (`X`/`Del`), Hide (`H`), Unhide All (`Alt+H`), Reset Transforms (`Alt+G/R/S`), Apply Transform (`Ctrl+A`), Parent/Unparent (`Ctrl+P`/`Alt+P`), Join Objects (`Ctrl+J`), and Set Origin (to Geometry, to 3D Cursor).
- **Interactive 3D Gizmos & Snapping**:
  - Interactive Move (arrows + planes), Rotate (rings), and Scale (boxes + uniform) manipulators.
  - Grid snap integration (`🧲`) with configurable increments.
- **Input Architecture & Priority Routing**:
  - Isolated keyboard routing preventing hotkey leakage during text edits (`egui_wants_keyboard_input`).
  - Hierarchical priority: Text Input > Fly Navigation > Modal Transform > 3D Gizmos > Viewport Navigation > Hotkeys.
- **Inspector & Outliner Enhancements**:
  - Properties panel: Per-axis translation, rotation, and scale locks (`🔒`/`🔓`) and reset buttons (`↺`), Dimensions editing, Copy/Paste transform buttons, and dedicated Camera Optics section.
  - Outliner: Camera icon (`📷`), active camera indicator (`★`), transform lock toggles (`🔒`/`🔓`), hierarchy tree indentation with child relationships, and multi-select support (`Ctrl+Click`).

## [0.2.0-alpha.2] - 2026-10-30

### Added
- **Interactive Viewport & Camera System (Sprint 1)**:
  - Input-first frame pipeline eliminating 1-frame rendering latency.
  - Orbit (MMB / Alt+LMB), Pan (Shift+MMB), Dolly Zoom (Ctrl+MMB), and Zoom (Scroll wheel).
  - Perspective and Orthographic projection toggle (Numpad 5).
  - Standard viewpoints: Front (Numpad 1), Back (Ctrl+1), Right (Numpad 3), Left (Ctrl+3), Top (Numpad 7), Bottom (Ctrl+7).
  - Frame Selected (`F` / Numpad `.`) and Frame All (`Home`).
  - 3D-to-2D screen projection (`Camera::world_to_screen`) and screen-to-ray unprojection for both camera modes.
- **Interactive 3D Transform Gizmos (Sprint 2)**:
  - Interactive on-screen viewport gizmos: Move (arrows + planes), Rotate (rings), and Scale (boxes + uniform).
  - 3D ray-line closest point calculation (`Ray::closest_point_on_line`) and 3D ray-plane intersection (`Ray::intersect_plane`).
  - Real-time mouse dragging with live visual feedback, hover highlight, and grid snapping (`🧲`).
  - Transform shortcuts: `Q` (Select), `W` (Move), `E` (Rotate), `R` (Scale), `Alt+G` (Reset Pos), `Alt+R` (Reset Rot), `Alt+S` (Reset Scale).
  - Transform Orientation (Global vs Local) and Pivot Point (Median, Active, Cursor) options.
- **Modeling Foundation & Edit Mode (Sprint 3)**:
  - Mode switching between Object Mode and Edit Mode via `Tab` key.
  - Pure-CPU `EditableMesh` (B-Rep polygon mesh) data structure in `forge-geometry`.
  - Vertex Select (`1`), Edge Select (`2`), and Face Select (`3`) modes with single and Shift multi-selection.
  - Core modeling tools: Extrude (`E`), Inset (`I`), Subdivide, Merge at center (`M`), Fill (`F`), Delete (`Del`/`X`).
  - Viewport wireframe and vertex point overlays in Edit Mode with interactive centroid gizmo.
- **Editor Workflow, Modifiers & Primitives (Sprint 4)**:
  - Command pattern Undo / Redo system (`HistoryStack` in `forge-scene`) supporting Transform, Add, Delete, ModifyMesh, and ApplyTransform (`Ctrl+Z`, `Ctrl+Shift+Z`).
  - New `forge-modifiers` crate providing non-destructive `ModifierStack`:
    - Mirror (X, Y, Z axes with outward normal preservation).
    - Subdivision Surface (procedural subdivision levels 1..=4).
    - Solidify (wall thickness for open surfaces).
    - Array (repetitive geometry with 3D offset).
  - Properties panel modifier controls: enable toggles, sliders, reordering (`▲`/`▼`), delete, and "Apply to Mesh" baking.
  - Expanded procedural primitives: Cube, UV Sphere, Icosphere, Plane, Circle, Cylinder, Cone, Torus.
- **Quality Gates**:
  - Full workspace test suite expanded to **40/40 tests passing**.
  - `cargo fmt --check` cleanly enforced.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings.
  - Release build verified (`target/release/forge3d-studio`).

## [0.1.0-alpha.1] - 2026-10-30

### Added
- **Core Engine Architecture**:
  - `forge-core`: Strongly typed, generational `ObjectId` and `AssetId`, domain traits (`Identifiable`, `Named`), and error definitions (`CoreError`).
  - `forge-math`: 3D linear algebra (glam-based), coordinate conventions (Right-Handed, Y-up, distance in meters), affine `Transform` with parent-child matrix composition and Euler angle degrees conversion, `Aabb` bounding boxes, and 3D `Ray` with ray-plane, ray-sphere, and Möller–Trumbore triangle intersection algorithms.
  - `forge-geometry`: Pure-CPU mesh data representation with explicit WGSL-aligned vertex buffer layout (position, normal, uv, color), procedural geometry generators (`generate_cube`, `generate_uv_sphere`, `generate_plane`, `generate_grid_lines`), normal calculation, and mesh validation invariants (no NaNs, no out-of-bounds indices, no degenerate triangles).
  - `forge-scene`: Scene hierarchy management, `SceneObject`, selection state tracking, transient `EditorState`, and `.forge` project serialization schema v1.
  - `forge-gpu`: `wgpu` (v30.0.1) device, queue, and surface wrappers, typed `GpuBuffer` abstractions, and runtime adapter capability discovery (verified on hardware Intel HD Graphics 4000).
  - `forge-render`: Viewport rendering pipeline, perspective `Camera` with Orbit, Pan, and Zoom controls, screen-to-ray unprojection for object picking, WGSL shaders with Blinn-Phong directional and ambient lighting, depth testing (`Depth32Float`), ground grid, and selection highlight feedback.
  - `forge3d-studio`: Native desktop application built on `eframe` (v0.37) with custom dark graphite UI, menu bar, left tool palette, central 3D viewport with offscreen WGPU texture integration, scene outliner, transform inspector, and bottom status bar reporting GPU adapter name, FPS, frame time, and geometry metrics.
