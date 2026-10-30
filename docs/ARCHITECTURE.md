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

