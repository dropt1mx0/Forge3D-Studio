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
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

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
        let project: ForgeProject = serde_json::from_str(json_str).map_err(|e| {
            CoreError::SerializationError(format!("Failed to parse .forge project JSON: {e}"))
        })?;

        if project.schema_version > Self::CURRENT_SCHEMA_VERSION {
            return Err(CoreError::SerializationError(format!(
                "Project schema version {} is newer than supported ({})",
                project.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            )));
        }

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
}
