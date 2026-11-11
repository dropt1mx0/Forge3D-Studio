# Forge3D Studio

**Forge3D Studio** is an independent, open-source 3D creation application built with Rust and modern graphics standards.

*Built with Rust. Open by Design.*

---

## Vision & Architecture

Forge3D is engineered from first principles with a modular, layered architecture:
- **`forge-core`**: Core identities, entity IDs, and error definitions.
- **`forge-math`**: 3D linear algebra (glam-based), coordinate conventions (Right-Handed, Y-up), and geometric intersection routines (Ray, AABB, Triangle).
- **`forge-geometry`**: Pure-CPU mesh representation, procedural generators (Cube, UV Sphere, Plane, Grid), normal calculation, triangulation, and validation invariants.
- **`forge-scene`**: Scene hierarchy, transform propagation, selection state, editor interaction state, and `.forge` project serialization.
- **`forge-gpu`**: `wgpu` device, queue, surface abstraction, and runtime hardware capability discovery.
- **`forge-render`**: Viewport rendering pipeline, WGSL shaders, perspective camera with orbit/pan/zoom, depth testing, lighting, and selection highlights.
- **`forge3d-studio`**: Native desktop application with custom dark graphite UI layout (menu bar, toolbar, central viewport, scene outliner, transform properties, status bar).

---

## Building and Running

### Prerequisites
- Stable Rust (1.87+ recommended, tested with Rust 1.99)
- Standard graphics drivers (Vulkan, Metal, DirectX 12, or OpenGL/GLES)
- Linux: `libwayland`, `libX11`, `libvulkan` (dynamically loaded)

### Build
```bash
cargo build --workspace
```

### Run
```bash
cargo run -p forge3d-studio
```

### Run Tests
```bash
cargo test --workspace
```

---

## Milestones & Status
- **Alpha 0.1**: Initial native application, GPU viewport, camera navigation, primitives, selection, transforms, and scene hierarchy.
- **Alpha 0.2**: Interaction & Workspace UX, automatic gizmo, modal transforms, resizable workspace, expanded primitives.

## Donate

You can support Forge3D directly using one of these wallets. Please use the matching asset and network.

| Asset | Network | Wallet address |
| --- | --- | --- |
| ETH | Ethereum | `0x285E817deBfddd18286B1B2ef6bc14fBA0c9E382` |
| BTC | Bitcoin | `bc1q0muwvll862k4w9eavuu9837wulw5s4xxfmq6fv` |
| SOL | Solana | `3BuKpWvAosSo61dFJgx1MVmFbNMwTDZStUEJkPseLsPM` |
| BNB | BNB Smart Chain | `0x285E817deBfddd18286B1B2ef6bc14fBA0c9E382` |
| TRX | TRON | `TSKSLWq4kWVw4y79ripbYLA4yPrQip2Ggz` |

---

## Security
If you discover a vulnerability or security-sensitive defect, please review [SECURITY.md](SECURITY.md) and contact maintainers privately at `dropt1m.sup@gmail.com`.

---

## License
Dual-licensed under MIT or Apache-2.0. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
