# FORGE3D RUST EDITION
## Master Project Specification & Gemini Kickoff Instructions

**Project name:** Forge3D Rust Edition  
**Project type:** Independent, open-source 3D creation application  
**Primary language:** Rust  
**Initial platform:** Linux  
**Target platforms:** Linux, Windows, macOS  
**Graphics foundation:** wgpu + WGSL, with backend capabilities checked at runtime  
**Initial release target:** Alpha 0.1  
**Document status:** Initial master specification; update as implementation decisions are made

---

# 1. Product vision

Forge3D Rust Edition is an independent, open-source 3D creation application built primarily with Rust.

The long-term ambition is to develop a capable 3D suite covering:

- 3D viewport and scene editing
- Polygon mesh modeling
- Materials and lighting
- Real-time rendering
- Import and export of common 3D formats
- Procedural geometry and node-based workflows
- Animation and rigging
- Sculpting
- Physics
- Extensibility through plugins and scripting

Forge3D is not intended to be a superficial clone or a one-shot rewrite of another application. It must have its own architecture, implementation, product identity, documentation, and development community.

The first objective is much smaller and concrete: build a reliable native application with a working GPU viewport, camera controls, basic objects, object selection, transforms, and project save/load foundations.

## Product principles

1. **Correctness before speed.** A fast incorrect geometry engine is not useful.
2. **Measure performance.** Never claim Forge3D is faster without reproducible benchmarks.
3. **Rust-first, not Rust-at-any-cost.** Use Rust for the core and use mature native libraries when justified.
4. **Modular architecture.** Geometry, scene data, rendering, UI, import/export, and application orchestration must have clear boundaries.
5. **Cross-platform by design.** Avoid unnecessary Linux-only assumptions in core architecture.
6. **Local-first.** Core editing workflows should not require an online account or cloud service.
7. **Interoperability.** Support common open 3D formats where practical.
8. **Original identity.** Forge3D must have its own branding, UI assets, and product language.
9. **Incremental delivery.** Ship small working milestones rather than generating a huge unverified codebase.
10. **Open development.** Document decisions, limitations, dependencies, and benchmarks.

---

# 2. Target users and initial use case

Initial target users:

- Hobbyist 3D artists
- Indie game developers
- Technical artists
- Developers interested in procedural 3D
- Linux users who want a native 3D tool
- Contributors interested in Rust graphics and geometry

Initial use case:

A user launches Forge3D, creates a scene, adds a cube, sphere, and plane, navigates a 3D viewport, selects and transforms objects, imports an OBJ or glTF asset, and saves the scene in Forge3D's own project format.

Do not claim professional production readiness in the first alpha.

---

# 3. Product scope and release strategy

## Alpha 0.1 — first public technical milestone

Alpha 0.1 must provide a real, compiling, testable desktop application with:

- Native application window
- GPU initialization
- Working 3D viewport
- Perspective camera
- Orbit, pan, and zoom
- Grid and coordinate axes
- Basic lighting
- Cube, sphere, and plane primitives
- Object selection
- Object translation, rotation, and scaling
- Basic scene hierarchy
- Basic project save/load, if it can be implemented without destabilizing the milestone
- Clear startup and runtime error reporting

The UI may be simple, but controls that appear functional must actually work. Do not add decorative fake buttons.

## Alpha 0.2 — basic modeling

- Mesh data model
- Vertex, edge, and face selection
- Edit mode
- Basic transforms
- Extrude
- Inset
- Normals
- Triangulation
- OBJ import/export
- glTF/GLB import

## Alpha 0.3 — scene and materials

- Scene outliner
- Object properties
- Material data model
- Basic PBR viewport
- Lights and cameras
- Scene serialization versioning
- Undo/redo foundation

## Alpha 0.4 — rendering and asset pipeline

- Shadow mapping
- Environment lighting
- Texture loading
- Asset browser foundation
- More robust glTF/GLB support
- Better resource management
- Performance instrumentation

## Later milestones

- Modifiers
- Geometry nodes / procedural graph
- Animation timeline
- Rigging
- Physics
- Sculpting
- Offline rendering
- Scripting and plugin system
- USD and additional interchange formats

These are long-term goals, not promises for Alpha 0.1.

---

# 4. Technical architecture

Start with a Cargo workspace, but do not create empty crates merely to match a diagram. Add crates when they have a real responsibility and code.

Suggested initial structure:

```text
Forge3D-Rust-Edition/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── LICENSE
├── THIRD_PARTY_NOTICES.md
├── CONTRIBUTING.md
├── SECURITY.md
├── AGENTS.md
├── GEMINI.md
├── .agents/
│   └── skills/
├── apps/
│   └── forge3d-studio/
├── crates/
│   ├── forge-core/
│   ├── forge-math/
│   ├── forge-geometry/
│   ├── forge-scene/
│   ├── forge-gpu/
│   ├── forge-render/
│   ├── forge-asset/
│   ├── forge-io/
│   └── forge-bench/
├── assets/
├── docs/
│   ├── architecture/
│   ├── decisions/
│   ├── development/
│   ├── formats/
│   └── performance/
├── examples/
├── tests/
├── benchmarks/
└── scripts/
```

## Responsibility boundaries

### `forge-core`
- Shared IDs
- Application-independent foundational types
- Common error types where appropriate
- No UI or GPU-specific dependencies

### `forge-math`
- Coordinate conventions
- Vectors, matrices, quaternions, transforms
- Numerical helpers
- Tests for mathematical invariants

### `forge-geometry`
- Mesh topology
- Geometry algorithms
- Mesh validation
- Normals and triangulation
- No wgpu handles

### `forge-scene`
- Scene graph or chosen scene representation
- Objects and components
- Transform hierarchy
- Persistent scene data
- Separation of persistent scene state from editor-only state

### `forge-gpu`
- wgpu device, queue, surface, buffers, textures
- Adapter capability discovery
- GPU resource lifecycle
- Backend-specific details

### `forge-render`
- Render pipeline
- Camera and render extraction
- Materials and lighting
- Viewport rendering
- No UI framework dependency

### `forge-asset`
- Asset IDs and references
- Asset metadata
- Cache strategy
- Import pipeline coordination

### `forge-io`
- OBJ, glTF, GLB, and later formats
- Forge3D project serialization
- Versioning and migrations

### `forge-bench`
- Reproducible benchmark workloads
- Benchmark metadata
- Reports and comparison helpers

### `apps/forge3d-studio`
- Application entry point
- Window and event loop
- Editor UI
- User interaction
- Orchestration between crates

## Dependency direction

Prefer dependencies flowing from application/UI toward domain and infrastructure layers. Avoid circular crate dependencies.

Keep scene and geometry crates independent from GPU resource types. The renderer should derive GPU representations from scene data rather than making GPU handles the authoritative scene model.

Document the final dependency graph in `docs/architecture/`.

---

# 5. Technology selection

Use current stable Rust unless a documented technical reason requires otherwise.

Potential technologies:

| Area | Candidate | Notes |
|---|---|---|
| Language | Rust stable | Primary implementation language |
| GPU abstraction | wgpu | Portable GPU API; check actual backend capabilities |
| Shader language | WGSL | Keep shader interfaces validated |
| Math | glam | Verify conventions and alignment |
| UI | egui or another native-capable framework | Prototype and evaluate before locking in |
| Windowing | winit or UI framework integration | Avoid duplicate event loops |
| Physics | Rapier3D | Later milestone; review license and integration |
| Serialization | serde | Versioned project schema required |
| Errors | thiserror / anyhow | Use at appropriate boundaries |
| Logging | tracing | Structured diagnostics |
| CPU parallelism | rayon | Use for suitable CPU workloads |
| Async I/O | tokio where appropriate | Do not use async without a need |
| File watching | notify | Asset pipeline use |
| Testing | built-in Rust tests; nextest optional | CI should remain reproducible |

Do not add every candidate immediately. Evaluate the smallest stack that can deliver Alpha 0.1.

For every dependency:
- Check current maintenance status.
- Check supported platforms.
- Check license and transitive dependencies.
- Check compile-time and binary-size impact.
- Record why it is needed.
- Prefer mature dependencies over custom reinvention unless a measured product requirement justifies custom code.

---

# 6. Graphics and viewport requirements

## First viewport

Implement in this order:

1. Create application window.
2. Initialize a wgpu instance and select a compatible adapter.
3. Create device and queue.
4. Configure the surface.
5. Clear the frame to a known background color.
6. Render a triangle.
7. Render a cube with depth testing.
8. Add perspective camera.
9. Add orbit, pan, and zoom.
10. Add grid and axes.
11. Add basic lighting.
12. Add mesh upload/update path.
13. Render multiple objects.
14. Add object selection feedback.

## Renderer architecture

- Separate CPU scene representation from GPU buffers.
- Define explicit GPU resource lifetimes.
- Handle window resize and surface reconfiguration.
- Query adapter features and limits.
- Implement fallbacks for optional capabilities.
- Avoid per-frame allocations in hot paths.
- Avoid unnecessary CPU/GPU readbacks.
- Keep shader source version-controlled.
- Validate Rust/WGSL buffer layouts and bind-group contracts.
- Surface shader compilation and pipeline errors clearly.

Do not claim that Vulkan, Metal, Direct3D 12, ray tracing, or a particular GPU feature is active unless the runtime adapter and backend confirm it.

---

# 7. Geometry engine requirements

Geometry is a core differentiator, but it must be developed incrementally.

First implement:
- Vertex and index buffers for render meshes
- A clear editable mesh representation
- Mesh validation
- Transform application
- Normal generation
- Triangulation
- Bounding boxes

Later implement:
- Vertex/edge/face topology
- Stable handles or generational IDs
- Attribute domains
- Extrude
- Inset
- Bevel
- Subdivision
- Boolean operations
- Mesh simplification
- Spatial acceleration structures

For every geometry operation document:
- Preconditions
- Output invariants
- Degenerate cases
- Numerical tolerances
- Attribute propagation
- Expected complexity
- Error behavior

Test open boundaries, non-manifold edges, duplicate vertices, degenerate faces, NaN/infinite coordinates, and very small/large coordinate scales.

Do not assume every polygon is a triangle.

---

# 8. Scene model and editing

Define and document:
- Object IDs
- Asset IDs
- Transform conventions
- Coordinate system and handedness
- Parent-child transforms
- Scene hierarchy
- Selection state
- Visibility
- Persistent scene state
- Transient editor state
- Dirty-state tracking
- Undo/redo design

The scene model must not depend on a particular renderer or UI framework.

Design change propagation explicitly:

```text
User command
    ↓
Scene/domain mutation
    ↓
Change notification
    ↓
Derived data invalidation
    ↓
GPU upload/update
    ↓
Rendered frame
```

Avoid full-scene rebuilds for every small edit when incremental updates are practical.

---

# 9. UI and product identity

Forge3D must have its own visual identity.

Initial visual direction:
- Dark graphite interface
- Clear, restrained typography
- Large central viewport
- Left-side tool area
- Right-side outliner and properties
- Bottom timeline area reserved for later
- Compact top-level menus
- High-DPI and UI scaling considerations

Do not copy Blender's icons, artwork, logo, distinctive branding, or protected UI assets. Familiar 3D workflows and common interface concepts may be independently designed, but Forge3D should look and feel like its own product.

First UI priorities:
- File menu
- Viewport
- Object creation
- Selection
- Transform controls
- Outliner
- Properties
- Status and error reporting

Do not build a huge polished interface before the viewport and scene model work.

---

# 10. File formats and project storage

Create a native Forge3D project format, initially `.forge`.

Requirements:
- Versioned schema
- Scene hierarchy
- Object transforms
- Mesh data or asset references
- Material data
- Camera and lighting data
- Project metadata
- External asset references
- Safe load failure behavior
- Atomic or recoverable save strategy where practical

Initial interchange priority:
1. OBJ import/export
2. glTF import
3. GLB import/export
4. STL import/export

Later:
- USD
- Additional formats after technical and license review

Do not promise native `.blend` compatibility in the initial roadmap. Treat any future `.blend` support as a separate format compatibility project, with its own research and legal review.

---

# 11. Performance strategy and benchmarks

Rust does not automatically make an application faster than C++, Java, or an existing optimized program. Performance must be demonstrated.

Create a reproducible benchmark plan.

Measure:
- Cold startup time
- Warm startup time
- Idle RAM
- Peak RSS
- CPU usage
- GPU memory where measurable
- Viewport frame time
- FPS and frame-time percentiles
- Scene load/save time
- Mesh operation throughput
- Allocation behavior where tooling permits

Initial workloads:
- Cube and simple scene
- 100,000 vertices
- 1 million vertices
- 10 million vertices where hardware permits
- 100,000 scene objects where feasible
- Large GLB import
- Repeated mesh transforms
- Material-heavy scene

Record:
- CPU
- GPU
- RAM
- OS
- GPU driver
- Rust version
- Build profile
- Feature flags
- Scene fixture
- Settings
- Number of runs
- Variance

Compare with other software only using equivalent workloads, quality settings, and hardware. Never invent results or cherry-pick measurements.

---

# 12. Testing and quality gates

Use:
- Unit tests
- Integration tests
- Property-based tests where useful
- Serialization round-trip tests
- Import/export fixtures
- Geometry invariants
- GPU smoke tests where available
- Performance benchmarks
- Cross-platform CI as the project matures

Typical checks:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

Run targeted commands during iteration and full workspace checks at milestones.

Do not claim a command passed unless it was actually executed and its result inspected.

GPU tests may be unavailable in headless CI. Mark them skipped with a clear reason; do not present a skip as a successful GPU validation.

---

# 13. Clean-room and licensing policy

Forge3D must be independently implemented.

Rules:
- Do not copy Blender or other applications' source code.
- Do not ask an AI model to rewrite copied source code to disguise its origin.
- Do not copy protected artwork, icons, branding, or documentation text.
- Use public standards, documented APIs, academic literature, and independently written functional specifications.
- Record research sources and dates for important compatibility work.
- Maintain `THIRD_PARTY_NOTICES.md`.
- Review direct and transitive dependency licenses.
- Check patent, trademark, copyright, and distribution considerations where relevant.
- Do not assume that calling a process “clean room” automatically removes legal risk.

Clean-room engineering is an engineering process, not a guarantee of legal clearance. Escalate material uncertainty for qualified legal review.

---

# 14. Security and autonomy boundaries

The AI agent is authorized to work autonomously within the project workspace.

Allowed:
- Inspect project files
- Create and edit project files
- Add normal project dependencies after review
- Run Cargo commands
- Run tests and benchmarks
- Format and lint code
- Create documentation
- Review Git diffs
- Create local commits if Git is configured and the change is verified

Not allowed without explicit user authorization:
- `sudo` or elevated system operations
- Destructive operations outside the project directory
- Deleting unrelated files
- Reading unrelated personal files
- Uploading secrets or private data
- Pushing to remote repositories
- Publishing packages or releases
- Deploying services
- Changing global system configuration
- Disabling security controls
- Running untrusted scripts with elevated privileges

If blocked by an approval requirement, do not attempt to bypass it. Record the blocker, continue with independent safe tasks, and list the exact user action needed.

---

# 15. Autonomous development protocol

The user wants to be able to leave the agent working for a period without answering routine questions.

Therefore:

- Do not ask routine preference questions.
- Make reasonable technical decisions independently.
- Record assumptions and alternatives.
- Do not pause for approval for ordinary file creation or code edits inside the workspace.
- Do not stop after writing a plan; proceed to implementation when the environment permits.
- If a dependency is unavailable, investigate a suitable alternative.
- If a test fails, investigate and fix the cause.
- If a task is blocked, document it and move to independent tasks.
- Never fabricate successful commands, tests, benchmarks, or completed features.
- Never hide a failing test by weakening or deleting it.
- Do not generate enormous changes that cannot be reviewed or compiled.
- Work in small, verifiable increments.
- Preserve existing user work.

Maintain these project files:
- `TASKS.md`
- `CHANGELOG.md`
- `docs/ARCHITECTURE.md`
- `docs/DECISIONS.md`
- `docs/ROADMAP.md`
- `docs/BENCHMARKS.md`
- `docs/DEVELOPMENT.md`
- `docs/SESSION_STATE.md`

At the end of each meaningful work session:
1. Update completed and pending tasks.
2. Record commands actually run.
3. Record test results.
4. Record unresolved errors and blockers.
5. Record exact next steps in `docs/SESSION_STATE.md`.

If context or execution time is running low, leave a clear continuation report rather than pretending the project is complete.

---

# 16. Agent Skills integration

The repository contains a local `.agents/skills/` directory.

The current Forge3D Engineering Skill Pack includes:
- `rust-engineering`
- `graphics-programming`
- `3d-engine-architecture`
- `geometry-engine`
- `rendering-engine`
- `performance-engineering`
- `gpu-rendering`
- `testing-and-qa`
- `clean-room-engineering`
- `code-review`

Before implementation:
1. Inspect `.agents/skills/`.
2. Validate each `SKILL.md` frontmatter and description.
3. Confirm which skills the IDE actually discovers.
4. Use relevant skills for each task.
5. Do not duplicate or overwrite existing skills.
6. If a skill is not discoverable, document the issue and continue using its instructions manually where possible.

Do not claim that a skill is active merely because its folder exists.

---

# 17. First implementation mission: Alpha 0.1

Work through these tasks in order.

## Task A — Workspace reconnaissance
- Inspect the current repository and all existing files.
- Inspect Git status.
- Inspect Rust/Cargo versions.
- Inspect installed target/toolchain components.
- Check available graphics environment and GPU adapter if possible.
- Identify existing user work and preserve it.

## Task B — Architecture and setup
- Create or refine Cargo workspace.
- Add only crates needed for the first milestone.
- Create initial architecture documentation.
- Create dependency/license inventory.
- Create task and session tracking files.
- Confirm local Agent Skills.

## Task C — Native application
- Create a working application entry point.
- Open a native window.
- Initialize wgpu.
- Select an adapter and report its name and capabilities.
- Handle surface configuration and resize.
- Render a clear color.
- Render a triangle.
- Render a cube with depth testing.

## Task D — Viewport
- Perspective camera
- Orbit
- Pan
- Zoom
- Grid
- Coordinate axes
- Basic lighting

## Task E — Objects and scene
- Cube, sphere, plane
- Scene object IDs
- Object selection
- Translation
- Rotation
- Scaling
- Basic outliner or object list

## Task F — Verification
- Run format check
- Run Cargo check
- Run tests
- Run Clippy
- Launch the application if environment permits
- Verify actual interaction
- Record screenshots only if the environment supports reliable capture
- Record known limitations

## Alpha 0.1 acceptance criteria

Alpha 0.1 is not complete until:
- The application compiles.
- The application launches on the development machine.
- A real GPU viewport is displayed, or a clearly documented fallback is used.
- Camera navigation works.
- At least cube, sphere, and plane can be displayed.
- Selection and transforms work.
- Errors are reported rather than silently ignored.
- Tests and build checks have actual recorded results.
- Documentation describes what works and what does not.

If any criterion cannot be completed, report it as incomplete with a specific reason. Do not relabel it as complete.

---

# 18. Gemini kickoff prompt

(See Kickoff Prompt in Master Spec)

---

# 19. First public release checklist

Before publishing Alpha 0.1, verify:

- [ ] Project builds from a clean checkout
- [ ] README has build and run instructions
- [ ] License is present and dependency compatibility reviewed
- [ ] Third-party notices are present
- [ ] No secrets or private files are committed
- [ ] No copied third-party assets or code
- [ ] Application launches
- [ ] Viewport renders
- [ ] Camera navigation works
- [ ] Object selection works
- [ ] Transform tools work
- [ ] Tests and lint checks are recorded
- [ ] Known limitations are documented
- [ ] Benchmark methodology is documented
- [ ] Screenshots/demo are genuine captures of the application

---

# 20. Definition of success

The first success is not “a Blender replacement”.

The first success is a clean, independently implemented Rust application that opens, renders a real 3D scene, lets a user navigate and manipulate objects, and has an architecture that can grow without being rewritten.

Forge3D should earn larger ambitions through working milestones, reliable engineering, and measurable results.

**FORGE3D RUST EDITION — Built with Rust. Open by Design.**
