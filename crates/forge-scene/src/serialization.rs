use forge_core::error::{CoreError, Result};
use serde::{Deserialize, Serialize};

use crate::scene::Scene;

/// Forge3D Project container file schema (`.forge`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeProject {
    pub schema_version: u32,
    pub app_version: String,
    pub name: String,
    pub scene: Scene,
}

impl ForgeProject {
    pub const CURRENT_SCHEMA_VERSION: u32 = 2;

    /// Creates a new project from a scene.
    pub fn new(name: impl Into<String>, scene: Scene) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            name: name.into(),
            scene,
        }
    }

    /// Serializes the project to formatted JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| CoreError::SerializationError(format!("Failed to serialize project: {e}")))
    }

    /// Deserializes a project from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self> {
        let mut project: ForgeProject = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SerializationError(format!("Failed to parse .forge project JSON: {e}"))
        })?;

        if project.schema_version > Self::CURRENT_SCHEMA_VERSION {
            return Err(CoreError::SerializationError(format!(
                "Project schema version {} is newer than supported ({})",
                project.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            )));
        }

        if project.schema_version < 2 {
            let is_showcase = project.scene.name == "House, Tree & Rock Showcase";
            for object in project.scene.objects.values_mut() {
                let material_id = match object.name.as_str() {
                    "Tree Trunk" if is_showcase => Some("polyhaven/bark_brown_01"),
                    name if is_showcase && name.starts_with("Tree Branch ") => {
                        Some("polyhaven/bark_brown_01")
                    }
                    name if is_showcase && name.starts_with("Garden Fence ") => {
                        Some("polyhaven/wooden_planks")
                    }
                    name if is_showcase && name.starts_with("Porch ") && name != "Porch Deck" => {
                        Some("polyhaven/wooden_planks")
                    }
                    "Rock" | "Pebble" if is_showcase => Some("polyhaven/mossy_rock"),
                    name if is_showcase && name.starts_with("Stepping Stone ") => {
                        Some("polyhaven/mossy_rock")
                    }
                    _ => None,
                };
                if object.material_id.is_none()
                    && let Some(material_id) = material_id
                {
                    object.material_id = Some(material_id.into());
                    for vertex in &mut object.mesh.vertices {
                        vertex.color = [1.0; 4];
                    }
                }
            }

            for (material_id, definition) in [
                (
                    crate::scene::BROWN_FLOOR_TILES_MATERIAL_ID,
                    crate::material::PbrMaterial::brown_floor_tiles(),
                ),
                (
                    "polyhaven/bark_brown_01",
                    crate::material::PbrMaterial::bark_brown_01(),
                ),
                (
                    "polyhaven/wooden_planks",
                    crate::material::PbrMaterial::wooden_planks(),
                ),
                (
                    "polyhaven/mossy_rock",
                    crate::material::PbrMaterial::mossy_rock(),
                ),
            ] {
                if project
                    .scene
                    .objects
                    .values()
                    .any(|object| object.material_id.as_deref() == Some(material_id))
                {
                    project
                        .scene
                        .materials
                        .entry(material_id.into())
                        .or_insert(definition);
                }
            }
        }
        project.schema_version = Self::CURRENT_SCHEMA_VERSION;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_roundtrip() {
        let scene = Scene::create_default_scene();
        let project = ForgeProject::new("Test Project", scene);

        let json = project.to_json().expect("Serialization failed");
        assert!(json.contains("schema_version"));

        let loaded = ForgeProject::from_json(&json).expect("Deserialization failed");
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.scene.objects.len(), 4);
        assert!(loaded.scene.active_camera_id.is_some());
    }

    #[test]
    fn version_one_showcase_project_migrates_its_built_in_material() {
        let mut scene = Scene::create_showcase_scene();
        scene.materials.clear();
        for object in scene.objects.values_mut() {
            if object.material_id.as_deref() != Some(crate::scene::BROWN_FLOOR_TILES_MATERIAL_ID) {
                object.material_id = None;
            }
        }
        let mut value = serde_json::to_value(ForgeProject::new("Legacy", scene)).unwrap();
        value["schema_version"] = serde_json::json!(1);
        value["scene"].as_object_mut().unwrap().remove("materials");

        let loaded = ForgeProject::from_json(&value.to_string()).unwrap();
        assert_eq!(loaded.schema_version, ForgeProject::CURRENT_SCHEMA_VERSION);
        let material = loaded
            .scene
            .materials
            .get(crate::scene::BROWN_FLOOR_TILES_MATERIAL_ID)
            .unwrap();
        assert!(material.textures.base_color.is_some());
        assert!(material.textures.normal.is_some());
        assert!(material.textures.roughness.is_some());
        assert!(material.textures.ambient_occlusion.is_some());
        for (object_name, material_id) in [
            ("Tree Trunk", "polyhaven/bark_brown_01"),
            ("Garden Fence Picket 1", "polyhaven/wooden_planks"),
            ("Rock", "polyhaven/mossy_rock"),
        ] {
            let object = loaded
                .scene
                .objects
                .values()
                .find(|object| object.name == object_name)
                .unwrap();
            assert_eq!(object.material_id.as_deref(), Some(material_id));
            assert!(loaded.scene.materials.contains_key(material_id));
        }
    }
}
