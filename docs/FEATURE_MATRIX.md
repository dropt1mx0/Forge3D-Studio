# Forge3D Feature Matrix & Requirements Traceability

This document provides complete traceability between the master specification (**OPEN SOURCE 3D EDITOR — Technical Specification of a Universal 3D Modeling and Digital Production Environment**) and the Forge3D Rust Edition codebase.

## Status Legend
- `NOT_STARTED`: Specification requirement acknowledged; no code exists.
- `PLANNED`: Architectural design established; scheduled for a near milestone.
- `IN_PROGRESS`: Code exists and is under active development.
- `IMPLEMENTED`: Functionality is written and compiles cleanly.
- `TESTED`: Automated unit/integration tests verify correct functionality.
- `VERIFIED`: Functionality confirmed through automated tests and hardware/GUI runtime execution.
- `BLOCKED`: Development halted due to an external architectural blocker.
- `DEFERRED`: Intentionally scheduled for long-term advanced phases.

---

## 1. Scene Management & Object Systems

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SCN-001** | Atomic Object Identification | 1.1 | Unique generational/atomic identifiers (`ObjectId`) for scene elements. | `VERIFIED` | `forge-core` | P0 | Alpha 0.1 | Unique ID generation, thread safety, serialize/deserialize. | `forge-core::id::tests` (2/2) | 64-bit integer space; no distributed UUIDs. |
| **SCN-002** | Transform Hierarchy | 1.2 | Affine parent-child spatial hierarchy with forward matrix accumulation. | `VERIFIED` | `forge-math`, `forge-scene` | P0 | Alpha 0.1 | Child objects inherit parent translation, rotation, scale; cycle prevention. | `forge-scene::scene::tests` | Deeply nested graphs evaluate linearly on CPU. |
| **SCN-003** | Scene Outliner & Graph | 1.3 | Hierarchical scene tree representation with visibility and selection toggles. | `VERIFIED` | `forge-scene`, `apps/forge3d-studio` | P0 | Alpha 0.1 | Interactive tree widget displaying names, visibility eye toggles, selection highlights. | `interaction_flow.rs` | Single active scene; no folder-like collections yet. |
| **SCN-004** | Object Parenting & Unparenting | 1.4 | Dynamic parenting (`Ctrl+P`) and unparenting (`Alt+P`) preserving world transform. | `TESTED` | `forge-scene` | P1 | Alpha 0.3 | Parenting preserves child visual position in world space; cycle prevention. | `test_parent_and_unparent_objects` | Hierarchy drag-and-drop in Outliner UI not yet interactive. |
| **SCN-005** | Object Joining | 1.5 | Merging multiple selected objects and meshes into a single active mesh (`Ctrl+J`). | `TESTED` | `forge-geometry`, `forge-scene` | P1 | Alpha 0.3 | Vertex/index buffers combined with world-to-local transform baked in. | `test_join_objects` | Multi-material slot assignment on join not yet supported. |
| **SCN-006** | Multi-Scene Architecture | 1.6 | Support for multiple independent scenes per project with cross-scene linking. | `PLANNED` | `forge-scene` | P2 | Phase 4 | User can create, rename, switch, and delete multiple scenes in a project. | None | Current `ForgeProject` supports single `Scene`. |
| **SCN-007** | Collections & Tagging | 1.7 | Grouping objects into colored, hierarchical collections with visibility masks. | `PLANNED` | `forge-scene` | P2 | Phase 4 | Nested collections with isolated render/viewport visibility toggles. | None | Objects currently reside in flat or parented lists. |
| **SCN-008** | Instancing & Linked Duplicates | 1.8 | Linked duplicates (`Alt+D`) sharing underlying geometry data with distinct transforms. | `PLANNED` | `forge-scene`, `forge-render` | P1 | Phase 4 | Multiple `SceneObject` entities referencing single `Mesh` instance; GPU instanced draws. | None | Duplicates (`Shift+D`) currently perform deep copies. |

---

## 2. Mesh Modeling & Geometry Editing

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **GEO-001** | GPU Render Mesh | 2.1 | Interleaved 48-byte vertex buffers and 32-bit index buffers with normal generation. | `VERIFIED` | `forge-geometry` | P0 | Alpha 0.1 | Verified memory layout (`bytemuck`), outward normals, AABB computation. | `forge-geometry::mesh::tests` | Static index buffer; rebuild required on edit. |
| **GEO-002** | B-Rep Editable Mesh Topology | 2.2 | Pure-CPU polygon B-Rep structure (`EditableMesh`) supporting arbitrary polygons. | `TESTED` | `forge-geometry` | P0 | Alpha 0.2 | Explicit vertices, directed edges, polygon faces, face normal recalculation. | `edit_mesh::tests` (3/3) | Half-edge adjacency not explicitly cached. |
| **GEO-003** | Procedural Primitives | 2.3 | Standard 3D geometric primitives generated with correct normals and UVs. | `VERIFIED` | `forge-geometry` | P0 | Alpha 0.1 | Cube, UV Sphere, Icosphere, Plane, Cylinder, Cone, Torus, Circle, Capsule, Monkey (Suzanne), Camera Marker. | `primitives::*::tests` (11/11) | Segment count fixed at creation; no parametric history. |
| **GEO-004** | Extrude Tool | 2.4 | Extruding selected faces along normal creating connecting boundary quad walls (`E`). | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Generates side walls, offsets face, preserves manifold winding. | `test_editable_mesh_extrude` | Individual face extrusion only; manifold inset extrude in progress. |
| **GEO-005** | Inset Tool | 2.5 | Insetting faces towards centroid with connecting quad ring (`I`). | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Insets arbitrary polygon faces with adjustable depth and boundary quads. | `edit_mesh::tests` | Self-intersection on complex concave polygons not resolved. |
| **GEO-006** | Subdivide Tool | 2.6 | Splitting faces (quads to 4 quads, triangles to 4 triangles) with vertex smoothing. | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Subdivides selected or all faces; maintains surface continuity. | `test_editable_mesh_subdivide` | Linear subdivision; Catmull-Clark boundary creases in Phase 2. |
| **GEO-007** | Merge Vertices | 2.7 | Merging selected vertices at centroid, first, last, or cursor (`M`). | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Replaces vertices, remaps face indices, removes degenerate edges. | `edit_mesh::tests` | Merge at center implemented; merge by distance in progress. |
| **GEO-008** | Fill Face / Edge | 2.8 | Creating an edge from 2 vertices or an n-gon face from $\ge 3$ vertices (`F`). | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Creates valid topological elements without normal inversion. | `edit_mesh::tests` | Concave n-gon ear clipping required for rendering triangulation. |
| **GEO-009** | Delete Elements | 2.9 | Deleting vertices, edges, or faces with topological integrity (`Del`/`X`). | `TESTED` | `forge-geometry` | P1 | Alpha 0.2 | Deleting vertex removes adjacent edges and faces without leaving orphaned indices. | `edit_mesh::tests` | Dissolve (preserving face boundaries) not yet implemented. |
| **GEO-010** | Loop Cut & Slide | 2.10 | Detecting edge rings, inserting edge loops, and sliding along quad flow (`Ctrl+R`). | `PLANNED` | `forge-geometry` | P1 | Phase 2 | Ring traversal, quad splitting, interactive slide interpolation along rails. | None | Scheduled for immediate Phase 2 modeling sprint. |
| **GEO-011** | Bevel Tool | 2.11 | Chamfering or rounding selected edges and vertices with configurable segments. | `PLANNED` | `forge-geometry` | P1 | Phase 2 | Edge splitting, corner miter generation, profile curvature calculation. | None | Basic vertex offset stub in place; full miter pending. |
| **GEO-012** | Knife Tool | 2.12 | Interactive screen-ray projection cutting faces along user-defined line segments (`K`). | `PLANNED` | `forge-geometry` | P2 | Phase 2 | Bisecting edges/faces with raycast intersection points and retriangulation. | None | High complexity; requires stable raycast narrowphase. |
| **GEO-013** | Bridge Edge Loops | 2.13 | Creating quad skin between two open boundary loops of equal or unequal vertex count. | `PLANNED` | `forge-geometry` | P2 | Phase 2 | Generates connecting quad strips with twist and bridge interpolation. | None | Edge loop boundary detection required. |

---

## 3. Modeling Modes & Selection Systems

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SEL-001** | Object Mode vs Edit Mode | 3.1 | Strict separation between scene object manipulation and geometry editing (`Tab`). | `VERIFIED` | `forge-scene`, `apps/forge3d-studio` | P0 | Alpha 0.2 | Mode toggle hotkey, state isolation, mesh synchronization upon commit. | `app.rs`, `interaction_flow.rs` | Single object active edit mode; multi-object edit mode planned. |
| **SEL-002** | Vertex / Edge / Face Selection | 3.2 | Independent selection domains with hotkeys (`1`, `2`, `3`). | `TESTED` | `forge-geometry`, `apps/forge3d-studio` | P0 | Alpha 0.2 | Selecting elements, highlight rendering, multi-element transformation. | `edit_mesh::tests` | Viewport click-picking for sub-elements in progress. |
| **SEL-003** | Raycast Object Picking | 3.3 | Unprojected screen ray intersection with AABB broadphase and triangle narrowphase. | `VERIFIED` | `forge-math`, `forge-scene` | P0 | Alpha 0.1 | Click over object selects it; inflation $\epsilon=0.005$ for flat planes. | `test_simulation_object_picking_and_selection` | Heavy scenes (>100k triangles) evaluate linearly without BVH. |
| **SEL-004** | Box Selection | 3.4 | 2D screen-space rubberband rectangle selecting all intersecting elements (`B`). | `VERIFIED` | `forge-scene`, `apps/forge3d-studio` | P0 | Alpha 0.1 | Screen projected coordinate bounds test; Shift modifier toggles selection. | `test_simulation_selection_tools_all_none_invert_box` | Selects based on projected object origin / center. |
| **SEL-005** | Select All / None / Invert | 3.5 | Fast selection shortcuts: `A` (All), `Alt+A` (None), `Ctrl+I` (Invert). | `VERIFIED` | `forge-scene` | P0 | Alpha 0.1 | Toggles active and multi-selection sets accurately. | `test_simulation_selection_tools_all_none_invert_box` | Complete. |
| **SEL-006** | Loop / Ring Selection | 3.6 | Boundary edge loop (`Alt+Click`) and ring (`Ctrl+Alt+Click`) topological traversal. | `PLANNED` | `forge-geometry` | P1 | Phase 2 | Traverses continuous quad edge rings and boundary loops. | None | Requires half-edge or edge-adjacency lookup cache. |
| **SEL-007** | Circle / Lasso Selection | 3.7 | Interactive brush radius circle select (`C`) and freehand lasso polygon selection. | `PLANNED` | `apps/forge3d-studio` | P2 | Phase 2 | Circle brush painting selection; point-in-polygon lasso testing. | None | Scheduled for secondary selection sprint. |

---

## 4. Geometry Modifiers

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **MOD-001** | Modifier Stack Architecture | 4.1 | Non-destructive modifier pipeline with reordering, enable toggles, and baking. | `TESTED` | `forge-modifiers`, `forge-scene` | P0 | Alpha 0.2 | Modifiers evaluate on pure-CPU copy; "Apply" commits evaluated mesh. | `stack::tests::test_modifier_stack_pipeline` | Evaluation is synchronous on main thread. |
| **MOD-002** | Mirror Modifier | 4.2 | Reflection across local X, Y, or Z axes with outward normal preservation. | `TESTED` | `forge-modifiers` | P0 | Alpha 0.2 | Duplicates vertices across plane, reverses triangle winding to preserve normals. | `test_mirror_modifier_doubles_cube` | Centerline clipping and vertex merging threshold in progress. |
| **MOD-003** | Subdivision Surface Modifier | 4.3 | Procedural quad subdivision up to 4 iterations for surface smoothing. | `TESTED` | `forge-modifiers` | P0 | Alpha 0.2 | Subdivides faces, interpolates vertex attributes cleanly. | `test_subdivision_modifier` | Linear subdivision; Catmull-Clark limit surface math planned. |
| **MOD-004** | Solidify Modifier | 4.4 | Extruding open surfaces along vertex normals to create manifold thickness. | `TESTED` | `forge-modifiers` | P0 | Alpha 0.2 | Generates inner and outer shells with connecting rim quad walls. | `test_solidify_modifier` | Even thickness on acute corners requires angle clamp. |
| **MOD-005** | Array Modifier | 4.5 | Repeating geometry $N$ times with a 3D relative or constant spatial offset vector. | `TESTED` | `forge-modifiers` | P0 | Alpha 0.2 | Duplicates vertices/indices with affine translation offsets. | `test_array_modifier` | Curve-guided array and object offset planned for Phase 3. |
| **MOD-006** | Boolean Modifier | 4.6 | Constructive Solid Geometry (CSG): Union, Difference, Intersect. | `PLANNED` | `forge-geometry`, `forge-modifiers` | P1 | Phase 3 | Manifold polygon clipping, coplanar polygon handling, watertight output. | None | Requires robust exact arithmetic or BSP/CSG algorithm. |
| **MOD-007** | Bevel Modifier | 4.7 | Non-destructive chamfer/fillet across all or weight-limited sharp edges. | `PLANNED` | `forge-modifiers` | P1 | Phase 3 | Edge angle threshold filtering, multi-segment curved bevels. | None | Dependent on core Bevel algorithm (GEO-011). |
| **MOD-008** | Decimate Modifier | 4.8 | Mesh simplification using Quadric Error Metrics (QEM) to reduce polygon count. | `PLANNED` | `forge-geometry`, `forge-modifiers` | P2 | Phase 3 | Edge collapse prioritizing minimal surface deviation; target ratio slider. | None | QEM edge collapse queue to be implemented. |
| **MOD-009** | Remesh Modifier | 4.9 | Generating uniform quad or dual-contour manifold meshes from noisy input. | `PLANNED` | `forge-geometry`, `forge-modifiers` | P2 | Phase 3 | Voxel grid conversion, marching cubes or dual contouring reconstruction. | None | Computationally heavy; requires background threadpool. |

---

## 5. Sculpting

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SCL-001** | Sculpt Mode & Brush Engine | 5.1 | Interactive spherical deformation brush affecting vertices within radius. | `PLANNED` | `forge-geometry`, `apps/forge3d-studio` | P2 | Phase 5 | Falloff curve (smooth, linear, sphere), brush radius/strength controls. | None | Requires spatial acceleration for responsive sculpting. |
| **SCL-002** | Core Sculpt Brushes | 5.2 | Draw, Clay, Grab, Smooth, Inflate, Flatten, Pinch, Crease. | `PLANNED` | `forge-geometry` | P2 | Phase 5 | Mathematical deformation vectors applied per-vertex with symmetry. | None | Initial brush suite scheduled for Phase 5. |
| **SCL-003** | Dynamic Topology (Dyntopo) | 5.3 | Local edge splitting and collapsing under brush stroke to add geometric detail. | `PLANNED` | `forge-geometry` | P2 | Phase 5 | Dynamic mesh refinement based on screen-space brush size. | None | Requires fast local topological re-meshing. |
| **SCL-004** | Multiresolution Sculpting | 5.4 | Hierarchical Catmull-Clark levels with displacement delta sculpting. | `DEFERRED` | `forge-geometry` | P3 | Phase 5 | Switch between low-poly and high-poly levels preserving sculpted deltas. | None | High architectural complexity. |

---

## 6. UV Mapping & Texture Painting

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **UVM-001** | Dedicated UV Editor Panel | 6.1 | 2D canvas displaying flattened UV vertices, edges, and texture coordinates. | `PLANNED` | `apps/forge3d-studio` | P1 | Phase 5 | 2D pan/zoom canvas, UV selection, UV transform gizmos. | None | 2D viewport drawing pipeline required. |
| **UVM-002** | Seam Marking | 6.2 | Marking and clearing edge seams (`Ctrl+E` -> Mark Seam) highlighted in red. | `PLANNED` | `forge-geometry` | P1 | Phase 5 | Mesh edge attribute tracking seams; seam-aware boundary splitting. | None | Requires edge attribute domain. |
| **UVM-003** | UV Unwrapping Algorithms | 6.3 | Conformal (LSCM) and Angle-Based Flattening (ABF++) unwrapping. | `PLANNED` | `forge-geometry` | P2 | Phase 5 | Least-squares distortion minimization; automatic island generation. | None | Requires sparse linear solver (`nalgebra` / custom CG solver). |
| **UVM-004** | UV Island Packing | 6.4 | Bin packing algorithm arranging UV islands efficiently within $[0, 1]^2$ domain. | `PLANNED` | `forge-geometry` | P2 | Phase 5 | Minimizes wasted texture area, respects margin/padding slider. | None | 2D polygon bin-packing algorithm required. |
| **UVM-005** | Viewport Texture Painting | 6.5 | Direct 3D painting onto texture maps via raycast screen-to-UV projection. | `PLANNED` | `forge-gpu`, `forge-render` | P3 | Phase 5 | Brush stroke updates GPU texture; supports diffuse/roughness/normal layers. | None | Requires compute shader or CPU texture baking. |

---

## 7. Materials & Node-Based Shaders

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **MAT-001** | PBR Material Data Model | 7.1 | Metallic-Roughness Disney BRDF material definition (Albedo, Metallic, Roughness, Normal, Emissive). | `PLANNED` | `forge-scene`, `forge-render` | P0 | Alpha 0.3 | Struct with colors, scalar parameters, and texture asset references. | None | Currently single base color per object. |
| **MAT-002** | Texture Map Sampling | 7.2 | Loading and binding 2D texture maps (Albedo, Normal, Roughness, AO) in WGSL. | `PLANNED` | `forge-gpu`, `forge-render` | P0 | Alpha 0.3 | 2D texture sampler bind groups, sRGB vs Linear color space handling. | None | Texture loading pipeline scheduled for Alpha 0.3. |
| **MAT-003** | Node-Based Shader Graph | 7.3 | Visual node graph for procedural textures and math operations. | `PLANNED` | `forge-render`, `apps/forge3d-studio` | P2 | Phase 6 | Node editor canvas, dataflow execution, dynamic WGSL generation. | None | Advanced node architecture planned for Phase 6. |

---

## 8. Graphics Engine & Viewport

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **GFX-001** | Cross-Platform WGPU Backend | 8.1 | Portable GPU initialization with runtime discovery and OpenGL fallback. | `VERIFIED` | `forge-gpu` | P0 | Alpha 0.1 | Runs on Vulkan, Metal, DX12, and Mesa OpenGL (Intel HD 4000 verified). | `test_headless_gpu_init`, live smoke test | `hasvk` incomplete on Ivy Bridge; Gl fallback active. |
| **GFX-002** | Interactive 3D Viewport | 8.2 | Offscreen GPU texture rendered and composited with egui with zero input latency. | `VERIFIED` | `forge-render`, `apps/forge3d-studio` | P0 | Alpha 0.1 | Resize handling, 60 FPS repaint, depth testing, clear color. | `interaction_flow.rs`, live smoke test | Viewport resolution currently matches window panel. |
| **GFX-003** | Display Modes (Solid / Wireframe) | 8.3 | Solid Shaded, Wireframe, Material Preview, and Rendered viewport modes. | `IN_PROGRESS` | `forge-render` | P0 | Alpha 0.3 | Toggle between solid Blinn-Phong, wireframe lines, and textured shading. | `interaction_flow.rs` (Solid tested) | Dedicated Wireframe and Material preview modes in progress. |
| **GFX-004** | Ground Grid & RGB Axes | 8.4 | Infinite or finite ground grid with colored coordinate axes (+X red, +Y green, +Z blue). | `VERIFIED` | `forge-render` | P0 | Alpha 0.1 | Subdivided grid lines, depth fade, anti-aliased line rendering. | `test_grid_generation` | Dynamic screen-space adaptive grid scaling in progress. |
| **GFX-005** | Standard Camera Controls | 8.5 | Orbit (MMB / Alt+LMB / Empty LMB), Pan (Shift+MMB), Dolly/Zoom (Adaptive Smooth Scroll, Zoom to Cursor, Ctrl+MMB). | `VERIFIED` | `forge-render`, `apps/forge3d-studio` | P0 | Alpha 0.1 | Pitch clamping ($-89.4^\circ$ to $+89.4^\circ$), target pivot preservation, exponential adaptive zoom, cursor homothety, smooth easing. | `lmb_orbit_interaction.rs` (8/8), `adaptive_smooth_zoom.rs` (9/9), `camera.rs` (14/14) | Complete. |
| **GFX-006** | Fly / Walk Navigation | 8.6 | First-person WASD navigation with mouse-look, speed boost, and ground lock. | `VERIFIED` | `forge-render`, `apps/forge3d-studio` | P1 | Alpha 0.3 | `Shift+F` toggle, WASDQE motion, ground collision height lock (1.7m). | `test_camera_fly_movement_and_look` | Mesh collision during walk mode pending. |
| **GFX-007** | Scene Camera Object & Views | 8.7 | Persistent camera objects with focal length, sensor size, depth of field, and Camera View (`Num 0`). | `VERIFIED` | `forge-scene`, `apps/forge3d-studio` | P0 | Alpha 0.3 | 3D procedural camera marker, film gate overlay, focal length / FOV synchronization. | `test_simulation_scene_camera_object` | Bokeh blur post-pass not yet rendered in real-time. |

---

## 9. Lighting

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **LGT-001** | Directional Sun & Ambient | 9.1 | Default directional sunlight with configurable vector and ambient lighting. | `VERIFIED` | `forge-render` | P0 | Alpha 0.1 | Uniform buffer upload, Blinn-Phong specular calculation in WGSL. | `interaction_flow.rs` | Fixed single direction in Alpha 0.1. |
| **LGT-002** | Multiple Scene Lights | 9.2 | First-class Point, Directional, Spot, and Area light scene objects. | `PLANNED` | `forge-scene`, `forge-render` | P0 | Alpha 0.3 | Light objects in hierarchy, light gizmo markers, multi-light WGSL loop. | None | Scheduled for Alpha 0.3 scene & lighting milestone. |
| **LGT-003** | Shadow Mapping | 9.3 | Real-time dynamic shadow mapping (Cascaded Shadow Maps for sun, cubemaps for point). | `PLANNED` | `forge-gpu`, `forge-render` | P1 | Alpha 0.4 | Depth texture render pass, shadow comparison sampling in WGSL, PCF filtering. | None | Requires secondary depth pass per light. |
| **LGT-004** | Image-Based Lighting (IBL) | 9.4 | Environment lighting from HDR equirectangular maps (diffuse irradiance + specular prefilter). | `PLANNED` | `forge-render` | P1 | Alpha 0.4 | Cubemap generation, split-sum approximation, HDR background skybox. | None | Scheduled for Alpha 0.4. |

---

## 10. Real-Time & Path Tracing Rendering

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **RND-001** | Forward+ / Clustered Real-Time Renderer | 10.1 | Modern real-time PBR renderer handling dozens of dynamic lights efficiently. | `PLANNED` | `forge-render` | P1 | Phase 4 | Light grid culling, compute shader clustering, metallic/roughness evaluation. | None | Currently single-pass forward Blinn-Phong. |
| **RND-002** | Screen-Space Post-Processing | 10.2 | SSAO, Screen-Space Reflections (SSR), Bloom, Tone Mapping (ACES/Khronos PBR Neutral). | `PLANNED` | `forge-render` | P2 | Phase 4 | Fullscreen quad post-process passes, depth/normal buffer sampling. | None | Scheduled for Phase 4. |
| **RND-003** | Offline Path Tracer Engine | 10.3 | Physically accurate unidirectional path tracer with Monte Carlo integration. | `PLANNED` | `forge-render` | P2 | Phase 9 | SAH BVH construction, ray-triangle intersection, BSDF importance sampling. | None | Long-term milestone for offline production rendering. |
| **RND-004** | Hardware Ray Tracing Integration | 10.4 | Optional Vulkan / D3D12 hardware ray tracing acceleration where supported. | `DEFERRED` | `forge-gpu`, `forge-render` | P3 | Phase 10 | Bottom-level and top-level acceleration structures (BLAS/TLAS). | None | Hardware-gated; requires Vulkan 1.3+ / RT cores. |

---

## 11. Animation

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ANI-001** | Time & Frame Context | 11.1 | Global timeline state (FPS, current frame, start frame, end frame, play/pause). | `PLANNED` | `forge-scene`, `apps/forge3d-studio` | P1 | Phase 7 | Timeline transport controls, spacebar playback, deterministic frame stepping. | None | Scheduled for Phase 7. |
| **ANI-002** | Keyframe Data Structure | 11.2 | Generic keyframe storage with interpolation modes (Constant, Linear, Bezier). | `PLANNED` | `forge-scene` | P1 | Phase 7 | Curve evaluation, tangent handles, in-between frame interpolation. | None | Data structures to be designed in `forge-scene`. |
| **ANI-003** | Timeline & Dope Sheet UI | 11.3 | Bottom workspace panel displaying keyframe markers and tracks. | `PLANNED` | `apps/forge3d-studio` | P1 | Phase 7 | Interactive key dragging, box selection of keys, channel folding. | None | UI area currently reserved in layout. |
| **ANI-004** | Graph Editor | 11.4 | Visual F-Curve editor with interactive Bezier handles. | `PLANNED` | `apps/forge3d-studio` | P2 | Phase 7 | Curve visualization, tangent handle editing, curve extrapolation. | None | Scheduled for Phase 7. |

---

## 12. Rigging & Character Animation

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **RIG-001** | Armature & Bone Hierarchy | 12.1 | Hierarchical bone objects with head, tail, roll, rest matrix, and pose matrix. | `PLANNED` | `forge-scene` | P1 | Phase 7 | Bone tree evaluation, Edit Mode vs Pose Mode for armatures. | None | Scheduled for Phase 7. |
| **RIG-002** | Vertex Weighting & Skinning | 12.2 | Vertex skinning weights (up to 4 bones per vertex) with Linear Blend Skinning (LBS). | `PLANNED` | `forge-geometry`, `forge-render` | P1 | Phase 7 | GPU vertex skinning shader, weight painting brush interface. | None | Vertex layout will need bone index / weight attributes. |
| **RIG-003** | Inverse Kinematics (IK) | 12.3 | Two-bone and multi-bone analytical and numerical IK solvers (FABRIK). | `PLANNED` | `forge-scene` | P2 | Phase 7 | Target and pole vector constraints; fast CPU convergence. | None | Scheduled for Phase 7. |
| **RIG-004** | Shape Keys / Morph Targets | 12.4 | Linear blend morph targets for facial animation and corrective deformations. | `PLANNED` | `forge-geometry`, `forge-render` | P2 | Phase 7 | Relative delta vertex buffers; normalized weight sliders. | None | Scheduled for Phase 7. |

---

## 13. Procedural Geometry & Geometry Nodes

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **NOD-001** | Node Graph Core | 13.1 | Directed acyclic graph (DAG) data structure with typed sockets and dirty propagation. | `PLANNED` | `forge-core` | P1 | Phase 6 | Type-safe sockets (Float, Vector, Color, Geometry), cycle detection. | None | Scheduled for Phase 6. |
| **NOD-002** | Attribute System | 13.2 | Dynamic attribute domains (Point, Vertex, Edge, Face, FaceCorner) on mesh geometry. | `PLANNED` | `forge-geometry` | P1 | Phase 6 | Named and anonymous attribute buffers, attribute interpolation. | None | Essential foundation for nodes and USD. |
| **NOD-003** | Node Editor Workspace | 13.3 | Pan/zoom nodal canvas with wire bezier connections and search menu. | `PLANNED` | `apps/forge3d-studio` | P2 | Phase 6 | Node creation, socket connection dragging, live value inspection. | None | Dedicated workspace tab. |
| **NOD-004** | Point Instancing Nodes | 13.4 | Distributing points on surfaces and instancing mesh objects with random transform. | `PLANNED` | `forge-geometry`, `forge-scene` | P2 | Phase 6 | Poisson disk / random distribution on triangles, GPU instancing. | None | Scheduled for Phase 6. |

---

## 14. Physics Simulations

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PHY-001** | Rigid Body Simulation | 14.1 | Dynamic, static, and kinematic rigid body physics integration (Rapier3D). | `PLANNED` | `forge-scene` | P2 | Phase 8 | Gravity, restitution, friction, collision impulses, timeline synchronization. | None | Integration planned for Phase 8. |
| **PHY-002** | Cloth Simulation | 14.2 | Position-Based Dynamics (PBD) mass-spring cloth with self-collision and pinned vertices. | `PLANNED` | `forge-geometry` | P2 | Phase 8 | Stretch, shear, and bending constraints; collision against scene colliders. | None | Scheduled for Phase 8. |
| **PHY-003** | Simulation Caching & Baking | 14.3 | Deterministic caching of physics trajectories to disk (`.fphys` cache format). | `PLANNED` | `forge-scene` | P2 | Phase 8 | Scrubbing baked timeline without re-evaluating solver; point cache compression. | None | Scheduled for Phase 8. |
| **PHY-004** | Particle System | 14.4 | CPU/GPU particle emitter with velocity, lifetime, forces (wind, gravity), and sprite/mesh rendering. | `PLANNED` | `forge-render` | P2 | Phase 8 | Sub-emitter spawning, death events, collision with geometry. | None | Compute shader or CPU particle simulation. |

---

## 15. Visual Effects (VFX)

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **VFX-001** | Volumetric Fluids / Smoke | 15.1 | Grid-based fluid/smoke solver with density, temperature, and vorticity confinement. | `DEFERRED` | `forge-render` | P3 | Phase 9 | MacCormack advection, pressure Poisson solver, OpenVDB export. | None | High computation requirement. |
| **VFX-002** | Camera Motion Tracking | 15.2 | 2D feature point tracking and camera solve for integrating 3D assets into video. | `DEFERRED` | `forge-scene` | P3 | Phase 9 | KLT optical flow tracker, 8-point camera solver, undistort lens model. | None | Specialized CV domain. |

---

## 16. Compositing

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **CMP-001** | Node-Based 2D Compositor | 16.1 | Image processing graph combining render passes, color correction, and filters. | `DEFERRED` | `forge-render` | P3 | Phase 9 | Multi-layer OpenEXR reading, Alpha Over, Blur, Color Balance nodes. | None | Scheduled for Phase 9. |

---

## 17. 2D Graphics & Grease Pencil

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **DRW-001** | 3D Space 2D Vector Drawing | 17.1 | Freehand stroke drawing projected onto 3D surfaces or view planes. | `DEFERRED` | `forge-geometry`, `forge-render` | P3 | Phase 9 | Catmull-Rom smoothed stroke geometry, brush pressure, onion skinning. | None | Scheduled for Phase 9. |

---

## 18. Video Editing & Audio

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **VID-001** | Non-Linear Video Sequencer | 18.1 | Multi-track video and audio timeline sequencer with cuts, transitions, and audio sync. | `DEFERRED` | `apps/forge3d-studio` | P3 | Phase 9 | Video decoding (FFmpeg/native), waveform preview, audio mixing. | None | Scheduled for Phase 9. |

---

## 19. Asset Import & Export

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **IO-001** | Wavefront OBJ Import / Export | 19.1 | Fast parsing and generation of `.obj` and `.mtl` geometry and materials. | `VERIFIED` | `forge-io`, `forge-geometry`, `forge-scene` | P0 | Phase 5 | Robust parsing of vertices, normals, UVs, polygons, Ear Clipping 3D triangulation, negative indices; unified undo/redo. | `crates/forge-io/tests/*` (24/24), `apps/forge3d-studio/tests/obj_studio_integration.rs` (5/5) | Material libraries (.mtl) parsed as references; texture mapping scheduled for PBR sprint. |
| **IO-002** | glTF 2.0 / GLB Import / Export | 19.2 | Industry-standard 3D format support including PBR materials, transforms, and hierarchies. | `PLANNED` | `forge-scene`, `forge-render` | P0 | Alpha 0.3 | Node hierarchy, binary buffers, embedded textures, PBR parameters. | None | Core interchange format. |
| **IO-003** | STL Import / Export | 19.3 | ASCII and binary STL import and export for 3D printing workflows. | `PLANNED` | `forge-geometry` | P1 | Alpha 0.3 | Triangle mesh verification, binary header validation, scale preservation. | None | Simple, robust interchange. |
| **IO-004** | Universal Scene Description (USD) | 19.4 | OpenUSD (.usd, .usda, .usdc, .usdz) stage composition and interchange. | `DEFERRED` | `forge-scene` | P3 | Phase 10 | USD stage traversal, schema mapping, attribute synchronization. | None | Large dependency footprint; clean-room approach required. |

---

## 20. Architectural Visualization

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ARC-001** | Precision Measurement & Units | 20.1 | Metric (m, cm, mm) and Imperial (ft, in) unit system with live dimension measurements. | `PLANNED` | `forge-math`, `apps/forge3d-studio` | P1 | Phase 4 | Configurable display units, snapping to architectural increments. | None | Currently units are fixed meters. |
| **ARC-002** | Architectural Camera Projections | 20.2 | Two-point perspective camera mode with automatic vertical perspective tilt correction. | `PLANNED` | `forge-render` | P2 | Phase 4 | Viewport camera keeps vertical lines parallel to screen Y axis. | None | Mathematical projection adjustment. |

---

## 21. 3D Printing Preparation

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PRT-001** | Mesh Manifold & Watertight Validation | 21.1 | Automated verification of closed 2-manifold topology (no open boundaries, no non-manifold edges). | `PLANNED` | `forge-geometry` | P1 | Phase 4 | Highlights non-manifold edges in red; reports boundary count. | None | Validation checks exist in `validation.rs`; edge boundary count needed. |
| **PRT-002** | Overhang & Wall Thickness Analysis | 21.2 | Visual false-color heatmap of overhang angles ($>45^\circ$) and thin walls ($<1\text{mm}$). | `PLANNED` | `forge-render` | P2 | Phase 4 | Shader-based normal angle gradient; slice raycast wall testing. | None | Scheduled for 3D printing sprint. |

---

## 22. Scripting & Automation

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SCR-001** | Headless CLI Batch Execution | 22.1 | Command-line execution flags (`--background`, `--render`, `--eval-script`) without GUI. | `PLANNED` | `apps/forge3d-studio` | P1 | Phase 4 | Executes scenes, runs modifiers, exports renders/geometry in pure CLI mode. | None | Headless app harness exists (`new_headless`); CLI args parsing needed. |
| **SCR-002** | Embedded Scripting Engine | 22.2 | Safe embedded scripting (e.g. Rhai / Lua) exposing scene, mesh, and render APIs. | `PLANNED` | `forge-core`, `forge-scene` | P2 | Phase 10 | Sandboxed script execution, deterministic automation, custom macros. | None | Scheduled for ecosystem phase. |

---

## 23. Plugin Architecture

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PLG-001** | Plugin Manifest & Capability Model | 23.1 | Declarative plugin manifest (`plugin.toml`) defining permissions and hooks. | `PLANNED` | `forge-core` | P2 | Phase 10 | Secure capability declaration (Filesystem, Network, GPU access). | None | Security boundary per master specification. |
| **PLG-002** | Sandboxed WebAssembly Plugins | 23.2 | Isolated extension execution via Wasmtime/WASI with versioned engine APIs. | `DEFERRED` | `forge-core` | P3 | Phase 10 | Plugins cannot crash host process or perform unauthorized system calls. | None | Advanced ecosystem requirement. |

---

## 24. Resource Management

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **RES-001** | Dynamic GPU Buffer Management | 24.1 | Reusable GPU vertex, index, and uniform buffers with resizing and minimal reallocations. | `VERIFIED` | `forge-gpu`, `forge-render` | P0 | Alpha 0.1 | Buffers resize only when geometry overflows capacity; zero per-frame buffer leaks. | `ViewportRenderer` cache | Textures and materials not yet in unified pool. |
| **RES-002** | Asset Manager & Cache | 24.2 | Centralized asset registry with content-addressed caching and reference counting. | `PLANNED` | `forge-core` | P1 | Alpha 0.4 | De-duplicates loaded textures and meshes; releases unreferenced GPU resources. | None | New crate `crates/forge-asset` planned. |
| **RES-003** | Background Asynchronous Asset Loading | 24.3 | Offloading heavy file parsing and texture decompression to Rayon/worker threads. | `PLANNED` | `forge-scene` | P1 | Alpha 0.4 | Viewport remains responsive at 60 FPS while large assets load in background. | None | Scheduled for Alpha 0.4 asset pipeline. |

---

## 25. Project File System

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PRJ-001** | Native `.forge` JSON Schema v1 | 25.1 | Documented, versioned project schema storing scenes, objects, transforms, and meshes. | `VERIFIED` | `forge-scene` | P0 | Alpha 0.1 | Full save and load roundtrip testing; schema version check; safe error handling. | `test_project_roundtrip` | Meshes currently embedded in JSON; large scenes need binary container. |
| **PRJ-002** | Container Packaging Format (.forge binary) | 25.2 | Zip/container packaging storing `project.json` alongside external packed textures and binary buffers. | `PLANNED` | `forge-scene` | P1 | Phase 4 | Efficient binary storage; embedded vs relative external asset path support. | None | Format specification required in `docs/formats/`. |
| **PRJ-003** | Atomic Autosave & Crash Recovery | 25.3 | Background periodic autosave writing to temporary file with atomic rename (`.forge~`). | `PLANNED` | `forge-scene`, `apps/forge3d-studio` | P1 | Phase 4 | Recovers unsaved work upon unexpected crash; never corrupts target project file. | None | Scheduled for Phase 4. |

---

## 26. User Interface & Workspaces

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **UI-001** | Dark Graphite Professional Design | 26.1 | Refined dark theme styling (#1a1c21 base), restrained typography, and custom widgets. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.1 | Cohesive aesthetic, high contrast, clean typography, zero decorative fake buttons. | Live UI execution | Theme currently hardcoded in studio setup. |
| **UI-002** | Resizable Workspace Layout & Splitters | 26.2 | Top Menu, Resizable Left Toolbar (with drag splitter, Icon Only & Icon+Label modes, collapse), Central Viewport, Resizable Right Sidebar & Outliner/Inspector ratio, Bottom Timeline panel. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.2 | Interactive drag splitters with hover/drag accent lines, persistent panel sizes in `forge3d_settings.json`, clamped limits. | `alpha_0_2_ux_interaction.rs` (7/7), live smoke test | Dockable floating tabs planned for Phase 4. |
| **UI-003** | Unified 3D Interactive Transform Gizmos | 26.3 | Automatic Gizmo display on selection; Move (axes & planes), Rotate (rings), and Scale (boxes/axes); G/R/S hotkeys; multi-object transformation. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.2 | Precision ray-line and ray-plane intersections, 3 planar handles (XY/XZ/YZ), multi-selection median/active/cursor/individual pivots, global/local orientations, drag undo. | `alpha_0_2_ux_interaction.rs`, `interaction_flow.rs` | Complete. |
| **UI-004** | Modal Transform State Machine | 26.4 | Keyboard modal workflows: `G` Move, `R` Rotate, `S` Scale with X/Y/Z axis constraints, Shift+X/Y/Z plane constraints, precision numeric HUD, focus loss safety. | `VERIFIED` | `forge-scene`, `apps/forge3d-studio` | P0 | Alpha 0.2 | Precision numeric typing, Enter confirm, Esc cancel, window focus loss cancellation, UI text field priority. | `alpha_0_2_ux_interaction.rs`, `test_simulation_modal_*` | Complete. |
| **UI-005** | Viewport Telemetry Overlay | 26.5 | Real-time monospace debug card displaying pointer state, camera mode, and metrics. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.4 | Toggleable from View menu; displays coordinates, orbit target, distance. | `lmb_orbit_interaction.rs` | Complete. |
| **UI-006** | Searchable Command Palette | 26.6 | Quick command search modal (`F3` / Spacebar) executing any editor tool or action. | `PLANNED` | `apps/forge3d-studio` | P1 | Phase 4 | Fuzzy search over all registered actions and tools with hotkey hints. | None | Scheduled for Phase 4. |
| **UI-007** | Configurable Workspace Layouts | 26.7 | Preset workspaces (Modeling, UV Editing, Shading, Animation, Scripting). | `PLANNED` | `apps/forge3d-studio` | P2 | Phase 4 | Switching workspaces reconfigures active panels and editor tools. | None | Scheduled for Phase 4. |

---

## 27. Performance Optimization

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PRF-001** | Zero-Latency Input-First Frame Loop | 27.1 | Event processing executed prior to offscreen GPU render pass. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.1 | Zero-frame input lag on orbit, pan, zoom, and gizmo drag at 60 FPS. | Verified in ADR-0003 & live execution | Complete. |
| **PRF-002** | Broadphase AABB Raycast Acceleration | 27.2 | Fast Kay-Kajiya slab box testing with degenerate plane inflation ($\epsilon=0.005$). | `VERIFIED` | `forge-math` | P0 | Alpha 0.1 | Rapid elimination of non-intersecting objects prior to triangle tests. | `bounds::tests` (4/4) | Full scene BVH needed for >10,000 objects. |
| **PRF-003** | Low-Resource Hardware Baseline | 27.3 | Smooth operation on Intel Core i5 Ivy Bridge + Intel HD Graphics 4000 under Linux. | `VERIFIED` | `forge-gpu`, `apps/forge3d-studio` | P0 | Alpha 0.1 | Starts in ~600ms, consumes ~110MB idle RAM, 60 FPS viewport on Mesa Gl. | Measured baseline benchmarks | Target verified on actual user machine. |
| **PRF-004** | Spatial Acceleration Hierarchy (BVH / Octree) | 27.4 | Bounding Volume Hierarchy for complex meshes ($>100\text{k}$ triangles) and large scenes. | `PLANNED` | `forge-math`, `forge-geometry` | P1 | Phase 3 | Sub-millisecond raycast picking and selection on dense meshes. | None | Scheduled for Phase 3. |
| **PRF-005** | Multi-Threaded CPU Parallelism (Rayon) | 27.5 | Parallelizing modifier evaluation, mesh normal calculation, and file imports. | `PLANNED` | `forge-geometry`, `forge-modifiers` | P1 | Phase 3 | Scales linear operations across all available CPU cores without stalls. | None | Rayon dependency to be added where measured. |

---

## 28. Security & Stability

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SEC-001** | Renderer-Agnostic Domain Crates | 28.1 | Strict architectural prohibition of GPU handles or UI imports in core domain crates. | `VERIFIED` | `forge-core`, `forge-math`, `forge-geometry`, `forge-scene` | P0 | Alpha 0.1 | `wgpu` and `egui` never imported in domain crates; pure-CPU headless tests. | Crate dependency checks, CI | Verified via workspace architecture rules. |
| **SEC-002** | Safe Rust Enforcement | 28.2 | `#![forbid(unsafe_code)]` or zero unsafe blocks across entire active workspace. | `VERIFIED` | All crates | P0 | Alpha 0.1 | 100% safe Rust code; no raw pointer dereferences in active crates. | Clippy, codebase audit | Zero unsafe blocks across all 7 crates. |
| **SEC-003** | Focus Loss & Pointer Capture Safety | 28.3 | Automatic state reset to `Idle` on focus loss or mouse release outside window. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.4 | Prevents stuck drag modes or orphaned interaction states. | `test_focus_loss_resets_dragging_state` | Complete. |
| **SEC-004** | Hotkey Isolation & Text Protection | 28.4 | Viewport hotkeys blocked when UI text fields have focus (`egui_wants_keyboard_input`). | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.3 | Typing names or numbers never accidentally triggers modal transforms or deletes. | `viewport.rs`, manual scenario | Complete. |

---

## 29. Testing Infrastructure & Quality Gates

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **TST-001** | Unit & Integration Test Suite | 29.1 | Comprehensive pure-CPU automated tests across all domain and application crates. | `VERIFIED` | All crates | P0 | Alpha 0.1 | 80/80 tests passing with zero failures, running in < 0.3s. | Automated test runner (`cargo test --workspace`) | Complete for current code. |
| **TST-002** | Quality Gate Checks | 29.2 | Enforced `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, and release builds. | `VERIFIED` | Workspace | P0 | Alpha 0.1 | Zero formatting violations, zero clippy warnings, clean release compilation. | CLI quality gates | Verified in continuous development. |
| **TST-003** | Headless Application Simulation | 29.3 | Testing end-to-end user workflows (selection, modal transforms, undo/redo) without GPU. | `VERIFIED` | `apps/forge3d-studio` | P0 | Alpha 0.3 | `new_headless()` simulates complete application state transitions in CI. | `interaction_flow.rs` (8/8) | Complete. |
| **TST-004** | Offscreen Render Golden Image Tests | 29.4 | Headless WGPU offscreen rendering to image buffer with pixel-diff perceptual hashing. | `PLANNED` | `forge-render` | P1 | Alpha 0.4 | Detects visual regressions in shaders, lighting, and materials automatically. | None | Scheduled for Alpha 0.4. |

---

## 30. Development Roadmap

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **RDM-001** | Phase 1: Core Stability | 30.1 | Native window, wgpu viewport, camera navigation, primitives, transforms, undo/redo, `.forge` persistence. | `VERIFIED` | All initial crates | P0 | Phase 1 | Reliable desktop 3D editor with verified stability and hardware baseline. | 71/71 tests passing | Alpha 0.1 / 0.2 completed. |
| **RDM-002** | Phase 2: Professional Modeling | 30.2 | Complete polygonal modeling suite (Loop Cut, Bevel, Knife, Bridge, Inset, Extrude). | `IN_PROGRESS` | `forge-geometry` | P0 | Phase 2 | Users can model complex polygon meshes directly inside Forge3D. | EditMesh tests active | Core operations active; advanced tools next. |
| **RDM-003** | Phase 3: Modifier Pipeline | 30.3 | Extended modifier stack (Boolean, Bevel, Decimate, Remesh, Displace, Shrinkwrap). | `IN_PROGRESS` | `forge-modifiers` | P1 | Phase 3 | Complex non-destructive workflows with caching and high-speed evaluation. | 5 modifier tests | Mirror, Subdiv, Solidify, Array active. |
| **RDM-004** | Phase 4: Materials & Rendering | 30.4 | PBR materials, multi-light scene lighting, shadow mapping, environment maps, OBJ/glTF I/O. | `PLANNED` | `forge-render`, `forge-io` | P1 | Phase 4 | Full scene visual rendering, material preview, open format interoperability. | None | Next major development phase. |
| **RDM-005** | Phase 5–10 Production Milestones | 30.5 | UV & Sculpting, Procedural Nodes, Animation & Rigging, Physics, VFX, Ecosystem. | `PLANNED` | Multi-crate | P2–P4 | Phase 5–10 | Long-term evolution into a universal 3D creation suite. | None | Staged sequentially per master spec. |

---

## 31. Future Technologies

| Feature ID | Feature Name | Spec Section | Description | Status | Dependencies | Priority | Planned Phase | Acceptance Criteria | Test Coverage | Known Limitations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **FUT-001** | WebGPU Browser Target (WASM) | 31.1 | Compiling Forge3D studio to WebAssembly with WebGPU for browser-based 3D editing. | `PLANNED` | `apps/forge3d-studio` | P3 | Phase 10 | Runs natively in Chromium/Firefox with WebGPU without installation. | None | WebGPU standard stabilizing; wgpu supports wasm32-unknown-unknown. |
| **FUT-002** | OpenUSD Composition & Cloud Collaboration | 31.2 | Real-time multi-user scene editing over local network or cloud with USD live syncing. | `DEFERRED` | `forge-scene` | P4 | Phase 10 | Conflict-free replicated data types (CRDTs) for simultaneous 3D editing. | None | Long-term frontier capability. |

---

*Last Synchronized:* 2026-10-31 | *Audited by:* Principal Software Architect & Lead Graphics Engineer
