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

---

## Resolved Issues (Full 3D Control System — Phase 1–6)

### BUG-005: Hotkey Leakage into Text Input Fields
- **Severity**: P1 (UX / Interaction Blocker)
- **Symptoms**: Typing object names or numerical values in text edit fields triggered modal transforms (`G`, `R`, `S`), camera framing (`F`), or object deletion (`X`).
- **Root Cause**: Hotkeys were intercepted globally without checking whether egui's text edit context had active keyboard focus.
- **Fix**:
  - Added `let wants_keyboard = ui.ctx().egui_wants_keyboard_input();` check guarding all viewport navigation and transform shortcuts.
  - While text fields are focused, all single-character hotkeys (`G`, `R`, `S`, `X`, `F`, `H`, numbers) are ignored by the 3D viewport and routed exclusively to text widgets.

---

### BUG-006: Serialization Project Roundtrip Object Count Drift
- **Severity**: P2 (Unit Test Failure)
- **Symptoms**: `test_project_roundtrip` asserted exact object count `left: 4, right: 3` after adding default Camera to `create_default_scene()`.
- **Root Cause**: `create_default_scene` was updated to include a first-class Scene Camera object (Cube, Sphere, Plane, Camera = 4), but the unit test hardcoded the legacy count of 3.
- **Fix**: Updated `serialization::tests::test_project_roundtrip` assertion to 4 objects and verified that `camera_data` roundtrips accurately with full optical parameters.

---

### BUG-007: Circular Dependency Between Scene Domain and Render Domain
- **Severity**: P0 (Architecture / Compilation Blocker)
- **Symptoms**: Attempting to query or align cameras in `forge-scene` required `forge_render::Camera`, violating the strict domain decoupling rule (scene must not depend on render).
- **Root Cause**: `Camera` struct with GPU projection logic lived in `forge-render`.
- **Fix**:
  - Introduced pure mathematical `CameraViewParameters` struct in `forge-scene::object` containing eye, target, up, FOV, and clipping planes.
  - Encapsulated camera matrix calculations into `forge-scene::Scene::camera_view_parameters(id)` without any graphics dependencies.
  - Mapped parameters to `forge_render::Camera` in `forge3d-studio` at the application composition boundary.

---

## Issues & Verification Status (Critical Interaction Repair & Full System Audit)

### BUG-008: Viewport Pointer Hover vs Drag Disconnect (`interact_pos()` Bypass)
- **Severity**: P0 (Application Interaction Blocker)
- **Subsystem**: `apps/forge3d-studio/src/ui/viewport.rs`
- **Symptoms**: Mouse cursor moved across UI, but objects could not be hovered, gizmos did not highlight, modal operations (G/R/S) did not respond to cursor movement unless a mouse button was held down, and mouse wheel zoom was intermittently ignored.
- **Root Cause**:
  `ui.input(|i| i.pointer.interact_pos())` was used as the primary `mouse_pos`. In egui, `interact_pos()` is exclusively non-null while a mouse button is actively held down. During normal cursor motion or keyboard-driven modal transformations without mouse buttons pressed, `mouse_pos` evaluated to `None`.
- **Fix**:
  Replaced pointer sampling with `ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()))` and unified pointer boundary checks with `response.hovered() || response.contains_pointer() || ...`.
- **Automated Integration Test**: `test_simulation_object_picking_and_selection`, `test_simulation_modal_transform_g_x_5_enter_undo_redo` — **PASS**.
- **Manual GUI Verification Scenario**:
  1. Launch `./target/release/forge3d-studio`.
  2. Without clicking any mouse button, move the cursor over the central Cube.
  3. Observe whether the cursor position is tracked and whether the mouse wheel zooms in/out without requiring a mouse click first.
  4. Press `G` (Move), move mouse without holding buttons: object must follow cursor.
- **Manual GUI Status**: AWAITING USER CONFIRMATION.

---

### BUG-009: Modal Transform Confirmation Click Collision with Raycast Selection
- **Severity**: P0 (Transform State Machine Collision)
- **Subsystem**: `apps/forge3d-studio/src/ui/viewport.rs`
- **Symptoms**: Pressing `G`, `R`, or `S`, moving the mouse, and clicking Left Mouse Button to confirm the operation immediately deselected the object or selected whatever other object happened to be behind the cursor.
- **Root Cause**:
  When Left Mouse Button was clicked during modal transform, the modal handler set `modal.active = false` on that exact frame. Downstream in the same frame, the object selection handler evaluated `lmb_pressed && !modal.active`, triggering an unwanted raycast selection.
- **Fix**:
  Introduced `modal_consumed_click = true` flag whenever a modal transform confirms or cancels via mouse click, preventing raycast selection on that frame.
- **Automated Integration Test**: `test_simulation_modal_transform_g_x_5_enter_undo_redo`, `test_simulation_modal_rotate_r_y_90_enter` — **PASS**.
- **Manual GUI Verification Scenario**:
  1. Click the Cube to select it.
  2. Press `G` to enter Move mode. Move the mouse to displace the cube.
  3. Click Left Mouse Button (LMB) once to confirm the position.
  4. Verify: The Cube must remain selected (orange highlight, active in Outliner and Inspector). It must NOT be deselected.
- **Manual GUI Status**: AWAITING USER CONFIRMATION.

---

### BUG-010: Zero-Thickness Plane AABB Raycast Failure
- **Severity**: P0 (Raycast & Selection Geometry Failure)
- **Subsystem**: `crates/forge-math/src/bounds.rs`
- **Symptoms**: Ground Plane and flat 2D meshes could not be clicked or selected reliably; rays targeting the ground plane produced `None` or failed depending on floating-point precision.
- **Root Cause**:
  For flat meshes with zero thickness along an axis ($min.y == max.y == 0.0$), slab division produced `tmin > tmax` or `NaN` for parallel rays.
- **Fix**:
  Rewrote `Aabb::intersect_ray` with slab inflation $\epsilon = 0.005$ along degenerate dimensions and explicit parallel-ray containment checks.
- **Automated Unit & Integration Test**: `bounds::tests::test_aabb_ray_intersection_flat_plane`, `mesh::tests::test_mesh_raycast_flat_plane` — **PASS**.
- **Manual GUI Verification Scenario**:
  1. In the default scene, click directly on the gray Ground Plane.
  2. Verify: Ground Plane is selected, highlighted, and active in Outliner and Inspector.
  3. Click empty void above the horizon: selection is cleared.
- **Manual GUI Status**: AWAITING USER CONFIRMATION.

---

### BUG-011: Single-Frame Gizmo Undo Overwrite
- **Severity**: P1 (History / Undo System Failure)
- **Subsystem**: `apps/forge3d-studio/src/ui/gizmo.rs`, `apps/forge3d-studio/src/ui/viewport.rs`
- **Symptoms**: Undoing a mouse gizmo drag reverted only the final frame of movement rather than the entire manipulation from where the user started dragging.
- **Root Cause**:
  `old_transform` was captured per-frame rather than at the initial press.
- **Fix**:
  Updated `render_and_interact_gizmo` to record the initial transform at drag capture and return `completed_drag_start` upon `lmb_released`.
- **Automated Integration Test**: `test_simulation_modal_transform_g_x_5_enter_undo_redo` — **PASS**.
- **Manual GUI Verification Scenario**:
  1. Select the Cube.
  2. Drag the red X arrow of the Gizmo across the screen by 3 units, then release the mouse.
  3. Press `Ctrl+Z` (Undo).
  4. Verify: The Cube returns to its original starting position (0, 0.5, 0), not just 1 pixel back.
- **Manual GUI Status**: AWAITING USER CONFIRMATION.

---

### BUG-012: Empty Viewport Space LMB Navigation and Deselection Ambiguity
- **Severity**: P1 (Interaction & Navigation)
- **Subsystem**: `apps/forge3d-studio/src/ui/viewport.rs`, `crates/forge-scene/src/editor_state.rs`
- **Symptoms**: Holding LMB and dragging over empty viewport space did not orbit the camera (only MMB or Alt+LMB did). Attempting to add LMB orbit caused static deselection clicks to conflict with camera dragging or inadvertently deselect active objects while orbiting.
- **Root Cause**:
  Viewport input handling lacked an explicit pointer interaction state machine to disambiguate UI panels, gizmo handles, object selection, box selection, and empty space orbit. Without tracking press origin and drag distance, mouse release on drag could not distinguish an intentional empty-space click from an orbit release.
- **Fix**:
  - Implemented `PointerInteractionState` finite state machine (`Idle`, `ObjectPressed`, `GizmoDragging`, `BoxSelecting`, `ViewportOrbiting`, `UIInteraction`).
  - Recorded `pointer_press_pos` on pointer press over empty viewport space.
  - Allowed LMB dragging to orbit the camera via `app.camera.orbit(delta_x, delta_y)` while preserving pivot, distance, and clamped pitch.
  - On pointer release in `ViewportOrbiting`, if total cursor movement was $< 4.0$ px, treated it as a deliberate deselection click (`app.selection.deselect()`); if moved $\ge 4.0$ px, preserved current selection and finished orbit.
  - Handled pointer release outside viewport and focus loss gracefully.
- **Automated Unit & Integration Test**: `apps/forge3d-studio/tests/lmb_orbit_interaction.rs` (8/8 tests passing) — **PASS**.
- **Manual GUI Verification Scenario**:
  1. Click the Cube to select it.
  2. Move cursor to empty background space, press and hold LMB, and drag across the screen.
  3. Verify: Viewport camera smoothly orbits around the target; Cube remains selected during and after orbit.
  4. With Cube selected, single-click empty background space without dragging (displacement $< 4$ px).
  5. Verify: Selection is cleared.
- **Manual GUI Status**: AWAITING USER CONFIRMATION.


