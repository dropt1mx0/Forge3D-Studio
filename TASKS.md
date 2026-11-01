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

---

## Critical Interaction Repair & Full System Audit (P0)

### 1. Viewport Input System & Coordinate Pipeline
- [x] Fixed `interact_pos()` pointer sampling bypass (`hover_pos().or(latest_pos())`)
- [x] Corrected DPI scaling and screen-to-local viewport rect offset calculations
- [x] Screen-to-ray world projection with verified perspective & orthographic reverse matrix
- [x] Raycasting intersection fixed for flat planar geometry (minimum slab thickness $\epsilon = 0.005$)
- [x] Raycast object picking on mouse click: highlights object, selects in Outliner, populates Inspector
- [x] Deselect on clicking empty space; Shift-click toggle for multi-selection
- [x] Box selection (rubber-band drag selection with `B` key and Select tool)
- [x] Select All (`A`), Deselect All (`Alt+A`), Invert Selection (`Ctrl+I`)

### 2. Object Interaction & Transformations
- [x] Modal transform event collision fix: confirmation clicks consume the event, preventing raycast selection fallthrough
- [x] Interactive Move, Rotate, Scale with real-time numeric typing, backspace, negative numbers, decimal points
- [x] Axis constraints (`X`, `Y`, `Z`), plane constraints (`Shift+X`, `Shift+Y`, `Shift+Z`), Global vs Local toggling
- [x] Undo/Redo stack verified for all transformation sequences

### 3. Gizmo Interaction Overhaul
- [x] Multi-frame drag start transform tracking in `GizmoState` to ensure true undo restoration
- [x] Ray-line and ray-plane intersection with hover feedback and drag lock
- [x] Move Gizmo, Rotate Gizmo, Scale Gizmo with screen-distance scaling

### 4. Camera Interaction & Navigation
- [x] Viewport Orbit (MMB, Alt+LMB), Pan (Shift+MMB), Zoom (Scroll wheel), Dolly (Ctrl+MMB)
- [x] Fly navigation (`Shift+F`) with WASDQE, Shift boost, Ctrl slow-down, mouse-look
- [x] View presets (Front, Back, Top, Bottom, Left, Right) pivoting around orbit center
- [x] Perspective / Orthographic projection toggle, Frame Selected, Frame All, Reset View

### 5. Keyboard Workflow & Priority Routing
- [x] Full hotkey priority hierarchy: Text Field Focus > Fly Nav > Modal Transform > Gizmo Drag > Navigation > Global Shortcuts
- [x] Text edits in Inspector and Outliner completely isolated from viewport hotkeys

### 6. Automated Testing & Verification
- [x] 8 new end-to-end integration tests in `apps/forge3d-studio/tests/interaction_flow.rs`
- [x] Total **55/55 tests passing** across workspace
- [x] `cargo fmt --check` clean (0 formatting issues)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (0 warnings)
- [x] `cargo build --release --workspace` clean
- [x] Verified running on live Intel HD 4000 GPU (IVB GT2, OpenGL backend) without panics

---

## Left Mouse Viewport Navigation (LMB Orbit Navigation)

### 1. Pointer Interaction Finite State Machine
- [x] Defined `PointerInteractionState` enum (`Idle`, `ObjectPressed`, `GizmoDragging`, `BoxSelecting`, `ViewportOrbiting`, `UIInteraction`)
- [x] Added `pointer_state`, `pointer_press_pos`, and `show_debug_info` fields to `EditorState`
- [x] State transition on pointer press based on strict input priority: UI > Gizmo > Object Selection > Box Select > Empty Viewport Navigation
- [x] Dragging state lock preventing unintended interaction mode shifts mid-drag

### 2. LMB Orbit Navigation over Empty Viewport Space
- [x] Dragging over empty space rotates camera horizontally (yaw) and vertically (pitch) around pivot
- [x] Reused existing camera orbit mathematics (`Camera::orbit(yaw, pitch)`)
- [x] Preserved camera target pivot point and camera distance
- [x] Prevented camera flipping via vertical pitch clamping ($-89.4^\circ$ to $+89.4^\circ$)
- [x] Smooth and consistent mouse sensitivity via `app.camera.settings.orbit_sensitivity`
- [x] Perspective and Orthographic projection support
- [x] Preserved existing MMB Orbit, Shift+MMB Pan, Scroll Zoom, and Fly Navigation

### 3. Click vs Drag Disambiguation
- [x] Recorded `pointer_press_pos` on empty space press
- [x] On pointer release: displacement $< 4.0$ px triggers selection clear (static click)
- [x] On pointer release: displacement $\ge 4.0$ px finishes orbit without clearing active selection
- [x] Clicking on scene objects immediately transitions to `ObjectPressed` and selects object without camera rotation

### 4. Safety & Focus Loss Handling
- [x] Automatic reset of `pointer_state` to `Idle` on pointer release outside viewport
- [x] Focus loss check resetting dragging state if LMB is released outside the application window
- [x] Prevented conflicting egui pointer captures (`egui_wants_pointer_input()`)

### 5. Viewport Debug Telemetry
- [x] Integrated monospace floating debug telemetry panel in viewport when `show_debug_info` is active
- [x] Surfaces Pointer State, Pointer Pos, Viewport Rect boundaries, Camera Mode, Orbit Pivot, and Camera Distance
- [x] Added "Show Viewport Debug Telemetry" toggle checkbox in the View menu

### 6. Automated Testing & Verification
- [x] Dedicated test suite in `apps/forge3d-studio/tests/lmb_orbit_interaction.rs` with 8 comprehensive integration tests:
  - `test_lmb_drag_over_empty_viewport_starts_orbit`
  - `test_lmb_click_on_object_selects_it`
  - `test_lmb_drag_on_gizmo_does_not_activate_orbit`
  - `test_lmb_drag_with_box_select_does_not_activate_orbit`
  - `test_ui_interaction_does_not_activate_orbit`
  - `test_pointer_release_resets_interaction_state`
  - `test_focus_loss_resets_dragging_state`
  - `test_existing_mmb_orbit_and_shift_mmb_pan_still_work`
- [x] Total **71/71 tests passing** across workspace
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (zero warnings)
- [x] `cargo build --release --workspace` clean

---

## Master Project Audit & Strategic Alignment (P0)
- [x] Comprehensive technical audit against master specification and 31 production domains
- [x] Complete feature traceability matrix in `docs/FEATURE_MATRIX.md` (all 31 domains mapped)
- [x] Architecture assessment and crate boundaries review
- [x] Master 10-phase development roadmap in `docs/ROADMAP.md` with P0–P4 priority tiers
- [x] Performance baseline measurements recorded in `docs/BENCHMARKS.md` on real Ivy Bridge hardware
- [x] ADR-0012 architectural decision record in `docs/DECISIONS.md`
- [x] Defined next implementation cycle (Phase 2 Modeling Tooling & Phase 4 Asset I/O Crate)

---

## Forge3D Alpha 0.2 — Interaction & Workspace UX (P0)

### 1. Automatic Object Interaction & Unified Gizmo System
- [x] Automatic Transform Gizmo display immediately upon object selection
- [x] Full Move, Rotate, and Scale gizmo manipulators (3 axes, 3 planes XY/XZ/YZ, 3 rotation rings)
- [x] Automatic gizmo rebinding upon selecting a different object or multi-selection
- [x] Global and Local coordinate orientation support
- [x] Pivot Points: Median Point, Active Element, 3D Cursor, Individual Origins
- [x] Multi-object transformation support with compound undo action (`Action::MultiTransform`)

### 2. Keyboard Transform System (Modal Transforms)
- [x] Modal keys: `G` (Move), `R` (Rotate), `S` (Scale)
- [x] Single-axis constraints: `X`, `Y`, `Z`
- [x] Planar constraints: `Shift+X` (YZ), `Shift+Y` (XZ), `Shift+Z` (XY)
- [x] Real-time precision numeric input with decimal point and backspace
- [x] `Enter` / LMB to confirm, `Escape` / RMB to cancel
- [x] Safe cancellation upon window focus loss
- [x] Strict UI priority: hotkeys suppressed when text fields have focus (`egui_wants_keyboard_input`)
- [x] No accidental selection on transform confirmation (`modal_consumed_click`)

### 3. Unified Selection & Gizmo Priority
- [x] LMB click selects object and automatically reveals gizmo
- [x] LMB drag on gizmo manipulates active object(s)
- [x] Shift+LMB toggles object multi-selection
- [x] Box Select (`B`) for region multi-selection
- [x] LMB click on empty space deselects if displacement $< 4.0$ px; orbits if $\ge 4.0$ px
- [x] Verified input priority hierarchy: UI > Gizmo > Object Select > Box Select > Viewport Navigation
- [x] Preserved existing MMB Orbit, Shift+MMB Pan, Alt+LMB, and scroll zoom

### 4. Resizable Workspace Layout & Splitters
- [x] Draggable splitter with visual feedback (accent line `#f09020`, resize cursor) for Left Toolbar (48–240 px)
- [x] Toolbar display modes: Icon Only vs Icon + Label, plus collapse toggle
- [x] Draggable splitter with visual feedback for Right Sidebar (220–600 px)
- [x] Draggable vertical splitter for Outliner / Properties ratio (0.15–0.85)
- [x] Bottom Timeline panel (60–350 px) with transport controls and drag splitter
- [x] Clamped dimensions preventing layout destruction
- [x] Persistent panel dimensions and states via `forge3d_settings.json`

### 5. Categorized Toolbar
- [x] Grouped tools into Selection, Transform, Modeling, Add Mesh, Utilities
- [x] Clear titles, icons, tooltips, and keyboard shortcuts
- [x] Zero fake visual buttons without implementations

### 6. Add Object System Expansion
- [x] Procedural Capsule primitive generator (`generate_capsule`) with verified normals and UVs
- [x] Procedural Monkey / Suzanne mascot head (`generate_monkey`) clean-room 42-vertex model
- [x] Searchable Add Menu with category groupings (`Mesh Primitives`, `Scene Objects`) and query filter

### 7. Automated Testing & Verification
- [x] Added `apps/forge3d-studio/tests/alpha_0_2_ux_interaction.rs` with 7 integration tests
- [x] Total **80/80 tests passing** across workspace
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (zero warnings)
- [x] `cargo build --workspace` clean

---

## Forge3D — Free Cursor Object Dragging (P0)

### 1. World-Space Drag Calculation
- [x] Construct camera ray from mouse position through working viewport camera
- [x] Establish camera-aligned working plane passing through initial grab point (`plane_normal = -camera.forward().normalize()`)
- [x] Intersect ray with working plane to establish initial hit point
- [x] Compute ray-plane intersection on every pointer movement frame
- [x] Calculate world-space delta between current and initial hit points

### 2. Grab Point Offset Retention (Zero-Jump Guarantee)
- [x] Store `initial_offset = obj.translation - hit_point` for every selected object at drag initiation
- [x] Apply translation as `obj.translation = current_hit + initial_offset`
- [x] Zero-delta identity at frame 0 preventing sudden object jumps or snap-to-cursor behavior

### 3. Multi-Object Drag Support
- [x] Detect multiple selected objects and track all initial transforms in `ObjectDragState`
- [x] Apply single calculated world-space delta to all selected objects simultaneously
- [x] Preserve exact relative distances and configuration between objects
- [x] Record group translation as a single atomic `Action::MultiTransform` in undo history on mouse release

### 4. Input Priority Hierarchy
- [x] Strict hierarchy enforced: UI Interaction > Gizmo Interaction > Object Drag > Object Selection > Box Selection > Viewport Navigation
- [x] Protected against accidental drag when clicking on UI panels (`ui_captures_pointer`)
- [x] Protected against accidental drag when clicking Gizmo handles (`gizmo_consumed`)
- [x] Viewport navigation (MMB Orbit, Shift+MMB Pan, Scroll Zoom, empty space LMB orbit) remains completely functional

### 5. Edge Cases & Robustness
- [x] Safe parallel ray rejection (`|ray_dir · plane_normal| < 1e-4`)
- [x] Positive distance check ($t > 0.0$) preventing negative/behind-camera intersection
- [x] Distance clamping ($t \le 10000.0$) preventing coordinate divergence near horizon
- [x] Maximum delta clamp ($|\Delta| \le 5000.0$) preventing warp spikes
- [x] Window focus loss cancellation restoring initial object transforms
- [x] `Escape` key cancellation restoring initial object transforms
- [x] Viewport boundary exit handling

### 6. Undo / Redo & Gizmo Compatibility
- [x] Single undo command created only upon mouse release
- [x] `Ctrl+Z` reverses the complete drag movement in one step
- [x] Full compatibility with all existing Gizmo manipulators (Move, Rotate, Scale) and modal hotkeys (`G`, `R`, `S`)

### 7. Automated Testing & Quality Gates
- [x] 14 automated unit and integration tests in `apps/forge3d-studio/tests/free_cursor_drag.rs` (including live egui viewport and full application UI drag & isolation tests)
- [x] Total **94/94 tests passing** across workspace
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (zero warnings)
- [x] Full application live input verified: `test_full_app_ui_live_drag_and_ui_isolation` and `test_egui_render_viewport_live_drag` pass with real pointer movement, drag deltas, release commit, and undo


