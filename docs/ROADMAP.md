# Forge3D Roadmap

## Alpha 0.1 — First Public Technical Milestone (In Progress)
- Native desktop application window
- wgpu GPU initialization with runtime capability discovery
- 3D viewport with perspective camera (orbit, pan, zoom)
- Viewport ground grid and RGB coordinate axes
- Primitive generation: Cube, UV Sphere, Plane
- Object selection (viewport raycast and scene outliner)
- Object transforms: Translation, Rotation, Scaling
- Real-time transform inspector and scene outliner
- Basic project save/load foundation (`.forge` JSON schema v1)
- Diagnostics bar (GPU adapter, FPS, frame time, triangle count, error console)

## Alpha 0.2 — Basic Modeling
- Editable mesh topology (half-edge / B-Rep foundations)
- Vertex, edge, and face selection modes
- Extrude, inset, bevel primitives
- Normal editing and recalculation
- Wavefront OBJ import/export
- glTF/GLB basic import

## Alpha 0.3 — Scene & Materials
- Scene outliner enhancements and grouping
- Material data model (roughness, metallic, albedo)
- Physically-based lighting (point, directional, spot)
- Undo/redo command stack
- Scene schema migrations

## Alpha 0.4 — Rendering & Asset Pipeline
- Shadow mapping
- Image-based environment lighting (HDR)
- Texture loading and sampler management
- Asset browser foundation
- Performance instrumentation
