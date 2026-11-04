/// WGSL shader for rendered meshes with directional + ambient lighting,
/// specular highlights, and object selection coloring.
pub const MESH_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    is_selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) vertex_color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = object.model * vec4<f32>(model.position, 1.0);
    out.world_pos = world_pos4.xyz;
    out.clip_position = camera.view_proj * world_pos4;
    
    // Normal transform (assuming uniform scale; for non-uniform, model inverse transpose)
    let normal4 = object.model * vec4<f32>(model.normal, 0.0);
    out.world_normal = normalize(normal4.xyz);
    out.uv = model.uv;
    out.vertex_color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.6));
    let normal = normalize(in.world_normal);
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);

    // Blinn-Phong lighting components
    let ambient = vec3<f32>(0.22, 0.24, 0.28);
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = vec3<f32>(0.78, 0.76, 0.72) * diff;
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = vec3<f32>(0.35) * spec;

    let base_rgb = object.color.rgb * in.vertex_color.rgb;
    var lit = (ambient + diffuse) * base_rgb + specular;

    // Selection highlight feedback
    if (object.is_selected == 1u) {
        let selection_tint = vec3<f32>(1.0, 0.62, 0.18); // Warm orange highlight
        lit = mix(lit, selection_tint, 0.35);
    }

    return vec4<f32>(lit, object.color.a);
}
"#;

/// WGSL shader for grid and coordinate axes lines.
pub const GRID_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// WGSL shader for Edit Mode wireframe edges with model transform.
pub const EDIT_EDGE_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    is_selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = object.model * vec4<f32>(model.position, 1.0);
    var clip = camera.view_proj * world_pos4;
    // Nudge depth slightly towards camera to prevent z-fighting with mesh faces
    clip.z -= 0.0001 * clip.w;
    out.clip_position = clip;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Crisp dark slate wireframe edge color (RGB: [0.30, 0.33, 0.40], Alpha: 0.95)
    return vec4<f32>(0.30, 0.33, 0.40, 0.95);
}
"#;

/// WGSL shader for Edit Mode vertex handles rendering using screen-space billboard quads.
pub const EDIT_VERTEX_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    is_selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct EditModeUniform {
    viewport_size: vec2<f32>,
    vertex_point_size: f32,
    is_vertex_mode: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

@group(2) @binding(0)
var<uniform> edit_cfg: EditModeUniform;

// Slot 0: Unit Quad [-1..1]
struct QuadVertexInput {
    @location(0) quad_pos: vec2<f32>,
};

// Slot 1: Instance Position (reused from EditVertexBuffer)
struct InstancePosInput {
    @location(1) center_pos: vec3<f32>,
};

// Slot 2: Instance Flags (selection / active state)
struct InstanceFlagsInput {
    @location(2) flags: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) flags: u32,
};

@vertex
fn vs_main(
    quad: QuadVertexInput,
    instance_pos: InstancePosInput,
    instance_flags: InstanceFlagsInput,
) -> VertexOutput {
    var out: VertexOutput;

    // 1. Transform vertex center to clip space
    let world_pos = object.model * vec4<f32>(instance_pos.center_pos, 1.0);
    let clip_center = camera.view_proj * world_pos;

    // 2. Compute screen-space pixel offset in NDC
    let pixel_scale = 2.0 / max(edit_cfg.viewport_size, vec2<f32>(1.0, 1.0));
    var radius_px = edit_cfg.vertex_point_size * 0.5;

    let is_selected = (instance_flags.flags & 1u) != 0u;
    let is_active = (instance_flags.flags & 2u) != 0u;

    if (is_active) {
        radius_px = radius_px * 1.5;
    } else if (is_selected) {
        radius_px = radius_px * 1.35;
    }

    let offset_ndc = quad.quad_pos * radius_px * pixel_scale;

    var final_clip = clip_center;
    final_clip.x += offset_ndc.x * clip_center.w;
    final_clip.y += offset_ndc.y * clip_center.w;

    // Nudge depth towards camera so vertex handles sit in front of both mesh faces and wireframe edges
    final_clip.z -= 0.00025 * clip_center.w;

    out.clip_position = final_clip;
    out.uv = quad.quad_pos;
    out.flags = instance_flags.flags;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    if (dist > 1.0) {
        discard;
    }

    let is_selected = (in.flags & 1u) != 0u;
    let is_active = (in.flags & 2u) != 0u;

    // In non-vertex selection modes (Edge, Face), hide unselected vertices
    if (edit_cfg.is_vertex_mode == 0u && !is_selected && !is_active) {
        discard;
    }

    // Crisp circular anti-aliasing on outer perimeter
    let edge_alpha = clamp((1.0 - dist) * 3.5, 0.0, 1.0);

    if (is_active) {
        // Active vertex: bright lemon-yellow/white center, golden ring, dark outer rim
        if (dist > 0.78) {
            return vec4<f32>(0.08, 0.09, 0.11, edge_alpha);
        } else if (dist > 0.42) {
            return vec4<f32>(1.0, 0.72, 0.16, 1.0);
        } else {
            return vec4<f32>(1.0, 1.0, 0.90, 1.0);
        }
    } else if (is_selected) {
        // Selected vertex: golden orange with bright yellow center and dark outer rim
        if (dist > 0.78) {
            return vec4<f32>(0.08, 0.09, 0.11, edge_alpha);
        } else if (dist > 0.45) {
            return vec4<f32>(1.0, 0.72, 0.16, 1.0);
        } else {
            return vec4<f32>(1.0, 0.90, 0.40, 1.0);
        }
    } else {
        // Unselected vertex: crisp neutral slate with dark outer rim
        if (dist > 0.75) {
            return vec4<f32>(0.08, 0.09, 0.11, edge_alpha);
        } else {
            return vec4<f32>(0.22, 0.25, 0.30, 0.95);
        }
    }
}
"#;
