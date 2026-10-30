# Forge3D Architecture

## Crate Dependency Graph

```text
               apps/forge3d-studio
             /          |          \
            v           v           v
      forge-render  forge-scene  forge-gpu
           |      \   /     \       /
           |       v v       v     v
           |   forge-geometry   (wgpu)
           |       /     \
           v      v       v
         forge-math   forge-core
              \           /
               v         v
                 (glam, etc.)
```

## Core Invariants
1. **Renderer-agnostic domain crates**: `forge-core`, `forge-math`, `forge-geometry`, and `forge-scene` must NEVER import `wgpu`, `egui`, or any graphics/UI handle.
2. **Explicit Data Flow**:
   ```text
   User Input / UI Interaction
          ↓
   Scene Mutation (forge-scene)
          ↓
   Derived Data Update (World Transforms, AABBs)
          ↓
   GPU Buffer Sync / Update (forge-gpu / forge-render)
          ↓
   Viewport Render Pass (wgpu)
   ```
3. **Coordinate Conventions**:
   - Right-handed coordinate system.
   - +Y is Up.
   - +Z is Forward / Towards Camera in view space.
   - +X is Right.
   - Angles in radians (degrees converted for UI presentation).
   - Distance units: Meters.
4. **Pure-CPU Testing**: All math, mesh algorithms, hierarchy, and serialization can be tested in standard CI without needing GPU devices or display servers.
