---
name: rendering-engine
description: "Use when implementing Forge3D's rendering pipeline, PBR materials, lighting, cameras, shadows, post-processing, render graphs, offline rendering, or viewport visual quality."
---

# Rendering Engine Skill

## Mission
Develop a maintainable renderer with a clear separation between viewport rendering and future offline rendering.

## Pipeline
Start with a simple forward renderer and add complexity only when justified. Document:
- Coordinate and clip-space conventions
- Linear versus sRGB color handling
- Depth convention
- Camera projection
- Material parameter definitions
- Lighting units and assumptions

## Feature progression
1. Mesh and depth rendering
2. Camera and transforms
3. Basic directional/point lighting
4. Metallic-roughness PBR
5. Image-based lighting
6. Shadow mapping
7. Tone mapping and color management
8. Anti-aliasing
9. Post-processing
10. Offline/path-traced rendering as a separate milestone

## Materials
- Use documented physically based parameter conventions.
- Keep material data independent from GPU bind-group objects.
- Define texture color-space semantics.
- Handle missing textures and unsupported features gracefully.

## Quality and correctness
- Compare reference scenes and expected outputs.
- Add deterministic render tests where practical.
- Avoid relying solely on screenshots as correctness tests.
- Document backend and hardware differences.

## Performance
- Track draw calls, pipeline switches, resource uploads, CPU frame time, and GPU frame time.
- Use frustum culling and instancing when scene scale warrants them.
- Avoid premature render graph complexity.
- Do not claim photorealism or production readiness based on a basic viewport.

## Completion checklist
- [ ] Color-space behavior defined
- [ ] Camera/depth conventions documented
- [ ] Materials validated
- [ ] Missing resources handled
- [ ] Performance counters available
- [ ] Viewport and offline rendering concerns separated
