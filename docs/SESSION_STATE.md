# Forge3D Rust Edition — Session State

**Document Status:** Current  
**Last Updated:** 2026-11-05  
**Current Milestone:** Master Specification Full Implementation — Section 8-11, 21-24 (PBR Materials, STL/PLY/glTF/GLB I/O, Modifiers, Sculpting, Workspaces, Command Palette, UV Suite, Scripting & Plugin System)

---

## 1. System & Hardware Reconnaissance
- **OS**: Linux Lenovo ThinkPad T530 (Kernel 7.0.0-38-generic #38-Ubuntu x86_64)
- **Display Server**: Active (Wayland `wayland-0` and X11 `:0`)
- **GPU Hardware**: Intel Corporation 3rd Gen Core processor Graphics Controller (Ivy Bridge HD Graphics 4000)
- **Active Backend**: Mesa OpenGL (`Gl`) fallback via `wgpu 30.0.1` (Vulkan `hasvk` incomplete on Ivy Bridge)
- **Toolchain**:
  - `rustc 1.99.0 (b940084d7 2026-09-28)`
  - `cargo 1.99.0 (5f94df478 2026-08-27)`
  - `git 2.53.0` (initialized on `main` branch)
  - Components: `rustfmt`, `clippy` installed and active
- **Quality Gates**:
  - 100% passing tests across all crates (238 geometry tests, 48 IO/mesh tests, GPU picking tests, and studio integration tests).
  - Zero compiler or Clippy warnings with `-D warnings`.
  - Zero `cargo fmt` diffs.

---

## 2. Completed Architectural Modules

### `crates/forge-core`
- Generational / atomic `ObjectId` and `AssetId` with `Display`, `Debug`, `Hash`, `Eq`, and `serde`.
- `Identifiable` and `Named` traits.
- Domain error handling via `CoreError` with `thiserror`.

### `crates/forge-math`
- Coordinate conventions: Right-handed, Y-up, distance in meters.
- Affine `Transform` with matrix composition (`mul_transform`), inversion, point/vector transformation, and degrees-to-quaternion Euler conversion.
- `Aabb`: Axis-aligned bounding box with encapsulation, ray intersection (slab method with $\epsilon = 0.005$ degenerate slab inflation for flat 2D planes), center, extents, union, and 3D affine transform.
- `Ray`: 3D ray with ray-plane, ray-sphere, AABB, and Möller–Trumbore triangle intersection algorithm.

### `crates/forge-geometry`
- `Vertex`: 48-byte WGSL-aligned struct (`Pod`, `Zeroable`) with position [f32; 3], normal [f32; 3], uv [f32; 2], color [f32; 4].
- `LineVertex`: 28-byte struct (`Pod`, `Zeroable`) with position [f32; 3] and color [f32; 4].
- Procedural Generators:
  - `generate_cube(size)`: 24 vertices, 36 indices, outward unit face normals, per-face UVs.
  - `generate_uv_sphere(radius, sectors, stacks)`: smooth outward normals, welded poles, seamless UVs.
  - `generate_icosphere(radius, subdivisions)`: regular icosahedron with recursive midpoint subdivision.
  - `generate_plane(size, subdivisions)`: triangulated XZ grid with +Y normals.
  - `generate_cylinder(radius, height, sectors)`: lateral wall with cap disc triangulation.
  - `generate_cone(radius, height, sectors)`: base disc and apex fan triangulation.
  - `generate_torus(major_r, minor_r, major_s, minor_s)`: circular torus mesh with outward normals.
  - `generate_circle(radius, sectors)`: flat disc mesh with center fan.
  - `generate_capsule(radius, cylinder_height, sectors, rings)`: hemispherically capped cylinder with outward unit normals.
  - `generate_monkey(scale)`: clean-room procedural low-poly mascot head (42 vertices, 40 faces).
  - `generate_grid_lines(half_size, step)`: major/minor grid lines + colored coordinate axes (+X red, +Y green, +Z blue).
- Invariant Validation: `validate_mesh_data` checking for NaNs, infinites, out-of-range indices, duplicate indices, and degenerate zero-area triangles.
- B-Rep PolyMesh (`EditableMesh`):
  - Extrude, Inset, Subdivide, Bevel vertices/edges, Loop Cut & Slide, Knife connect vertices, Bisect planar cut, Spin lathe/screw revolve, Dissolve edges, Bridge edge loops, Separate faces, Weld co-located vertices, Tris-to-quads, and Proportional Editing.
- Sculpting Brush Engine (`sculpt.rs`):
  - 9 brushes: `Draw`, `Clay`, `Smooth`, `Flatten`, `Inflate`, `Deflate`, `Grab`, `Pinch`, `Crease`.
  - Cubic spherical falloff $w(d) = (1 - (d/r)^2)^3$ with mirror symmetry across X, Y, and Z.
- UV Unwrapping & Projection Suite (`uv.rs`):
  - `smart_project_uvs`: clusters coplanar faces into islands and packs into $[0, 1] \times [0, 1]$.
  - `box_project_uvs`, `sphere_project_uvs`, `cylinder_project_uvs`.
  - `pack_uv_islands`: normalizes and packs existing UVs with configurable margin.

### `crates/forge-modifiers`
- Non-destructive modifier stack:
  - `SubdivisionModifier`: Catmull-Clark and Loop smoothing.
  - `MirrorModifier`: X, Y, Z mirror axes with vertex welding.
  - `ArrayModifier`: Linear instances with relative/constant offsets.
  - `BooleanModifier`: CSG Union, Difference, Intersect.
  - `SolidifyModifier`: Surface shell thickening along normals.
  - `BevelModifier`: Procedural edge chamfering with segment count and profile curvature.
  - `DecimateModifier`: Planar quadric simplification.
  - `DisplaceModifier`: Normal/noise procedural displacement.
  - `WeightedNormalModifier`: Area and angle weighted smooth normals.

### `crates/forge-scene`
- `SceneObject`: Encapsulates ID, name, transform, visibility, primitive type, mesh, and base color.
- `Scene`: Object hierarchy, parent-child world transform accumulation, raycast picking, dirty tracking, and `create_default_scene()`.
- `SelectionState`: Active and hovered selection management.
- `EditorState`: Active mode, workspace modes (`Modeling`, `Sculpting`, `UvEditing`, `Shading`, `Animation`, `Rendering`, `GeometryNodes`, `Scripting`), gizmos, panels, and snapping.
- `ExtensionManager` (`extension.rs`):
  - Safe plugin lifecycle (`Enabled`, `Disabled`).
  - Manifest metadata (`PluginManifest`) and runtime sandbox permissions (`FileRead`, `FileWrite`, `SceneMutate`, `Network`).
  - Custom operator registration and execution validation.
  - Event hook telemetry (`SceneLoaded`, `ObjectAdded`, `ObjectRemoved`, `FrameRendered`, `ProjectSaved`).
- `ForgeProject`: JSON schema v2 serialization for `.forge` project files with version checks, backward-compatible v1 material migration, and roundtrip testing.

### `crates/forge-gpu`
- `GpuContext`: Adapter capability discovery and headless initialization with graceful fallback.
- `GpuAdapterInfo`: Summary formatting for diagnostics and UI status bar.
- `GpuBuffer`: Typed wrappers for Vertex, Index, and Uniform GPU buffers.

### `crates/forge-render`
- `Camera`: Perspective and Orthographic camera with Orbit, Pan, Dolly, Exponential Adaptive Zoom, Cursor-Anchored Zoom, and Smooth Inertial Easing.
- WGSL Shaders:
  - `MESH_SHADER`: Cook-Torrance/GGX metallic-roughness shading with base-color, normal, metallic, roughness, AO, and emissive maps; scene-driven lights; shadow comparison; and ACES tone mapping.
  - `GRID_SHADER`: Line list pipeline for ground grid and RGB coordinate axes.
- `RenderPipelines`: WGPU render pipelines with depth testing (`Depth32Float`, LessEqual).
- `ViewportRenderer`: Viewport renderer with depth texture management, dynamic GPU mesh cache, uniform uploads, and command encoding.

### `crates/forge-io`
- Zero-GPU / zero-UI decoupled asset pipeline crate:
  - Wavefront OBJ: import/export with 3D Ear Clipping triangulation.
  - STL: ASCII and binary import/export with 84-byte length validation and spatial welding.
  - Stanford PLY: ASCII and Little-Endian binary import/export with custom attributes.
  - glTF 2.0 / GLB: Clean-room parser and exporter for binary GLB containers and self-contained glTF files with embedded Base64 data URIs. Preserves mesh primitives, node transforms, and PBR metallic-roughness materials.
- Atomic Undo/Redo integration via `Action::ImportObjects` in `forge-scene`.

### `apps/forge3d-studio`
- Native desktop application built on `eframe` (v0.37) and `egui`.
- Blender-style Command Palette (`F3` / `Space` / `Ctrl+K`) with 50+ operators, live search filtering, and arrow key execution.
- Workspaces switcher supporting all 8 core workflows.
- Live Viewport raycast sculpting interaction with brush shelf, radius/strength sliders, and symmetry toggles.
- Extension & Plugin Manager modal dialog with runtime permission tags, enable/disable controls, and telemetry log viewer.
- Full File Import/Export menu suite supporting Wavefront OBJ, STL, Stanford PLY, and glTF 2.0 / GLB.

---

## 3. Verification & Test Evidence
- **All Tests**: 450 passed, 0 failed across the full workspace, including the PBR material bind-group/render smoke test and v1 project migration.
- **`cargo fmt --check`**: Clean (exit code 0).
- **`cargo check --workspace`**: Clean (exit code 0).
- **`cargo clippy --workspace --all-targets -- -D warnings`**: Clean (exit code 0, zero warnings).
- **GPU Adapter Test**: Verified live rendering on `Mesa Intel(R) HD Graphics 4000 (IVB GT2)` using the OpenGL backend.
- **Release Build & PBR Material Load**: `cargo build --release -p forge3d-studio` built cleanly; runtime log confirmed Brown Floor Tiles loaded at 4096×4096 and Bark Brown 01, Wooden Planks, and Mossy Rock at 2048×2048. All four-map sets (base color, normal, roughness, AO) loaded, using about 298.7 MiB total GPU texture memory including mipmaps. Target: `/home/dropt1m/Forge3D Rust Edition/target/release/forge3d-studio`.

---

## 4. Completed Milestones & Git Commits
- **Alpha 0.1 & 0.2**:
  - Foundational crates, wgpu backend on Intel HD 4000, interactive viewport rendering, editable B-Rep mesh, modifier stack, undo/redo history.
- **Alpha 0.3 — Full 3D Control System (Phases 1–6)**:
  - **Phase 1 (Camera Foundation)**:
    - Orbit (MMB / Alt+LMB), Pan (Shift+MMB), Dolly Zoom (Ctrl+MMB), Zoom (Scroll).
    - Fly & Walk Navigation (`Shift+F`): WASD movement, Q/E vertical translation, Shift speed boost, Ctrl slow-down, mouse look, ground height locking.
    - Standard views: Numpad 1/3/7 Front/Right/Top, Ctrl+1/3/7 Back/Left/Bottom, Numpad 5 Ortho/Persp, Numpad 0 Camera View, Numpad 2/4/6/8 15° step rotation.
    - Emulate Numpad option for compact/laptop keyboards.
    - Framing: Frame Selected (`F` / Numpad `.`) and Frame All (`Home`).
  - **Phase 2 (Object Manipulation & Modal Transforms)**:
    - Modal Grab/Move (`G`), Rotate (`R`), Scale (`S`) finite state machine.
    - Axis locks (`X`, `Y`, `Z`), plane locks (`Shift+X/Y/Z`), repeat axis press toggles Global vs Local space.
    - Real-time precision numeric input (digits, `-`, `.`, Backspace) with HUD overlay.
    - Confirm with `Enter`/`LMB`, cancel with `Escape`/`RMB`.
    - Multi-object transforms with Pivot Points: Median Point, Active Element, 3D Cursor, Individual Origins.
    - Duplicate (`Shift+D`), Delete (`X`/`Del`), Hide (`H`), Unhide All (`Alt+H`), Reset Transforms (`Alt+G/R/S`), Apply Transform (`Ctrl+A`), Parent/Unparent (`Ctrl+P`/`Alt+P`), Join Objects (`Ctrl+J`), Set Origin (to Geometry, to Cursor).
  - **Phase 3 (Interactive Gizmos)**:
    - On-screen Move, Rotate, Scale manipulators with hover highlights and grid snapping (`🧲`).
  - **Phase 4 (Keyboard Workflow & Input Architecture)**:
    - Isolated keyboard routing: `egui_wants_keyboard_input` protects text fields in Inspector/Outliner.
    - Hierarchical priority routing eliminating shortcut conflicts.
  - **Phase 5 (Scene Camera Objects)**:
    - `PrimitiveType::Camera` scene object with procedural 3D camera marker mesh.
    - Active camera management, outliner `★` badge, Camera View overlay with film gate letterboxing.
    - Optical properties: Perspective/Orthographic, Focal Length / FOV synchronization, Sensor dimensions, Clipping, Depth of Field, Resolution presets (1080p, 4K, 1:1), and Align Active Camera to View (`Ctrl+Alt+Num0`).
    - Full serialization in `.forge` project files.
  - **Phase 6 (Stability & Quality Gates)**:
    - 50/50 tests passing, clippy zero warnings, cargo fmt clean, release binary verified on live hardware.
- **Alpha 0.2 — Interaction & Workspace UX**:
  - **Automatic Object Interaction**: Transform gizmo automatically displayed upon selecting objects; immediate rebinding on new selection; multi-object transformation with Median, Active, Cursor, and Individual pivots in Global/Local orientations.
  - **Keyboard Transform System**: Direct G/R/S modal transforms, X/Y/Z axis constraints, Shift+X/Y/Z plane constraints, real-time numeric typing, focus loss safety, UI text input priority.
  - **Unified Selection & Gizmo**: LMB click selects and reveals gizmo; gizmo drag manipulates object(s); G/R/S transforms from keyboard; click empty space deselects (< 4px) or orbits (>= 4px); Shift+LMB multi-select; Box Select (`B`).
  - **Resizable Workspace Layout**: Left Toolbar resizable (48–240px, Icon Only / Icon + Label modes, collapse); Right Sidebar resizable (220–600px); Outliner/Properties vertical split (0.15–0.85); Bottom Timeline (60–350px); drag splitters with hover/drag accent `#f09020`; persistent settings in `forge3d_settings.json`.
  - **Categorized Toolbar**: Selection, Transform, Modeling, Add Mesh, Utilities.
  - **Add Object Expansion**: Added procedural clean-room Capsule and Monkey (Suzanne mascot) primitives; searchable categorized Add Menu.
  - **Test Suite**: 80/80 tests passing across all workspace crates and integration tests (`alpha_0_2_ux_interaction`, `interaction_flow`, `lmb_orbit_interaction`).
- **Alpha 0.2.1 — Free Cursor Object Dragging**:
  - **World-Space Drag Calculation**: Camera-aligned working plane passing through initial grab point with normal opposing camera forward (`-camera.forward().normalize()`). Ray-plane projection tracking cursor movement with zero perspective skewing.
  - **Zero-Jump Grab Point Preservation**: Initial offset between grab point and each object centroid retained throughout drag (`initial_offset = obj.translation - hit_point`).
  - **Multi-Object Synchronization**: Dragging any selected object synchronously translates all selected objects, preserving their relative positions, recorded as a single atomic `Action::MultiTransform` in undo history.
  - **Input Priority Enforcement**: Strict priority order: UI Interaction (`!response.hovered()`) > Gizmo Handles (`gizmo_consumed`) > Object Drag > Selection > Box Selection > Viewport Orbit. Viewport canvas widget is correctly identified via `response.hovered()`, avoiding spurious self-capture by `egui_wants_pointer_input()`.
  - **Edge Case Protection**: Safe rejection of parallel rays (`|denom| < 1e-4`), distance clamp ($t \le 10000.0$), delta clamp ($|\Delta| \le 5000.0$), window focus loss cancellation, and `Escape` key rollback.
  - **Test Suite**: 14 dedicated automated tests in `tests/free_cursor_drag.rs`, bringing workspace total to **94/94 tests passing**.

---

## 5. Master Keyboard & Navigation Reference

| Action | Shortcut | Context |
| :--- | :--- | :--- |
| **Free Object Drag** | `LMB + Drag on geometry` | Viewport (Object Mode) |
| **Cancel Drag** | `Escape` | During Drag |
| **Orbit Camera** | `LMB + Drag (empty space)`, `MMB + Drag`, `Alt + LMB + Drag` | Viewport |
| **Pan Camera** | `Shift + MMB + Drag` | Viewport |
| **Zoom / Dolly** | `Scroll Wheel` or `Ctrl + MMB + Drag` | Viewport |
| **Box Select** | `B + Drag` or Toolbar Box Select Button | Viewport |
| **Fly / Walk Mode** | `Shift + F` (Exit with `Esc` / `Enter`) | Viewport |
| **Fly Movement** | `W` (Fwd), `S` (Back), `A` (Left), `D` (Right), `Q` (Down), `E` (Up) | Fly Mode |
| **Fly Speed Modifiers** | `Shift` (Boost), `Ctrl` (Slow), `Scroll` (Base Speed) | Fly Mode |
| **Standard Views** | `Num 1` (Front), `Ctrl+Num 1` (Back), `Num 3` (Right), `Ctrl+Num 3` (Left), `Num 7` (Top), `Ctrl+Num 7` (Bottom) | Viewport |
| **Projection Toggle** | `Num 5` (Perspective / Orthographic) | Viewport |
| **Step Rotate** | `Num 2` / `Num 8` (Pitch 15°), `Num 4` / `Num 6` (Yaw 15°) | Viewport |
| **Camera View** | `Num 0` (Toggle View through Active Scene Camera) | Viewport |
| **Align Camera to View** | `Ctrl + Alt + Num 0` | Viewport |
| **Frame Selected / All** | `F` / `Num .` (Selected), `Home` (All) | Viewport |
| **Grab / Move** | `G` (Move), `G → X/Y/Z → [Number] → Enter` | Object Selection |
| **Rotate** | `R` (Rotate), `R → X/Y/Z → [Degrees] → Enter` | Object Selection |
| **Scale** | `S` (Scale), `S → X/Y/Z → [Factor] → Enter` | Object Selection |
| **Plane Constraint** | `Shift + X` (YZ), `Shift + Y` (XZ), `Shift + Z` (XY) | Modal Transform |
| **Toggle Local/Global** | Press axis key again (`X`, `Y`, `Z`) | Modal Transform |
| **Cancel Transform** | `Escape` or `RMB` | Modal Transform |
| **Confirm Transform** | `Enter` or `LMB` | Modal Transform |
| **Duplicate Object** | `Shift + D` (Duplicates and starts Grab) | Object Selection |
| **Delete Object** | `X` or `Delete` | Object Selection |
| **Hide / Unhide** | `H` (Hide Selected), `Alt + H` (Unhide All) | Object Mode |
| **Parent / Unparent** | `Ctrl + P` (Set Parent to Active), `Alt + P` (Clear Parent) | Multi-Selection |
| **Join Objects** | `Ctrl + J` (Join selected into active mesh) | Multi-Selection |
| **Reset Transforms** | `Alt + G` (Location), `Alt + R` (Rotation), `Alt + S` (Scale) | Object Selection |
| **Apply Transform** | `Ctrl + A` (Bake transform into geometry) | Object Selection |
| **Undo / Redo** | `Ctrl + Z` (Undo), `Ctrl + Shift + Z` / `Ctrl + Y` (Redo) | Global |

---

## 6. How to Run & Verify
```bash
cd "/home/dropt1m/Forge3D Rust Edition"
cargo run --release -p forge3d-studio
```

---

## 7. Master Audit Summary & Immediate Next Steps
- **Audit Deliverables Completed**:
  - `docs/FEATURE_MATRIX.md` generated covering all 31 specification domains with status, priority, dependencies, acceptance criteria, and limitations.
  - `docs/ROADMAP.md` updated with strategic P0–P4 priority tiers, 10-phase master progression, and rigorous exit criteria.
  - `docs/BENCHMARKS.md` updated with real measured hardware performance baseline on Intel Core i5 Ivy Bridge + Mesa OpenGL.
  - `docs/DECISIONS.md` expanded with ADR-0012 recording the audit and roadmap decisions.
- **Phase 5 Milestone Completed**:
  - Implemented `crates/forge-io` Wavefront OBJ Parser, Importer, and Exporter.
  - Ear Clipping triangulation for convex/concave $N$-gons ($N \ge 3$).
  - Negative and relative index resolution at parse time.
  - Native file dialogs integrated in `forge3d-studio` (`File -> Import/Export`).
  - Atomic Undo/Redo (`Action::ImportObjects`) and full world-transform baking during export.
  - All workspace tests passing: **149/149**.
- **Immediate Next Tasks**:
  1. Phase 4: PBR materials & direct/environment lighting pipeline in `crates/forge-render`.
  2. Implement glTF 2.0 / GLB import and export in `crates/forge-io`.
  3. Expand `crates/forge-geometry` with B-Rep modeling operations (Loop Cut, Bevel, Dissolve).

## 8. Built-in Showcase Scene (2026-11-04)
- Expanded `Scene::create_showcase_scene()` with a porch, roof trim, window muntins and sills, tree branches and foliage highlights, garden fence, shrubs, flowers, and individual stepping stones.
- The graphical app opens with a timed build walkthrough: terrain, house shell, roof, facade, tree, then rock and garden details appear over roughly 8.5 seconds. The finished scene remains fully visible.
- Added an in-viewport virtual keyboard and animated pointer, explicitly labeled as simulated visual playback that sends no operating-system input. `Examples -> Replay House Build Demo` replays the sequence for screen capture.
- Baked procedural vertex-color patterns into tree bark and branches, fence rails and pickets, the main rock, pebble, and stepping stones.
- `File -> New Scene` and headless app initialization retain the existing default scene; reloading the showcase still clears transient selection/edit state and invalidates renderer caches.
- Validation at this milestone: `cargo fmt --all -- --check`, `cargo check --workspace`, and the full workspace tests passed. The current strict workspace Clippy run also passes.

## 9. Poly Haven Brown Floor Tiles PBR Material (2026-11-04)
- Added the CC0 4K albedo, OpenGL normal, roughness, and AO maps from Poly Haven at `assets/textures/brown_floor_tiles/`, with source and license notes.
- Added serde-compatible optional material IDs to `SceneObject`; the showcase Front Path and Porch Deck now select `polyhaven/brown_floor_tiles`.
- Added one-time GPU texture upload, CPU-generated mipmaps, sRGB albedo sampling, linear data maps, tangent-space normal reconstruction, GGX direct light, and AO-modulated ambient fill in the mesh pass. All other objects use a shared 1×1 default material.
- Runtime map resolution is limited by `max_texture_dimension_2d`; at full 4K, the four maps and mip chains use about 170.7 MiB on the GPU. Decode/upload happens once on the first rendered frame containing the material. A live release run confirmed the full-resolution load on the Intel HD 4000/OpenGL path.
- This initial texture-only implementation is superseded by the scene-driven material and lighting system below.

## 10. Scene-Driven PBR and Lighting System (2026-11-04)
- `forge-scene` now owns serializable `PbrMaterial`, `PbrTextureSet`, `LightData`, and `WorldLighting` domain data; GPU handles remain renderer-owned.
- Materials support base color, tangent normal, metallic, roughness, ambient occlusion, and emissive maps, plus linear factors, normal/AO strength, and UV scale. Every mesh uses the shared PBR layout; missing maps receive constant defaults.
- Scene lights support directional, point, and spot types, up to 16 per view. Point/spot lights use inverse-square falloff and finite range; world fill blends sky and ground colors by surface normal.
- First directional light renders a 1024×1024 shadow map. The PBR viewport uses ACES tone mapping and inverse-transpose normal transforms.
- Replaced the placeholder inspector with working material assignment/creation, map path/file-picker fields, PBR sliders, selected-light controls, and world fill controls. Diffuse and metal shelf actions now assign actual scene materials.
- `.forge` schema is version 2. Version-1 scenes load with defaults and recover the bundled brown tile material by its existing ID.
- Bundled CC0 materials now include 4K brown floor tiles and 2K bark, wooden planks, and mossy rock; the showcase assigns them to the path/deck, tree, fence/porch timber, and rocks/stepping stones. Each set provides base color, OpenGL normal, roughness, and AO maps.
- Version-1 showcase migration also restores bark, wood, and rock material assignments and clears procedural vertex tint where it would darken the albedo.
- Texture paths are project-relative when bundled and may also be absolute. The application package must include the source map files.
- Remaining viewport limits: hemispheric ambient fill rather than HDR IBL; shadow map applies to the first directional light only; point/spot lights do not cast shadows; no cutout shadowing or height/displacement mapping.
- Validation: `cargo fmt --all -- --check`, `cargo check --workspace`, strict workspace Clippy, and all 450 workspace tests passed. The release app launched on Mesa OpenGL and loaded all four PBR sets: 4K tiles plus three 2K materials, about 298.7 MiB of GPU texture storage including mipmaps.

## 11. STL & Stanford PLY Asset Pipeline (2026-11-05)
- Implemented clean-room STL parser, importer, and exporter in `crates/forge-io/src/stl/`.
  - Supports ASCII and Binary STL formats with automatic format detection.
  - Robust against binary files whose 80-byte header starts with `"solid"`.
  - Spatial quantization vertex welding with configurable tolerance and area-weighted smooth normal recalculation.
  - Exporter bakes world transforms and formats as Binary or ASCII.
- Implemented clean-room Stanford PLY parser, importer, and exporter in `crates/forge-io/src/ply/`.
  - Supports ASCII and Binary Little Endian formats with dynamic element and property parsing.
  - Supports position, normal, UV, and RGBA vertex colors.
  - Automatically triangulates quads and arbitrary $N$-gon faces using 3D Ear Clipping.
  - Exporter supports ASCII and Binary Little Endian with custom comments and world transform baking.
- Studio Integration:
  - Added `File -> Import -> STL 3D (.stl)...` and `File -> Import -> Stanford PLY (.ply)...`.
  - Added `File -> Export -> STL 3D (.stl)...` (full scene and selected-only) and `File -> Export -> Stanford PLY (.ply)...` (full scene and selected-only).
  - Integrated with atomic `Action::ImportObjects` Undo/Redo history.
- Validation:
  - 13 new unit tests in `crates/forge-io/tests/` (`stl_tests.rs`, `ply_tests.rs`).
  - 7 new integration tests in `apps/forge3d-studio/tests/stl_ply_studio_integration.rs`.
  - Total **470/470 tests passing** across workspace with zero clippy warnings and zero formatting diffs.
