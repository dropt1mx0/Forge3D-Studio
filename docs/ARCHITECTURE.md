# Forge3D Architecture

## Crate Dependency Graph

```text
               apps/forge3d-studio
             /          |          \
            v           v           v
      forge-render  forge-scene  forge-gpu
           |      \   /     \       /
           |       v v       v     v
           |   forge-modifiers  (wgpu)
           |         |
           |         v
           |   forge-geometry
           |       /     \
           v      v       v
         forge-math   forge-core
              \           /
               v         v
                 (glam, etc.)
```

## Core Invariants
1. **Renderer-agnostic domain crates**: `forge-core`, `forge-math`, `forge-geometry`, `forge-modifiers`, and `forge-scene` must NEVER import `wgpu`, `egui`, or any graphics/UI handle.
2. **Explicit Data Flow**:
   ```text
   User Input / UI Interaction / 3D Gizmo
          ↓
   Scene Mutation / Mesh Edit / Command (forge-scene / forge-geometry)
          ↓
   Non-Destructive Modifier Evaluation (forge-modifiers: Mirror, Subdiv, Solidify, Array)
          ↓
   Derived Data Update (World Transforms, Evaluated Mesh, AABBs)
          ↓
   GPU Buffer Sync / Dynamic Reallocation (forge-gpu / forge-render)
          ↓
   Viewport Render Pass (wgpu) & Overlay Rendering (egui)
   ```
3. **Editable Mesh & Modeling Foundations**:
   - `EditableMesh` (B-Rep model in `forge-geometry::edit_mesh`): Explicit vertices, directed edges, and arbitrary polygon faces with per-face normal recalculation.
   - Operations: Extrude, Inset, Subdivide, Merge at center, Fill face, Delete (vertices/edges/faces), and affine selection transformations.
   - Independent selection states: Vertex, Edge, and Face selection indices with multi-select support.
4. **Coordinate Conventions**:
   - Right-handed coordinate system.
   - +Y is Up.
   - +Z is Forward / Towards Camera in view space.
   - +X is Right.
   - Angles in radians (degrees converted for UI presentation).
   - Distance units: Meters.
5. **Pure-CPU Testing**: All math, mesh algorithms, modifiers, hierarchy, history/undo, and serialization are tested in standard CI without needing GPU devices or display servers.
6. **Input-First Frame Pipeline**: In `forge3d-studio`, user inputs (gizmo manipulation, hotkeys, camera orbiting) are processed *prior* to offscreen texture rendering to ensure zero-latency 60 FPS viewport response.

## Full 3D Control System Architecture

### 1. Viewport Navigation vs. Scene Camera Objects
Forge3D maintains a strict architectural separation between the **Working Viewport Camera** (`forge_render::Camera`) and **Scene Camera Objects** (`forge_scene::SceneObject` with `PrimitiveType::Camera` and `CameraData`):
- **Viewport Camera**: Transient, interactive view frustum supporting Orbit, Pan, Dolly, Fly, Walk, and standard orthographic presets.
- **Scene Camera Object**: Persistent, first-class citizen of the scene hierarchy with procedural physical body marker (`generate_camera_marker()`), transform parenting, and optical sensor metadata (focal length, sensor size, clipping, DOF, resolution).
- **Decoupled Optical Translation**: `forge_scene::Scene::camera_view_parameters(id)` calculates world eye, target, up vectors, and projection settings entirely on the CPU without referencing graphics libraries. In Camera View (`Numpad 0`), the Viewport Camera mirrors these parameters and renders a film gate letterboxing overlay reflecting the camera's aspect ratio.

### 2. Modal Transformation Finite State Machine
Object transformations follow a modal state machine encapsulated in `forge_scene::ModalTransformState`:
```text
           [Idle / Select Mode]
                    │
         G, R, S    │
                    ▼
          [Active Modal Transform] ◄──────┐
           ├── Mouse Delta Drag           │ X, Y, Z / Shift+X, Y, Z
           ├── Numeric Key Buffer         │ (Toggles Local / Global)
           │   ('1', '2', '.', '-') ──────┘
           │
           ├── Enter / LMB Confirm ───────► [Push MultiTransform to History]
           └── Escape / RMB Cancel ───────► [Restore Initial Transforms]
```
- **Axis & Plane Locking**: Pressing `X`, `Y`, or `Z` limits transformation to that coordinate axis. Pressing `Shift+X`, `Shift+Y`, or `Shift+Z` limits transformation to the perpendicular plane. Pressing the same axis again toggles between Global and Local object orientation.
- **Numeric Buffer**: Characters typed while in modal transform are parsed into exact float offsets or degrees, overriding mouse pointer delta.

### 3. Input Routing Priority
To avoid conflicting shortcuts and prevent hotkey leakage, event handling follows a strict priority chain:
1. **Text Input Focus (`egui_wants_keyboard_input`)**: When any text field in Properties or Outliner is focused, all single-character hotkeys are swallowed by the UI widget.
2. **Fly / Walk Navigation Mode**: When active, WASD, Q, E, Shift, Ctrl, and mouse deltas are routed exclusively to camera translation and look vectors.
3. **Modal Transform Mode**: When active, digits, minus, dot, backspace, axis keys, Enter, and Escape are consumed by `ModalTransformState`.
4. **Interactive 3D Gizmos**: Pointer down on gizmo handles claims mouse interaction, disabling viewport orbit/pan.
5. **Viewport Navigation**: MMB, Alt+LMB, Shift+MMB, and scroll wheel drive camera orbit, pan, and zoom.
6. **Global Hotkeys**: Object shortcuts (`G`, `R`, `S`, `F`, `H`, `Tab`, Numpad) execute only when no higher-priority mode is active.

### 4. Selection & Raycast Geometry Pipeline
- **Screen Coordinates**: Sampled with `hover_pos().or(latest_pos())` relative to `viewport_rect.min`.
- **Screen to NDC**: $x_{ndc} = \frac{2 x_{screen}}{w} - 1$, $y_{ndc} = 1 - \frac{2 y_{screen}}{h}$.
- **Unprojection**: Ray origin and direction derived using $(P \cdot V)^{-1}$.
- **Broadphase**: Accelerated with `Aabb::intersect_ray` featuring Kay-Kajiya slab testing with $\epsilon = 0.005$ thickness inflation to guarantee robust intersection for zero-thickness 2D planar meshes.
- **Narrowphase**: Möller–Trumbore ray-triangle intersection over the object's evaluated geometry.
- **Click Consumption**: Modal transformation confirmation (`Enter`/`LMB`) marks `modal_consumed_click = true`, preventing event fallthrough into raycast selection on the same frame.

### 5. Box Selection Pipeline
- 2D rubber-band bounding rectangle is tracked in screen space while dragging in Select mode (`B`).
- Objects in the active scene have their world positions transformed via $(P \cdot V)$ and clipped against the near plane ($w_{clip} > 0.0$).
- Valid screen-space points falling inside `box_rect` are added to the selection set (or toggled when Shift is held).

### 6. Pointer Interaction Finite State Machine (`PointerInteractionState`)
To guarantee unambiguous pointer event handling and prevent conflicting operations (such as rotating the viewport while dragging a gizmo or selecting an object), the viewport interaction loop implements an explicit 6-state finite state machine:
- **`Idle`**: Default resting state. No pointer buttons held down.
- **`UIInteraction`**: Initiated when LMB is pressed over UI panels, floating windows, or widgets (`egui_wants_pointer_input()`). Viewport camera and object manipulation are inhibited.
- **`GizmoDragging`**: Initiated when LMB is pressed over a 3D gizmo axis handle or planar quad. Cursor drag deltas drive object translation, rotation, or scaling.
- **`BoxSelecting`**: Initiated when LMB is pressed in Box Select tool mode (`TransformMode::BoxSelect` or `B` key). Cursor drag renders a selection rectangle and selects enclosed objects upon release.
- **`ObjectPressed`**: Initiated when LMB is pressed directly over a scene object (via raycast). Selects the object immediately.
- **`ViewportOrbiting`**: Initiated when LMB is pressed over empty viewport space. Cursor movement rotates the working camera around its pivot target.

**Click vs. Drag Disambiguation**:
When `ViewportOrbiting` is active, pointer release measures the Euclidean distance from `pointer_press_pos`:
- If displacement was $< 4.0$ px, the interaction is classified as an intentional static click on empty space, clearing active selection (`app.selection.deselect()`).
- If displacement was $\ge 4.0$ px, the interaction is classified as a camera orbit drag; active selection is preserved intact.

**Focus Loss & Release Safety**:
If `lmb_down` is false while the state is not `Idle` (e.g. pointer released outside viewport or window lost focus), the state machine immediately resets to `Idle`, clearing press positions and active drag states.

### 7. Free Cursor Object Dragging Pipeline
To enable direct, unconstrained manipulation of 3D objects across the viewport, Forge3D implements free cursor dragging via camera-aligned ray-plane projection:

```text
[LMB Down on Mesh Geometry]
         ↓
1. Construct Camera Ray from Pointer Pos
2. Create Working Plane at Grab Point:
   - plane_point = hit_point
   - plane_normal = -camera.forward().normalize()
3. Compute Grab Offsets:
   - initial_offset_i = obj_i.translation - hit_point
4. Transition to PointerInteractionState::ObjectPressed
         ↓
[Pointer Dragging (distance >= 3.0 px)]
         ↓
5. Transition to PointerInteractionState::ObjectDragging
6. Per-frame Camera Ray:
   - current_hit = Ray ∩ Working Plane
   - delta = current_hit - hit_point (with bounds & finite validation)
7. Update Translations:
   - obj_i.translation = current_hit + initial_offset_i
   - (or initial_translation_i + delta)
         ↓
[LMB Released]
         ↓
8. Commit single atomic Action::MultiTransform to HistoryStack
9. Transition back to PointerInteractionState::Idle
```

- **Camera-Aligned Working Plane**: Constructing the plane normal as `-camera.forward().normalize()` ensures the working plane is perfectly parallel to the camera sensor. This provides natural 1:1 mouse movement mapping in world units, avoiding perspective distortion or projection collapse.
- **Grab Point Offset Retention**: Storing `initial_offsets` for all selected entities guarantees the object never jumps on frame 0 ($\Delta = 0$ at the initial hit point). The cursor stays anchored to the exact surface point clicked.
- **Multi-Object Synchronous Drag**: When multiple objects are selected, the single calculated world-space delta $\Delta$ is applied to all selected objects, preserving their relative positions.
- **Edge Case Robustness**:
  - Rays parallel to plane: Denominator threshold $|ray.dir \cdot plane.normal| < 10^{-4}$ safely returns early with no transform mutation.
  - Backward rays ($t \le 0.0$): Discarded.
  - Horizon / divergent rays ($t > 10000.0$ or $|\Delta| > 5000.0$): Clamped to prevent float overflow or NaN corruption.
  - Escape key / Focus loss: Reverts all selected objects to their `initial_transforms` without polluting the undo stack.

### 8. Adaptive Smooth Viewport Zoom & Cursor Anchoring
To eliminate sensitive camera jumping and allow fluid navigation at any scale, Forge3D implements an exponential multiplicative zoom pipeline with cursor-anchored homothety and frame-rate independent easing:

```text
[Mouse Wheel Event (smooth_scroll_delta.y)]
         ↓
1. Compute Exponential Zoom Multiplier:
   - sens = 0.0025 * settings.zoom_sensitivity.clamp(0.1, 5.0)
   - mult = exp(-scroll_delta * sens).clamp(exp(-1.2), exp(1.2))
         ↓
2. Determine Anchor Hit Point P:
   - If zoom_to_cursor:
       - Raycast from mouse screen pos against Scene geometry
       - If hit: P = Ray.point_at(dist)
       - If miss: Fallback intersect Ray with Focus Plane:
           plane_point = camera.target
           plane_normal = -camera.forward()
   - If !zoom_to_cursor or no valid hit:
       P = camera.target
         ↓
3. Scale Camera State via Homothety about P:
   - k = target_dist / current_dist
   - eye_new = P + (eye - P) * k
   - target_new = P + (target - P) * k
   - Bounds: clamp distance to [0.05, 5000.0]
   - (In Orthographic: ortho_size_new = ortho_size * k, eye_new += delta_target)
         ↓
4. If smooth_zoom enabled:
   - Queue to (zoom_target_eye, zoom_target_target, zoom_target_ortho_size)
   - Clamp target ratio to [0.15 * current, 6.0 * current] (anti-queueing guard)
   - On frame tick:
       alpha = 1.0 - exp(-18.0 * dt)
       eye = lerp(eye, zoom_target_eye, alpha)
       target = lerp(target, zoom_target_target, alpha)
       converged when |diff| < 0.001 (halts repaints)
   - Orbit / Pan / Dolly / Fly / Presets instantly call sync_zoom_targets()
```

- **Collinear View Invariant**: Homothety centered at $P$ guarantees the vector $(P - E_{new}) = (P - E) \cdot k$ is parallel to $(P - E)$. Under perspective projection, $P$ projects to the exact same 2D screen coordinates before and after zoom, eliminating visual disorientation.
- **Dynamic Step Scaling**: At close range ($d \approx 0.1\text{m}$), $7.3\%$ scaling moves the camera by millimeters; at large range ($d \approx 100\text{m}$), $7.3\%$ moves by meters. Distance is clamped to $[0.05, 5000.0]$ preventing inverted projection or horizon overflow.




