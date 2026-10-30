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
