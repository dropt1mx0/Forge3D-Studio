//! # Forge3D Render
//!
//! Viewport rendering, camera controls (orbit, pan, zoom, raycast), WGSL shaders,
//! depth testing, lighting passes, and grid rendering.

pub mod camera;
pub mod pipelines;
pub mod renderer;
pub mod shaders;
pub mod uniforms;

pub use camera::{Camera, CameraProjection, ViewPreset};
pub use pipelines::{DEPTH_FORMAT, RenderPipelines};
pub use renderer::{EditRenderOptions, ViewportRenderer};
pub use uniforms::{CameraUniform, EditModeUniform, ObjectUniform};
