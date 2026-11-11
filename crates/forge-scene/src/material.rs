//! Renderer-independent metallic-roughness material and scene light data.

use serde::{Deserialize, Serialize};

/// Image paths are project-relative unless they are absolute paths.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbrTextureSet {
    pub base_color: Option<String>,
    pub normal: Option<String>,
    pub metallic: Option<String>,
    pub roughness: Option<String>,
    pub ambient_occlusion: Option<String>,
    pub emissive: Option<String>,
}

/// glTF-style metallic-roughness surface parameters, expressed in linear space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PbrMaterial {
    pub name: String,
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive_factor: [f32; 3],
    pub uv_scale: [f32; 2],
    #[serde(default)]
    pub textures: PbrTextureSet,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: "New PBR Material".into(),
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.8,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            uv_scale: [1.0; 2],
            textures: PbrTextureSet::default(),
        }
    }
}

impl PbrMaterial {
    /// Packaged Poly Haven brown floor tile material with its full available map set.
    pub fn brown_floor_tiles() -> Self {
        Self {
            name: "Brown Floor Tiles (Poly Haven)".into(),
            roughness_factor: 1.0,
            textures: PbrTextureSet {
                base_color: Some(
                    "assets/textures/brown_floor_tiles/brown_floor_tiles_diff_4k.jpg".into(),
                ),
                normal: Some(
                    "assets/textures/brown_floor_tiles/brown_floor_tiles_nor_gl_4k.jpg".into(),
                ),
                roughness: Some(
                    "assets/textures/brown_floor_tiles/brown_floor_tiles_rough_4k.jpg".into(),
                ),
                ambient_occlusion: Some(
                    "assets/textures/brown_floor_tiles/brown_floor_tiles_ao_4k.jpg".into(),
                ),
                ..PbrTextureSet::default()
            },
            ..Self::default()
        }
    }

    /// Full 2K bark map set for tree trunks and branches.
    pub fn bark_brown_01() -> Self {
        Self::polyhaven_2k("Bark Brown 01", "bark_brown_01", "bark_brown_01")
    }

    /// Full 2K weathered wood map set for the garden fence and porch timber.
    pub fn wooden_planks() -> Self {
        Self::polyhaven_2k("Wooden Planks", "wooden_planks", "wooden_planks")
    }

    /// Full 2K mossy rock map set for the boulders and stepping stones.
    pub fn mossy_rock() -> Self {
        Self::polyhaven_2k("Mossy Rock", "mossy_rock", "mossy_rock")
    }

    fn polyhaven_2k(name: &str, directory: &str, stem: &str) -> Self {
        let texture_path = |map: &str| format!("assets/textures/{directory}/{stem}_{map}_2k.jpg");
        Self {
            name: name.into(),
            roughness_factor: 1.0,
            textures: PbrTextureSet {
                base_color: Some(texture_path("diff")),
                normal: Some(texture_path("nor_gl")),
                roughness: Some(texture_path("rough")),
                ambient_occlusion: Some(texture_path("ao")),
                ..PbrTextureSet::default()
            },
            ..Self::default()
        }
    }
}

/// Ambient environment approximation used when no environment map is assigned.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldLighting {
    pub sky_color: [f32; 3],
    pub ground_color: [f32; 3],
    pub intensity: f32,
}

impl Default for WorldLighting {
    fn default() -> Self {
        Self {
            sky_color: [0.65, 0.70, 0.78],
            ground_color: [0.45, 0.45, 0.48],
            intensity: 0.85,
        }
    }
}

/// Real-time light source attached to a scene object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LightData {
    pub kind: LightKind,
    pub enabled: bool,
    pub color: [f32; 3],
    /// Linear artist-controlled intensity; point and spot lights use inverse-square falloff.
    pub intensity: f32,
    /// Distance cutoff for point and spot lights. Directional lights ignore this field.
    pub range: f32,
    pub inner_cone_angle_deg: f32,
    pub outer_cone_angle_deg: f32,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            enabled: true,
            color: [1.0; 3],
            intensity: 30.0,
            range: 10.0,
            inner_cone_angle_deg: 25.0,
            outer_cone_angle_deg: 35.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightKind {
    Directional,
    #[default]
    Point,
    Spot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbr_material_round_trips_all_texture_channels() {
        let material = PbrMaterial {
            name: "Full PBR".into(),
            metallic_factor: 0.7,
            roughness_factor: 0.25,
            normal_scale: 0.8,
            occlusion_strength: 0.65,
            emissive_factor: [0.1, 0.2, 0.3],
            textures: PbrTextureSet {
                base_color: Some("maps/base.png".into()),
                normal: Some("maps/normal.png".into()),
                metallic: Some("maps/metal.png".into()),
                roughness: Some("maps/rough.png".into()),
                ambient_occlusion: Some("maps/ao.png".into()),
                emissive: Some("maps/emission.png".into()),
            },
            ..PbrMaterial::default()
        };
        let encoded = serde_json::to_string(&material).unwrap();
        let decoded: PbrMaterial = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, material);
    }

    #[test]
    fn old_material_json_uses_safe_pbr_defaults() {
        let decoded: PbrMaterial = serde_json::from_str(r#"{"name":"Legacy"}"#).unwrap();
        assert_eq!(decoded.name, "Legacy");
        assert_eq!(decoded.metallic_factor, 0.0);
        assert_eq!(decoded.roughness_factor, 0.8);
        assert!(decoded.textures.base_color.is_none());
    }
}
