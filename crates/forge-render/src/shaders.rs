/// WGSL forward metallic-roughness renderer with scene-driven lights and maps.
pub const MESH_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    normal_model: mat4x4<f32>,
    color: vec4<f32>,
    is_selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct MaterialUniform {
    base_color_factor: vec4<f32>,
    emissive_factor: vec4<f32>,
    material_params: vec4<f32>, // metallic, roughness, normal scale, AO strength
    uv_scale: vec2<f32>,
    _padding: vec2<u32>,
};

struct LightRecord {
    position_type: vec4<f32>,       // xyz position; w = 0 directional, 1 point, 2 spot
    direction_intensity: vec4<f32>, // xyz forward direction; w intensity
    color_range: vec4<f32>,         // rgb linear color; w range
    spot_cones: vec4<f32>,          // x inner cone cosine; y outer cone cosine
};

struct LightingUniform {
    sky_color_intensity: vec4<f32>,
    ground_color: vec4<f32>,
    light_count: u32,
    shadow_light_index: u32,
    _padding: vec2<u32>,
    shadow_view_proj: mat4x4<f32>,
    lights: array<LightRecord, 16>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

@group(2) @binding(0)
var albedo_texture: texture_2d<f32>;
@group(2) @binding(1)
var material_sampler: sampler;
@group(2) @binding(2)
var normal_texture: texture_2d<f32>;
@group(2) @binding(3)
var metallic_texture: texture_2d<f32>;
@group(2) @binding(4)
var roughness_texture: texture_2d<f32>;
@group(2) @binding(5)
var ao_texture: texture_2d<f32>;
@group(2) @binding(6)
var emissive_texture: texture_2d<f32>;
@group(2) @binding(7)
var<uniform> material: MaterialUniform;

@group(3) @binding(0)
var<uniform> lighting: LightingUniform;
@group(3) @binding(1)
var shadow_map: texture_depth_2d;
@group(3) @binding(2)
var shadow_sampler: sampler_comparison;

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
    
    let normal4 = object.normal_model * vec4<f32>(model.normal, 0.0);
    out.world_normal = normalize(normal4.xyz);
    out.uv = model.uv;
    out.vertex_color = model.color;
    return out;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(3.14159265 * denominator * denominator, 0.0001);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / max(n_dot_v * (1.0 - k) + k, 0.0001);
}

fn evaluate_brdf(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32,
) -> vec3<f32> {
    let half_dir = normalize(view_dir + light_dir);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let n_dot_v = max(dot(normal, view_dir), 0.001);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);
    let f0 = mix(vec3<f32>(0.04), base_color, metallic);
    let fresnel = fresnel_schlick(h_dot_v, f0);
    let distribution = distribution_ggx(n_dot_h, roughness);
    let geometry = geometry_schlick_ggx(n_dot_v, roughness)
        * geometry_schlick_ggx(n_dot_l, roughness);
    let specular = (distribution * geometry * fresnel)
        / max(4.0 * n_dot_v * max(n_dot_l, 0.001), 0.001);
    let diffuse = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic)
        * base_color / 3.14159265;
    return (diffuse + specular) * n_dot_l;
}

fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn directional_shadow(world_pos: vec3<f32>) -> f32 {
    let clip = lighting.shadow_view_proj * vec4<f32>(world_pos, 1.0);
    let ndc = clip.xyz / max(clip.w, 0.0001);
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0
        || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return textureSampleCompare(shadow_map, shadow_sampler, uv, ndc.z - 0.0015);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let geometric_normal = normalize(in.world_normal);
    var normal = geometric_normal;
    let uv = in.uv * material.uv_scale;
    {
        let uv_dx = dpdx(uv);
        let uv_dy = dpdy(uv);
        let pos_dx = dpdx(in.world_pos);
        let pos_dy = dpdy(in.world_pos);
        let uv_determinant = uv_dx.x * uv_dy.y - uv_dx.y * uv_dy.x;
        let determinant_sign = select(-1.0, 1.0, uv_determinant >= 0.0);
        let determinant_scale = determinant_sign / max(abs(uv_determinant), 0.0001);
        let tangent = normalize((pos_dx * uv_dy.y - pos_dy * uv_dx.y) * determinant_scale);
        let bitangent = normalize((-pos_dx * uv_dy.x + pos_dy * uv_dx.x) * determinant_scale);
        let normal_xy = (textureSample(normal_texture, material_sampler, uv).rg
            * 2.0 - vec2<f32>(1.0)) * material.material_params.z;
        let normal_z = sqrt(max(1.0 - dot(normal_xy, normal_xy), 0.0));
        let tangent_normal = normalize(vec3<f32>(normal_xy, normal_z));
        let tangent_basis = mat3x3<f32>(tangent, bitangent, geometric_normal);
        normal = normalize(tangent_basis * tangent_normal);
    }

    let base_sample = textureSample(albedo_texture, material_sampler, uv);
    let base_color = object.color.rgb * in.vertex_color.rgb
        * material.base_color_factor.rgb * base_sample.rgb;
    let metallic = clamp(material.material_params.x
        * textureSample(metallic_texture, material_sampler, uv).r, 0.0, 1.0);
    let roughness = clamp(material.material_params.y
        * textureSample(roughness_texture, material_sampler, uv).r, 0.045, 1.0);
    let ao_sample = textureSample(ao_texture, material_sampler, uv).r;
    let ambient_occlusion = mix(1.0, ao_sample, clamp(material.material_params.w, 0.0, 1.0));
    let emissive = material.emissive_factor.rgb
        * textureSample(emissive_texture, material_sampler, uv).rgb;
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let hemisphere = mix(
        lighting.ground_color.rgb,
        lighting.sky_color_intensity.rgb,
        clamp(normal.y * 0.5 + 0.5, 0.0, 1.0),
    ) * lighting.sky_color_intensity.w;
    let ambient_f0 = mix(vec3<f32>(0.04), base_color, metallic);
    let ambient = (base_color * (1.0 - metallic) + ambient_f0 * (1.0 - roughness * 0.5))
        * hemisphere * ambient_occlusion;
    var direct = vec3<f32>(0.0);
    for (var index = 0u; index < min(lighting.light_count, 16u); index += 1u) {
        let light = lighting.lights[index];
        var light_dir = normalize(-light.direction_intensity.xyz);
        var attenuation = light.direction_intensity.w;
        if light.position_type.w > 0.5 {
            let to_light = light.position_type.xyz - in.world_pos;
            let distance = max(length(to_light), 0.001);
            light_dir = to_light / distance;
            let range_ratio = distance / max(light.color_range.w, 0.001);
            let cutoff = clamp(1.0 - pow(range_ratio, 4.0), 0.0, 1.0);
            attenuation = light.direction_intensity.w * cutoff * cutoff
                / max(distance * distance, 0.01);
            if light.position_type.w > 1.5 {
                let cone = dot(-light_dir, normalize(light.direction_intensity.xyz));
                attenuation *= smoothstep(light.spot_cones.y, light.spot_cones.x, cone);
            }
        }
        var visibility = 1.0;
        if index == lighting.shadow_light_index {
            visibility = directional_shadow(in.world_pos);
        }
        direct += evaluate_brdf(normal, view_dir, light_dir, base_color, metallic, roughness)
            * light.color_range.rgb * attenuation * visibility;
    }
    var lit = ambient + direct + emissive;

    // Selection highlight feedback
    if (object.is_selected == 1u) {
        let selection_tint = vec3<f32>(1.0, 0.62, 0.18); // Warm orange highlight
        lit = mix(lit, selection_tint, 0.35);
    }

    let alpha = object.color.a * in.vertex_color.a * material.base_color_factor.a * base_sample.a;
    return vec4<f32>(aces_tonemap(lit), alpha);
}
"#;

/// Depth-only pass used to cast a single directional sun shadow in the viewport.
pub const SHADOW_SHADER: &str = r#"
struct ShadowCameraUniform {
    view_proj: mat4x4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: ShadowCameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * object.model * vec4<f32>(input.position, 1.0);
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

/// WGSL shader for GPU ID Buffer Picking.
/// Writes integer element IDs (u32) into an R32Uint render target.
pub const PICKING_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct PickingUniform {
    id_code: u32,
    _pad0: u32,
    viewport_size: vec2<f32>,
    point_size: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> picking_cfg: PickingUniform;

// 1. Mesh / Object picking
struct MeshVertexInput {
    @location(0) position: vec3<f32>,
};

struct MeshInstanceInput {
    @location(1) object_id: u32,
};

struct MeshVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) object_id: u32,
};

@vertex
fn vs_mesh(model: MeshVertexInput, instance: MeshInstanceInput) -> MeshVertexOutput {
    var out: MeshVertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.object_id = instance.object_id;
    return out;
}

@fragment
fn fs_mesh(in: MeshVertexOutput) -> @location(0) u32 {
    return in.object_id;
}

// 2. Vertex Handle picking (instanced billboarding)
struct QuadVertexInput {
    @location(0) quad_pos: vec2<f32>,
};

struct InstancePosInput {
    @location(1) center_pos: vec3<f32>,
};

struct InstanceIdInput {
    @location(2) id: u32,
};

struct VertexPickingOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) id: u32,
};

@vertex
fn vs_vertex(
    quad: QuadVertexInput,
    instance_pos: InstancePosInput,
    instance_id: InstanceIdInput,
) -> VertexPickingOutput {
    var out: VertexPickingOutput;
    let clip_center = camera.view_proj * vec4<f32>(instance_pos.center_pos, 1.0);
    let pixel_scale = 2.0 / max(picking_cfg.viewport_size, vec2<f32>(1.0, 1.0));
    let radius_px = picking_cfg.point_size * 0.65;
    let offset_ndc = quad.quad_pos * radius_px * pixel_scale;

    var final_clip = clip_center;
    final_clip.x += offset_ndc.x * clip_center.w;
    final_clip.y += offset_ndc.y * clip_center.w;
    final_clip.z -= 0.00030 * clip_center.w;

    out.clip_position = final_clip;
    out.uv = quad.quad_pos;
    out.id = instance_id.id;
    return out;
}

@fragment
fn fs_vertex(in: VertexPickingOutput) -> @location(0) u32 {
    if (length(in.uv) > 1.0) {
        discard;
    }
    return in.id;
}

// 3. Screen-Space Edge picking (with expanded screen-space hitbox)
struct EdgeVertexInput {
    @location(0) pos_a: vec3<f32>,
    @location(1) pos_b: vec3<f32>,
    @location(2) quad_corner: vec2<f32>,
    @location(3) edge_id: u32,
};

struct EdgePickingOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) edge_id: u32,
};

@vertex
fn vs_edge(input: EdgeVertexInput) -> EdgePickingOutput {
    var out: EdgePickingOutput;
    let clip_a = camera.view_proj * vec4<f32>(input.pos_a, 1.0);
    let clip_b = camera.view_proj * vec4<f32>(input.pos_b, 1.0);

    let ndc_a = clip_a.xy / max(clip_a.w, 0.00001);
    let ndc_b = clip_b.xy / max(clip_b.w, 0.00001);

    var delta = ndc_b - ndc_a;
    let len = length(delta);
    var dir = vec2<f32>(1.0, 0.0);
    if (len > 0.00001) {
        dir = delta / len;
    }
    let normal = vec2<f32>(-dir.y, dir.x);

    let pixel_scale = 2.0 / max(picking_cfg.viewport_size, vec2<f32>(1.0, 1.0));
    let half_width_px = 3.5;
    let offset_ndc = normal * (input.quad_corner.y * half_width_px) * pixel_scale;

    let t = (input.quad_corner.x + 1.0) * 0.5;
    var base_clip = mix(clip_a, clip_b, t);
    base_clip.x += offset_ndc.x * base_clip.w;
    base_clip.y += offset_ndc.y * base_clip.w;
    base_clip.z -= 0.00015 * base_clip.w;

    out.clip_position = base_clip;
    out.edge_id = input.edge_id;
    return out;
}

@fragment
fn fs_edge(in: EdgePickingOutput) -> @location(0) u32 {
    return in.edge_id;
}

// 4. Face picking with per-face ID
struct FaceVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) face_id: u32,
};

struct FacePickingOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) face_id: u32,
};

@vertex
fn vs_face(model: FaceVertexInput) -> FacePickingOutput {
    var out: FacePickingOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.face_id = model.face_id;
    return out;
}

@fragment
fn fs_face(in: FacePickingOutput) -> @location(0) u32 {
    return in.face_id;
}
"#;
