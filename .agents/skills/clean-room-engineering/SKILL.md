---
name: clean-room-engineering
description: "Use when researching, designing, implementing, or reviewing Forge3D features inspired by existing 3D applications, file formats, workflows, APIs, or public specifications while preserving independent implementation and license compliance."
---

# Clean-Room Engineering Skill

## Mission
Keep Forge3D an independently implemented product while learning from public standards, documented behavior, and general 3D concepts.

## Core principles
- Do not copy source code from Blender or other third-party applications.
- Do not paraphrase or translate protected source code into a new implementation.
- Do not copy proprietary artwork, icons, UI assets, branding, or documentation text.
- Do not assume that an algorithm is protected or unprotected without checking its legal context.
- Public standards and specifications may be implemented independently, subject to their own terms.
- Compatibility does not imply copying implementation.

## Research process
1. State the user-visible capability being implemented.
2. Identify public standards, specifications, academic literature, or independently observable behavior.
3. Record source URLs, versions, and access dates in an engineering research note.
4. Write an original functional specification.
5. Implement from that specification without using third-party source as a coding template.
6. Review dependencies and imported code for license compatibility.

## File formats and APIs
- Read and follow published format specifications and licenses.
- Distinguish file-format compatibility from copying application internals.
- Do not claim full compatibility based on partial parsing.
- Record unsupported features and round-trip limitations.

## Dependency governance
- Maintain THIRD_PARTY_NOTICES.md.
- Record package name, version, license, source, and usage.
- Review transitive dependencies.
- Escalate ambiguous or incompatible licensing rather than guessing.
- Do not describe MIT, Apache-2.0, GPL, or LGPL compatibility simplistically; evaluate actual distribution and linking obligations.

## Branding
- Forge3D must have its own name, logo, visual identity, icons, and documentation.
- Refer to other products only for factual compatibility or comparison.
- Do not imply endorsement or affiliation.

## Important legal boundary
This skill is an engineering compliance aid, not legal advice. Clean-room process does not automatically eliminate copyright, patent, trademark, or other intellectual-property risk. Escalate material uncertainty for qualified legal review.

## Completion checklist
- [ ] Original functional specification exists
- [ ] No copied implementation/source
- [ ] Sources and standards recorded
- [ ] Dependencies/license reviewed
- [ ] Branding independent
- [ ] Compatibility claims accurately scoped
