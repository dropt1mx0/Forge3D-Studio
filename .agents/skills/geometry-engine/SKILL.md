---
name: geometry-engine
description: "Use when implementing mesh data structures, topology, polygon operations, transforms, normals, triangulation, modifiers, spatial structures, mesh validation, or geometry algorithms in Forge3D."
---

# Geometry Engine Skill

## Mission
Implement robust geometry operations with correct topology, predictable numerical behavior, and measurable performance.

## Before implementation
- Define coordinate system, handedness, winding order, units, and tolerance policy.
- Identify whether data is indexed, non-indexed, half-edge, winged-edge, or another representation.
- State invariants for vertices, edges, faces, loops, and adjacency.
- Determine whether operations must preserve IDs, attributes, UVs, normals, and material assignments.

## Data model
- Separate editable topology from render-optimized vertex/index buffers.
- Use stable handles or generational IDs where deletion/reuse can invalidate references.
- Define attribute domains: point, edge, face, corner/loop, and instance as applicable.
- Avoid assuming every polygon is a triangle.
- Make non-manifold geometry behavior explicit.

## Algorithms
For each operation, document:
- Preconditions
- Expected output invariants
- Degenerate cases
- Numerical tolerance
- Complexity
- Attribute propagation
- Failure behavior

Prioritize basic operations in a testable order:
1. Mesh creation and validation
2. Transformations
3. Normal generation
4. Triangulation
5. Vertex/edge/face selection
6. Extrude and inset
7. Bevel
8. Subdivision
9. Boolean operations

## Numerical robustness
- Handle zero-length edges, degenerate faces, duplicate vertices, near-coplanar geometry, and non-finite values.
- Use scale-aware tolerances where appropriate.
- Do not silently accept NaN or infinity into persistent scene data.
- Test very small and very large coordinate magnitudes.

## Performance
- Benchmark representative meshes, not only tiny examples.
- Avoid repeated full-mesh scans when incremental updates are possible.
- Use parallelism only when partitioning and merge costs justify it.
- Keep cache invalidation explicit.

## Testing
Create fixtures for:
- Cube, plane, UV sphere
- Open boundaries
- Non-manifold edges
- Degenerate faces
- Duplicate vertices
- Large meshes
- Randomized/property-based cases where useful

## Completion checklist
- [ ] Invariants documented
- [ ] Degenerate cases tested
- [ ] Attributes preserved or explicitly dropped
- [ ] Numerical limits documented
- [ ] Benchmarks added for hot operations
