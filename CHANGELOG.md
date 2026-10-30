# Changelog

All notable changes to Forge3D Rust Edition will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
