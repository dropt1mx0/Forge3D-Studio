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
