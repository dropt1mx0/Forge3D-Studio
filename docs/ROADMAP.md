# Forge3D Roadmap

## Alpha 0.1 — Core Interaction & Foundation (Completed)
- [x] Native desktop application window (`eframe` + `egui` Dark Graphite UI)
- [x] wgpu GPU initialization with runtime capability discovery and graceful OpenGL fallback
- [x] 3D viewport with interactive camera (Orbit MMB, Pan Shift+MMB, Zoom Scroll, Dolly)
- [x] Camera presets (Numpad 1/3/7 Front/Right/Top), Frame Selected (Numpad . / F), Frame All (Home)
- [x] Orthographic and Perspective projection switching (Numpad 5)
- [x] Viewport ground grid and RGB coordinate axes
- [x] Primitive generation: Cube, UV Sphere, Plane, Cylinder, Cone, Torus, Circle, Icosphere
- [x] Object selection (viewport raycast and scene outliner)
- [x] Interactive 3D Transform Gizmos:
  - Translation: 3 colored arrows (X/Y/Z) + 3 planar quads (XY/XZ/YZ)
  - Rotation: 3 projected Euler rings (X/Y/Z)
  - Scaling: 3 box manipulators (X/Y/Z) + central uniform scale handle
  - Real-time drag feedback, raycast picking, and hover highlight
- [x] Snap to grid toggle (`🧲`) and hotkeys (Q: Select, W: Move, E: Rotate, R: Scale, Alt+G/R/S: Reset)
- [x] Transform inspector, coordinate space toggle (Global/Local), Pivot point selection
- [x] Project save/load foundation (`.forge` JSON schema v1)
- [x] Diagnostics bar (GPU adapter, FPS, frame time, triangle/vertex metrics, active mode)

## Alpha 0.2 — Modeling Workflow & Modifiers (Completed)
- [x] Object Mode vs. Edit Mode workflow (`Tab` toggle)
- [x] B-Rep `EditableMesh` topology (`EditVertex`, `EditEdge`, `EditFace` with arbitrary polygons)
- [x] Selection modes: Vertex (`1`), Edge (`2`), Face (`3`) with single and multi-selection
- [x] Viewport Edit Mode overlay: vertex dots with orange selection highlights and wireframe edges
- [x] Modeling Operations:
  - Extrude faces (`E`)
  - Inset faces (`I`)
  - Bevel edges/vertices
  - Subdivide topology
  - Merge vertices at center (`M`)
  - Fill polygon from selection (`F`)
  - Delete selected vertices/edges/faces (`Delete` / `X`)
  - Affine transformation of selected elements (Translate, Rotate, Scale)
- [x] Non-destructive Modifier Architecture (`forge-modifiers` crate):
  - Mirror modifier (X, Y, Z reflection with outward normal preservation)
  - Subdivision Surface modifier (quad subdivision)
  - Solidify modifier (wall extrusion)
  - Array modifier (repetition with 3D translation offset)
  - Modifier Stack in Properties panel with reordering, toggling, and "Apply to Mesh" baking
- [x] Centralized Undo/Redo History Stack (`Ctrl+Z`, `Ctrl+Shift+Z` / `Ctrl+Y`)

## Alpha 0.3 — Scene Hierarchy & Materials (Next)
- [ ] Material data model (roughness, metallic, albedo maps, normal maps)
- [ ] Multiple scene lights (point, directional, spot) with shadow mapping
- [ ] PBR shaders with environment lighting (HDR / IBL)
- [ ] Wavefront OBJ and glTF/GLB import/export pipelines (`forge-io` crate)
- [ ] Outliner parenting, grouping, and hierarchy drag-and-drop
