# Forge3D Rust Edition — Tasks

## Alpha 0.1 Tasks — Foundation Milestone
- [x] Workspace Reconnaissance & Toolchain
- [x] Architecture & Modular Cargo Workspace (`forge-core`, `forge-math`, `forge-geometry`, `forge-scene`, `forge-gpu`, `forge-render`, `apps/forge3d-studio`)
- [x] Native window & WGPU integration on Intel HD Graphics 4000
- [x] Baseline unit tests and quality gates

---

## Core Interaction & Modeling Overhaul (Sprints 1–4)

### Sprint 1: Viewport & Camera Overhaul (P0)
- [x] Fix input pipeline execution order (process inputs before offscreen render to eliminate 1-frame latency)
- [x] Direct MMB / Alt+LMB drag event tracking with continuous repaint requests
- [x] Camera controls: Orbit, Pan, Zoom, Dolly
- [x] Perspective and Orthographic projection toggle (Numpad 5)
- [x] Standard viewpoints: Front (Numpad 1), Back (Ctrl+1), Right (Numpad 3), Left (Ctrl+3), Top (Numpad 7), Bottom (Ctrl+7)
- [x] Frame Selected (`F` / Numpad `.`) and Frame All (`Home`)
- [x] Screen-to-ray world projection for both Perspective and Orthographic modes
- [x] 3D-to-2D world-to-screen coordinate projection (`Camera::world_to_screen`)

### Sprint 2: Object Transforms & Interactive Gizmos (P0)
- [x] Interactive 3D on-screen gizmo in viewport:
  - [x] Move: 3 colored arrows (X red, Y green, Z blue) + 3 planar handles (XY, XZ, YZ)
  - [x] Rotate: 3 rotation arcs/rings around X, Y, Z axes
  - [x] Scale: 3 axis boxes + central uniform scale box
- [x] Mathematical ray-line closest point calculation (`Ray::closest_point_on_line`)
- [x] Mathematical ray-plane intersection (`Ray::intersect_plane`)
- [x] Live hover highlighting (bright yellow) and real-time transform feedback
- [x] Hotkeys: `Q` (Select), `W` (Move), `E` (Rotate), `R` (Scale)
- [x] Apply Transform (bake into vertices) & Reset Transform (Alt+G, Alt+R, Alt+S)
- [x] Transform Orientation (Global vs Local) and Pivot Point options

### Sprint 3: Modeling Foundation & Edit Mode (P1)
- [x] Pure-CPU `EditableMesh` (B-Rep polygon mesh) data structure in `forge-geometry`
- [x] Mode separation: Object Mode vs Edit Mode (`Tab` key)
- [x] Selection elements: Vertex (`1`), Edge (`2`), Face (`3`)
- [x] Modeling operations:
  - [x] Extrude (`E`): Extrudes faces along normal creating quad boundary walls
  - [x] Inset (`I`): Scales inner polygon towards centroid with border quad strips
  - [x] Subdivide: Splits quads into 4 quads and triangles into 4 triangles
  - [x] Merge at Center (`M`): Merges selected vertices at centroid and cleans degeneracies
  - [x] Fill (`F`): Constructs face or edge from selected vertices
  - [x] Delete (`Del` / `X`): Deletes vertices, edges, or faces with topological integrity
- [x] Viewport wireframe and vertex point overlays in Edit Mode
- [x] Selection centroid gizmo for moving, rotating, and scaling mesh components

### Sprint 4: Editor Workflow, Modifiers & Primitives (P1)
- [x] Command-based Undo / Redo system (`HistoryStack` in `forge-scene`) supporting Transform, Add, Delete, ModifyMesh, and ApplyTransform (`Ctrl+Z`, `Ctrl+Shift+Z`)
- [x] Precision grid snapping (`🧲`) with configurable increments
- [x] Non-destructive modifier stack architecture in `forge-modifiers`:
  - [x] Mirror modifier (X, Y, Z axes with outward normal preservation)
  - [x] Subdivision Surface modifier (levels 1..=4)
  - [x] Solidify modifier (wall thickness for open surfaces)
  - [x] Array modifier (count + 3D offset)
- [x] Modifier stack UI in Properties panel (enable toggle, sliders, reordering `▲`/`▼`, delete, and "Apply to Mesh" baking)
- [x] Expanded Add Primitives: Cube, UV Sphere, Icosphere, Plane, Circle, Cylinder, Cone, Torus

### Quality Assurance & Verification
- [x] Unit test suite expanded to **40/40 tests passing** across 7 workspace crates
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (zero warnings)
- [x] `cargo build --release --workspace` completed in 19.69s
- [x] Smoke test verified on live GPU hardware (`Mesa Intel(R) HD Graphics 4000 (IVB GT2)` via `Gl`)
- [x] Documentation updated: `BUGS.md`, `TASKS.md`, `CHANGELOG.md`, `docs/ARCHITECTURE.md`, `docs/SESSION_STATE.md`

---

## Full 3D Control System (Phases 1–6)

### Phase 1: Camera Foundation
- [x] Camera Orbit: Yaw/pitch clamping, MMB + Drag, Alt + LMB + Drag, configurable sensitivity
- [x] Camera Pan: Screen-relative pan proportional to focus distance, Shift + MMB + Drag
- [x] Camera Zoom & Dolly: Scroll wheel zoom, Ctrl + MMB dolly, min/max distance clamping
- [x] Fly / Walk Navigation:
  - [x] WASD movement, Q/E vertical translation
  - [x] Shift acceleration boost, Ctrl deceleration slow-motion
  - [x] Pitch & yaw mouse look, continuous pointer tracking
  - [x] Walk mode: Ground height lock (`walk_height: 1.7m`)
  - [x] Escape / Enter to exit mode cleanly
  - [x] Text input isolation: Fly navigation disabled when text edits are focused
- [x] Camera Focus & Framing: Frame Selected (`F` / Numpad `.`), Frame All (`Home`), Reset View
- [x] Standard Views & Numpad:
  - [x] Numpad 1 Front / Ctrl+1 Back
  - [x] Numpad 3 Right / Ctrl+3 Left
  - [x] Numpad 7 Top / Ctrl+7 Bottom
  - [x] Numpad 5 Perspective / Orthographic toggle
  - [x] Numpad 0 Camera View toggle
  - [x] Numpad 2/4/6/8 15° step viewpoint rotation
  - [x] Emulate Numpad option for keyboards without numeric keypads

### Phase 2: Object Manipulation & Modal Transformations
- [x] Modal transformations finite state machine:
  - [x] `G` — Move / Grab mode
  - [x] `R` — Rotate mode
  - [x] `S` — Scale mode
  - [x] Axis locks: `X`, `Y`, `Z`
  - [x] Plane locks: `Shift+X` (YZ plane), `Shift+Y` (XZ plane), `Shift+Z` (XY plane)
  - [x] Second axis press toggles Local vs Global coordinate space
  - [x] Precision numeric input: digits `0`-`9`, `-`, `.`, and `Backspace` with live HUD badge
  - [x] Confirm with `Enter` / `LMB`, cancel with `Escape` / `RMB` (restores original transform)
- [x] Multi-object transformation support with configurable Pivot Points (Median Point, Active Element, 3D Cursor, Individual Origins)
- [x] Object operations:
  - [x] Duplicate (`Shift+D` + auto-enter Grab mode)
  - [x] Delete (`X` / `Delete`)
  - [x] Hide Selected (`H`), Unhide All (`Alt+H`)
  - [x] Parent / Unparent (`Ctrl+P`, `Alt+P`) with cycle prevention and world-transform preservation
  - [x] Join Objects (`Ctrl+J`) merging meshes into active element
  - [x] Set Origin: To Geometry center, To 3D Cursor position
  - [x] Reset Transforms: Position (`Alt+G`), Rotation (`Alt+R`), Scale (`Alt+S`)

### Phase 3: Interactive Gizmos
- [x] Move Gizmo: X, Y, Z axes, XY, XZ, YZ planar handles, central free drag
- [x] Rotate Gizmo: X, Y, Z Euler rings, screen-space rotation arc
- [x] Scale Gizmo: X, Y, Z axis boxes, central uniform scale handle
- [x] Live hover highlights, ray-line / ray-plane precision intersection
- [x] Discrete grid snapping (`🧲`) integration

### Phase 4: Keyboard Workflow & Input Architecture
- [x] Centralized priority-based event routing:
  1. Text Input Focus: Consumes all character keys when text fields are active (`egui_wants_keyboard_input`)
  2. Active Fly Navigation: Consumes WASDQE, Shift, Ctrl, and mouse delta
  3. Active Modal Transform: Consumes digits, axes, Enter, Escape, mouse delta
  4. 3D Gizmo Dragging: Consumes LMB pointer events
  5. Viewport Navigation: MMB, Alt+LMB, Shift+MMB, Scroll
  6. Global Hotkeys: G, R, S, F, H, X, Tab, Numpad
- [x] Full undo/redo history for single and multi-object transforms, parenting, join, origin, delete

### Phase 5: Scene Camera Objects
- [x] First-class Camera object in scene hierarchy (`PrimitiveType::Camera`)
- [x] Procedural 3D Camera marker mesh (body + lens frustum + upward orientation indicator)
- [x] Active camera tracking (`active_camera_id`), switching, and outliner `★` badge
- [x] Camera optics & sensor properties:
  - [x] Perspective & Orthographic projection modes
  - [x] Bi-directionally synchronized Focal Length (mm) and Field of View (deg)
  - [x] Sensor dimensions (Width × Height mm)
  - [x] Near and far clipping planes
  - [x] Orthographic scale
  - [x] Depth of Field settings (distance, f-stop)
  - [x] Resolution presets (1080p, 4K, 1:1)
- [x] Camera View (`Numpad 0`) with film gate / safe area letterboxing overlay
- [x] Align Active Camera to View (`Ctrl+Alt+Num0` and inspector button)
- [x] Full serialization of Camera objects in `.forge` schema

### Phase 6: Stability, Tests & Verification
- [x] 10 new unit tests added: total **50/50 tests passing** across workspace
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (zero warnings)
- [x] `cargo build --release --workspace` clean
- [x] Release binary smoke tested on live Intel HD 4000 GPU

