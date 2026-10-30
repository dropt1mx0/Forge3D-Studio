---
name: 3d-engine-architecture
description: "Use when designing Forge3D's application architecture, scene model, editor/runtime separation, ECS or scene graph, asset pipeline, plugin boundaries, serialization, or large-scale 3D engine systems."
---

# 3D Engine Architecture Skill

## Mission
Design a modular 3D creation application that can evolve from a small editor prototype into a professional tool without a monolithic rewrite.

## Core boundaries
- `forge-core`: shared identifiers, foundational types, application-independent utilities.
- `forge-math`: vectors, matrices, transforms, coordinate conventions.
- `forge-geometry` / `forge-mesh`: geometry and topology; no GPU handles.
- `forge-scene`: scene data, hierarchy, components, selection-independent domain model.
- `forge-render` / `forge-gpu`: rendering and hardware resources.
- `forge-asset` / `forge-io`: asset references, import/export, serialization.
- `forge-ui` / application crate: editor presentation and interaction.
- Animation, physics, nodes, and scripting remain separate capabilities.

Adjust this map when evidence supports a better boundary. Do not create empty crates solely to match a diagram.

## Scene architecture
- Define stable object and asset IDs.
- Document transform hierarchy and coordinate conventions.
- Separate persistent scene state from transient editor state.
- Define dirty-state and incremental update mechanisms.
- Avoid storing renderer caches as authoritative scene data.
- Make undo/redo a first-class architectural concern before complex editing tools arrive.

## Data flow
Document how edits propagate:
user command -> scene/domain mutation -> change notification -> derived data invalidation -> GPU upload/update -> rendered frame.

## Asset pipeline
- Distinguish source assets, imported representations, derived caches, and runtime GPU resources.
- Make importers deterministic where practical.
- Version serialized data and provide migration strategy.
- Never silently discard unknown fields or unsupported content.

## Plugin and scripting boundaries
- Define capabilities and permissions before executing third-party extensions.
- Isolate untrusted code; do not load arbitrary native plugins into the main process without a threat model.
- Version plugin APIs.
- Keep scripting APIs separate from internal implementation details.

## Decisions
For significant choices, record context, alternatives, trade-offs, decision, and consequences in `docs/DECISIONS.md`.

## Completion checklist
- [ ] Crate boundaries are clear
- [ ] Persistent and transient state separated
- [ ] Data flow documented
- [ ] IDs and serialization versioning defined
- [ ] Undo/redo implications considered
- [ ] Plugin trust boundaries considered
