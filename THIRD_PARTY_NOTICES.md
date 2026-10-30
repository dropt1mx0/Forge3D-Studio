# Third-Party Notices

Forge3D Rust Edition is an independent, clean-room 3D application written in Rust.
It relies on open-source libraries from the Rust and graphics ecosystems.

The following third-party dependencies are used under permissive open-source licenses:

| Package | Version | License | Source | Purpose |
|---|---|---|---|---|
| `wgpu` | 24 | MIT OR Apache-2.0 | https://github.com/gfx-rs/wgpu | Cross-platform GPU abstraction and WGSL pipeline |
| `glam` | 0.30 | MIT OR Apache-2.0 | https://github.com/bitshifter/glam-rs | High-performance 3D linear algebra and math |
| `eframe` / `egui` | 0.37 | MIT OR Apache-2.0 | https://github.com/emilk/egui | Immediate-mode UI framework and window integration |
| `thiserror` | 2.0 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror | Idiomatic typed domain error derivation |
| `anyhow` | 1.0 | MIT OR Apache-2.0 | https://github.com/dtolnay/anyhow | Application boundary error handling |
| `tracing` | 0.1 | MIT | https://github.com/tokio-rs/tracing | Structured diagnostics and logging |
| `tracing-subscriber` | 0.3 | MIT | https://github.com/tokio-rs/tracing | Log formatting and stdout/stderr capture |
| `bytemuck` | 1.x | MIT OR Apache-2.0 | https://github.com/Lokathor/bytemuck | Safe cast helpers for vertex and uniform GPU buffers |
| `serde` | 1.0 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | Serialization and deserialization framework |

All third-party dependencies are verified to be compatible with MIT / Apache-2.0 licensing.
No code, assets, or proprietary algorithms are copied from Blender or any proprietary 3D suite.
