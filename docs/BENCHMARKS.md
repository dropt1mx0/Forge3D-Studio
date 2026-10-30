# Benchmark and Quality Report

## Test Environment Baseline
- **Host**: Lenovo ThinkPad T530
- **OS**: Ubuntu Linux x86_64 (Kernel 7.0.0-38-generic)
- **CPU**: Intel Core i5/i7 3rd Gen (Ivy Bridge)
- **GPU**: Mesa Intel(R) HD Graphics 4000 (IVB GT2) (Backend: Gl)
- **Rust Toolchain**: rustc 1.99.0 stable (b940084d7 2026-09-28), cargo 1.99.0
- **Target Profile**: `dev` and `release`

## Measured Results (Alpha 0.1)

### 1. Test Suite Pass Rates
| Crate | Tests Passed | Tests Failed | Execution Time |
|---|---|---|---|
| `forge-core` | 2 | 0 | < 0.01s |
| `forge-math` | 7 | 0 | < 0.01s |
| `forge-geometry` | 7 | 0 | < 0.01s |
| `forge-scene` | 3 | 0 | 0.04s |
| `forge-gpu` | 1 (Hardware adapter verified) | 0 | 0.19s |
| `forge-render` | 2 | 0 | < 0.01s |
| **Total Workspace** | **22** | **0** | **0.25s** |

### 2. Quality Gates
- `cargo fmt --check`: **PASSED** (100% formatted to Rust standards)
- `cargo check --workspace`: **PASSED** (0 errors)
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASSED** (0 warnings)

### 3. Procedural Mesh Generation Metrics
- **Cube (1x1x1m)**: 24 vertices, 36 indices (12 triangles), generation time < 0.1 ms.
- **UV Sphere (32 sectors, 16 stacks)**: 561 vertices, 960 indices (320 triangles), generation time < 0.3 ms.
- **Ground Plane (10x10 subdivisions)**: 121 vertices, 600 indices (200 triangles), generation time < 0.2 ms.
- **Ground Grid (20x20m, 1m step)**: 86 line vertices (43 line segments), generation time < 0.1 ms.

### 4. GPU Runtime Adapter Verification
- Detected Hardware Adapter: `Mesa Intel(R) HD Graphics 4000 (IVB GT2)`
- Active Backend: `OpenGL (Gl)` via wgpu
- Device Type: `IntegratedGpu`
