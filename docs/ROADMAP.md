# Forge3D Master Development Roadmap

This roadmap establishes the phased engineering progression of **Forge3D Rust Edition**, evolving from the verified core foundation into a complete, modular, high-performance open-source 3D production environment.

---

## Strategic Priority Levels
- **P0 — Critical Stability & Foundation**: Fundamental engine architecture, rock-solid stability, zero input latency, memory safety, and core viewport navigation.
- **P1 — Core Functionality**: Professional mesh modeling, essential modifier stack, asset import/export (OBJ, glTF), and initial PBR materials.
- **P2 — Professional Tools**: Advanced modifiers, UV unwrapping, procedural geometry nodes, keyframe animation, rigging, and baking.
- **P3 — Advanced Production**: Physics simulations, real-time clustered PBR rendering, offline CPU/GPU path tracing, compositor, and VFX.
- **P4 — Ecosystem & Future**: Plugin architecture (WASM/native), scripting API, asset management, collaboration, and WebGPU deployment.

---

## Phase Progression & Milestones

### Phase 1 — Core Stability & Navigation Foundation (Status: VERIFIED & COMPLETED)
- **Primary Goals**: Deliver a crash-free desktop application with a high-performance GPU viewport, intuitive 3D camera navigation, primitive generation, transform gizmos, modal transformations, and project persistence.
- **Milestones Completed**:
  - [x] Cross-platform GPU backend via `wgpu` with automatic runtime adapter discovery and Mesa OpenGL fallback on legacy hardware (e.g. Intel HD Graphics 4000).
  - [x] Zero-latency input-first frame loop: pointer and keyboard events processed prior to offscreen GPU rendering pass.
  - [x] Complete 3D viewport navigation: Orbit (MMB, Alt+LMB, empty-space LMB), Pan (Shift+MMB), Dolly/Zoom (Scroll, Ctrl+MMB).
  - [x] Viewport presets (Front, Back, Right, Left, Top, Bottom, Ortho/Persp toggle, Camera View, Framing).
  - [x] First-class Scene Camera objects with procedural 3D physical body marker, optical properties (focal length, sensor size, clipping), and film gate letterboxing.
  - [x] 3D interactive transform gizmos: Move (arrows + planar quads), Rotate (Euler rings), Scale (axis boxes + uniform handle) with hover highlights and snap-to-grid (`🧲`).
  - [x] Modal transformation state machine (`G` Grab, `R` Rotate, `S` Scale) with axis locks (`X`, `Y`, `Z`), plane locks (`Shift+X/Y/Z`), Global/Local space toggling, and live numeric input HUD.
  - [x] Pointer interaction finite state machine (`Idle`, `UIInteraction`, `GizmoDragging`, `BoxSelecting`, `ObjectPressed`, `ViewportOrbiting`) preventing interaction collisions.
  - [x] Command-pattern history stack with full Undo/Redo (`Ctrl+Z`, `Ctrl+Shift+Z`) for transforms, hierarchy edits, and mesh operations.
  - [x] Native `.forge` project format (JSON schema v1) with version compatibility and save/load roundtrip validation.
- **Exit Criteria**: The editor reliably opens, creates, navigates, manipulates, saves, and reloads 3D scenes without crashes or input lag. *(Met — 71/71 tests passing, verified on live hardware)*.

---

### Phase 2 — Professional Polygonal Modeling (Status: IN PROGRESS)
- **Primary Goals**: Build a comprehensive, professional mesh-editing suite inside Forge3D, eliminating dependency on external software for geometry creation.
- **Target Tasks**:
  - [x] Object Mode vs. Edit Mode workflow (`Tab` key toggle) with mode isolation.
  - [x] Pure-CPU `EditableMesh` (B-Rep polygon mesh) data structure in `forge-geometry`.
  - [x] Selection elements: Vertex (`1`), Edge (`2`), Face (`3`) with single and multi-selection.
  - [x] Core operations: Extrude (`E`), Inset (`I`), Subdivide, Merge at center (`M`), Fill (`F`), Delete (`Del`/`X`).
  - [ ] **Loop Cut and Slide (`Ctrl+R`)**: Edge ring detection, quad loop splitting, and interactive slide along rail edges.
  - [ ] **Bevel Tool (`Ctrl+B`)**: Chamfering and rounding edges/vertices with multi-segment curved profiles.
  - [ ] **Knife Tool (`K`)**: Interactive screen-ray projection cutting faces along user-drawn line segments.
  - [ ] **Bridge Edge Loops**: Creating quad surfaces between two open boundary loops.
  - [ ] **Dissolve Elements**: Removing vertices/edges/faces while preserving planar boundary faces.
  - [ ] **Edge Slide & Vertex Slide**: Sliding topological elements along connecting edges.
  - [ ] **Separate (`P`) and Join (`Ctrl+J`)**: Extracting sub-meshes and combining objects cleanly.
  - [ ] **Topological Loop / Ring Selection (`Alt+Click`)**: Quick loop selection using edge-adjacency lookups.
- **Exit Criteria**: Artists can create and refine complex polygonal hard-surface and organic models directly in Forge3D.

---

### Phase 3 — Non-Destructive Modifier Pipeline (Status: IN PROGRESS)
- **Primary Goals**: Establish an extensible non-destructive modifier pipeline with dependency tracking, caching, and background re-evaluation.
- **Target Tasks**:
  - [x] Non-destructive modifier stack architecture in `forge-modifiers` with UI controls and "Apply" baking.
  - [x] Mirror modifier (reflection across X, Y, Z axes with outward normal preservation).
  - [x] Subdivision Surface modifier (quad splitting).
  - [x] Solidify modifier (wall extrusion for open surfaces).
  - [x] Array modifier (repetition with 3D translation offset).
  - [ ] **Boolean Modifier**: Constructive Solid Geometry (Union, Difference, Intersect) using exact arithmetic or robust BSP clipping.
  - [ ] **Bevel Modifier**: Non-destructive edge chamfering with angle limits and vertex weight masks.
  - [ ] **Decimate Modifier**: Mesh simplification via Quadric Error Metrics (QEM) for LOD generation.
  - [ ] **Remesh Modifier**: Uniform quad or dual-contouring surface reconstruction.
  - [ ] **Displace Modifier**: Texture-driven geometric height displacement.
  - [ ] **Modifier Caching & Invalidation**: Caching evaluated geometry and recalculating only modified stack layers.
- **Exit Criteria**: Complex non-destructive procedural modeling workflows operate smoothly with real-time viewport feedback.

---

### Phase 4 — Materials, Lighting & Asset Interchange (Status: PLANNED — NEXT CYCLE)
- **Primary Goals**: Transition from basic shading to full PBR materials, dynamic multi-light scenes, shadow mapping, and standard 3D asset interchange (`crates/forge-io`).
- **Target Tasks**:
  - [x] **Asset I/O Crate (`crates/forge-io`)**:
    - [x] Wavefront OBJ import and export (`.obj`) with clean-room parser, 3D Ear Clipping triangulation, and unified history undo/redo.
    - [ ] glTF 2.0 / GLB import and export (`.gltf` / `.glb`) with PBR material and node hierarchy support.
    - [ ] STL import and export (ASCII & binary) for 3D printing workflows.
  - [ ] **PBR Material Data Model**:
    - [ ] Metallic-Roughness Disney BRDF material definition (Albedo, Metallic, Roughness, Normal, AO, Emissive).
    - [ ] Texture asset management and 2D texture map loading (PNG, JPEG).
    - [ ] WGSL PBR shader pipeline with microfacet Cook-Torrance specular term.
  - [ ] **Dynamic Scene Lighting**:
    - [ ] Multiple scene lights: Point, Directional, Spot, Area lights with color and physical intensity.
    - [ ] Real-time shadow mapping (Cascaded Shadow Maps for directional lights, cubemap shadows for point lights).
    - [ ] Image-Based Lighting (IBL): Equirectangular HDR environment maps with diffuse irradiance and specular prefiltering.
  - [ ] **Viewport Shading Modes**:
    - [ ] Solid Shaded, Wireframe Overlay, Material Preview, and Rendered viewport display modes.
- **Exit Criteria**: Users can import standard 3D models (glTF/OBJ), configure realistic PBR materials, set up studio lighting with shadows, and preview high-fidelity scenes in real-time.

---

### Phase 5 — UV Mapping, Texture Painting & Sculpting (Status: PLANNED)
- **Primary Goals**: Provide complete surface texturing and organic sculpting capabilities.
- **Target Tasks**:
  - [ ] Dedicated 2D UV Editor workspace with UV island visualization, selection, and transform tools.
  - [ ] Seam marking (`Ctrl+E` -> Mark Seam) and automated conformal UV unwrapping (LSCM / ABF++).
  - [ ] UV island bin packing with configurable margins.
  - [ ] 3D viewport texture painting brush engine (Albedo, Roughness, Normal channels).
  - [ ] Sculpting Mode with spherical brush deformation: Draw, Clay, Grab, Smooth, Inflate, Flatten, Pinch, Crease.
  - [ ] Dynamic Topology (Dyntopo) for detail addition during sculpting.
- **Exit Criteria**: Complete end-to-end asset texturing and organic sculpting within the application.

---

### Phase 6 — Procedural Geometry & Node-Based Systems (Status: PLANNED)
- **Primary Goals**: Enable non-destructive procedural content generation and visual shader authoring.
- **Target Tasks**:
  - [ ] Generic Directed Acyclic Graph (DAG) node engine with typed sockets and cycle detection.
  - [ ] Node Editor workspace UI with zoomable/pannable canvas and interactive bezier connections.
  - [ ] Dynamic Attribute System on meshes (Point, Vertex, Edge, Face, Corner domains).
  - [ ] Geometry Nodes: Procedural generators, point distribution on surfaces, mesh instancing, curve math.
  - [ ] Node-based shader graph compiling into WGSL pipelines.
- **Exit Criteria**: Technical artists can author complex procedural geometry and custom shaders visually.

---

### Phase 7 — Animation, Rigging & Character Tools (Status: PLANNED)
- **Primary Goals**: Bring static 3D scenes to life through keyframing, timeline evaluation, and skeletal character animation.
- **Target Tasks**:
  - [ ] Global timeline engine (frame rates, playback controls, loop ranges, transport UI).
  - [ ] Keyframe data structures with Bezier, Linear, and Constant interpolation curves.
  - [ ] Timeline & Dope Sheet workspace panels with interactive keyframe manipulation.
  - [ ] Graph Editor with interactive F-Curve Bezier tangents.
  - [ ] Armature objects and hierarchical bone systems (Edit Mode vs Pose Mode).
  - [ ] Vertex skinning with Linear Blend Skinning (LBS) and weight painting brush interface.
  - [ ] Inverse Kinematics (IK) solvers (Two-Bone IK and FABRIK).
  - [ ] Shape keys / morph targets for facial animation.
- **Exit Criteria**: Animators can rig, skin, keyframe, and play back character and mechanical animations smoothly.

---

### Phase 8 — Physics Simulations (Status: PLANNED)
- **Primary Goals**: Integrate realistic physics simulation engines for dynamic worlds.
- **Target Tasks**:
  - [ ] Rigid Body physics integration (Rapier3D): Dynamic, Static, Kinematic colliders.
  - [ ] Position-Based Dynamics (PBD) cloth simulation with self-collision and pin constraints.
  - [ ] Soft body physics and spring networks.
  - [ ] Particle simulation engine with force fields (gravity, wind, vortex).
  - [ ] Deterministic simulation caching and disk baking.
- **Exit Criteria**: Artists can simulate physics interactions and bake results directly into scene animation tracks.

---

### Phase 9 — Advanced Production: VFX, Compositing & Path Tracing (Status: PLANNED)
- **Primary Goals**: Provide professional post-production rendering and visual effects.
- **Target Tasks**:
  - [ ] Offline CPU/GPU Monte Carlo Path Tracer with BVH acceleration and MIS importance sampling.
  - [ ] Node-based 2D Compositor supporting OpenEXR multi-pass rendering, color grading, and bloom.
  - [ ] Volumetric smoke/fire solver and OpenVDB format caching.
  - [ ] Non-linear video editing sequencer with audio synchronization.
- **Exit Criteria**: Final high-resolution production stills and cinematic animations can be rendered and composited entirely inside Forge3D.

---

### Phase 10 — Ecosystem, Scripting & Community Extensions (Status: PLANNED)
- **Primary Goals**: Build a thriving open-source ecosystem around Forge3D.
- **Target Tasks**:
  - [ ] Embedded scripting engine (Rhai/Python) with rich scene and geometry APIs.
  - [ ] Sandboxed WebAssembly (Wasmtime) plugin architecture with declarative capabilities.
  - [ ] Centralized Asset Browser with local indexing, material presets, and mesh libraries.
  - [ ] WebAssembly / WebGPU browser deployment target.
  - [ ] Collaborative multi-user editing protocol.
- **Exit Criteria**: Third-party developers can extend Forge3D without modifying the engine core.

---

*Last Updated:* 2026-10-31 | *Governing Spec:* `FORGE3D_MASTER_SPEC.md`
