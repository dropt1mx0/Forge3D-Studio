# Forge3D Rust Edition — Bug Tracker & Diagnostics

## Resolved Issues (Core Interaction & Modeling Overhaul)

### BUG-001: Workspace Camera Not Moving
- **Severity**: P0 (Critical Blocker)
- **Symptoms**: Viewport camera remained static when dragging with Middle Mouse Button (MMB) or attempting to navigate.
- **Root Cause**:
  1. **Pipeline Execution Inversion**: `renderer.render` was called *before* `render_viewport` in the frame loop. Camera updates from pointer drag events were applied after the offscreen GPU texture was rendered, causing a 1-frame lag and freezing when egui was not repainting continuously.
  2. **Egui Pointer Event Scope**: `Sense::click_and_drag()` in egui does not automatically capture MMB drag deltas unless pointer events are explicitly queried.
  3. **Missing Reactive Repaint**: `ctx.request_repaint()` was not invoked on mouse interaction, leaving egui in reactive idle mode.
- **Fix**:
  - Reordered frame execution: input processing (`render_viewport`) now executes prior to offscreen GPU render pass.
  - Added direct inspection of `input.pointer.button_down(PointerButton::Middle)`, `Alt + PointerButton::Primary`, and `input.smooth_scroll_delta`.
  - Added camera Dolly, Orbit, Pan, Zoom, standard view presets (Front, Back, Right, Left, Top, Bottom via Numpad 1/3/7), Orthographic/Perspective toggling (Numpad 5), and Framing (`F` / Numpad `.`).
  - Added continuous repaint requests whenever viewport interaction occurs.

---

### BUG-002: Inability to Interactively Move, Rotate, and Scale Objects in Viewport
- **Severity**: P0 (Critical Blocker)
- **Symptoms**: Objects could only be modified by typing numbers into the right-hand Properties inspector; no on-screen manipulation gizmo existed.
- **Root Cause**: Viewport lacked an interactive 3D gizmo and ray-space hit testing for object transformation.
- **Fix**:
  - Implemented `apps/forge3d-studio/src/ui/gizmo.rs` providing full 3D interactive gizmos:
    - **Move**: 3 colored axis arrows (+X red, +Y green, +Z blue) + 3 planar squares (XY, XZ, YZ).
    - **Rotate**: 3 rotation arcs/rings around X, Y, Z axes.
    - **Scale**: 3 colored axis boxes + central uniform scale box.
  - Implemented mathematical 3D ray-line closest point calculation (`Ray::closest_point_on_line`) and 3D ray-plane intersection (`Ray::intersect_plane`) for exact screen-to-world manipulation.
  - Added interactive hover highlights (bright yellow) and real-time transform feedback.
  - Added grid snapping toggle (`🧲`) with configurable increment.

---

### BUG-003: Inability to Edit Object Shape / Missing Edit Mode
- **Severity**: P1 (Core Modeling)
- **Symptoms**: Meshes were static triangular buffers; vertices, edges, and faces could not be selected or manipulated.
- **Root Cause**: Absence of an editable B-Rep polygon mesh data structure and mode separation.
- **Fix**:
  - Implemented `EditableMesh` in `crates/forge-geometry/src/edit_mesh.rs` with explicit `EditVertex`, `EditEdge`, and `EditFace` structures.
  - Implemented `EditMode` (`Object` vs `Edit`), toggled via the `Tab` key.
  - Added Vertex Select (`1`), Edge Select (`2`), and Face Select (`3`) modes with single-click and Shift-click multi-selection.
  - Implemented core modeling operations:
    - **Extrude (`E`)**: Extrudes selected faces along their normals, creating side quad walls.
    - **Inset (`I`)**: Scales an inner polygon loop towards face centroid with border quad strips.
    - **Subdivide**: Splits quad faces into 4 quads and triangles into 4 triangles.
    - **Merge at Center (`M`)**: Merges selected vertices at their centroid and removes degenerate elements.
    - **Fill (`F`)**: Constructs a face or edge from selected vertices.
    - **Delete (`Del` / `X`)**: Deletes selected vertices, edges, or faces with topological integrity.
  - Added viewport wireframe and vertex point overlays in Edit Mode.

---

### BUG-004: Missing Undo / Redo and Non-Destructive Modifiers
- **Severity**: P1 (Editor Workflow)
- **Symptoms**: Modifications were destructive and could not be undone.
- **Root Cause**: Absence of a command history stack and modifier architecture.
- **Fix**:
  - Implemented `crates/forge-scene/src/history.rs` (`HistoryStack` and `Action` commands for Transform, AddObject, DeleteObject, ModifyMesh, ApplyTransform) with `Ctrl+Z` (Undo) and `Ctrl+Shift+Z` / `Ctrl+Y` (Redo).
  - Implemented `crates/forge-modifiers` providing non-destructive `ModifierStack`:
    - **Mirror**: Reflects geometry across X, Y, or Z axes with outward normal preservation.
    - **Subdivision Surface**: Procedurally subdivides surfaces up to 4 levels.
    - **Solidify**: Extrudes flat surfaces along normals to give volume.
    - **Array**: Repeats geometry $N$ times with a 3D offset.
  - Connected modifiers stack to Properties panel with enable toggles, parameter sliders, reordering (`▲`/`▼`), deletion, and "Apply to Mesh" baking.
