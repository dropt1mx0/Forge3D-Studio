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
