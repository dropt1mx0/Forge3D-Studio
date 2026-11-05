//! Scripting and Extension system (Section 23: Scripting & Extensions).
//!
//! Provides a safe plugin lifecycle, runtime permission checks, custom operator
//! registrations, and event hooks for headless scripts and extensions.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Permissions requested by an extension in its manifest and verified at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginPermission {
    /// Permission to read files from disk.
    FileRead,
    /// Permission to write files to disk.
    FileWrite,
    /// Permission to create, modify, or delete scene objects, meshes, and materials.
    SceneMutate,
    /// Permission to open external network sockets.
    Network,
}

impl PluginPermission {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FileRead => "File Read",
            Self::FileWrite => "File Write",
            Self::SceneMutate => "Scene Mutation",
            Self::Network => "Network Access",
        }
    }
}

/// Metadata and capability manifest of an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub api_version: String,
    pub permissions: HashSet<PluginPermission>,
}

impl PluginManifest {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            author: author.into(),
            description: String::new(),
            api_version: "0.1.0".to_string(),
            permissions: HashSet::new(),
        }
    }

    pub fn with_permission(mut self, perm: PluginPermission) -> Self {
        self.permissions.insert(perm);
        self
    }
}

/// Operational state of an installed plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    Enabled,
    Disabled,
}

/// A registered plugin entry managed by `ExtensionManager`.
#[derive(Debug, Clone)]
pub struct RegisteredPlugin {
    pub manifest: PluginManifest,
    pub state: PluginState,
}

/// Operator definition registered by a plugin or built-in script.
#[derive(Debug, Clone)]
pub struct CustomOperator {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub category: String,
    pub required_permission: Option<PluginPermission>,
}

/// Event hook types fired during editor and scene operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventHook {
    SceneLoaded,
    ObjectAdded(forge_core::ObjectId),
    ObjectRemoved(forge_core::ObjectId),
    FrameRendered(u64),
    ProjectSaved(String),
}

/// Central manager for plugins, permissions, operators, and event hooks.
#[derive(Debug, Default)]
pub struct ExtensionManager {
    plugins: HashMap<String, RegisteredPlugin>,
    operators: HashMap<String, CustomOperator>,
    event_log: Vec<String>,
}

impl ExtensionManager {
    /// Creates a new empty extension manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new extension. Defaults to `PluginState::Enabled`.
    pub fn register_plugin(&mut self, manifest: PluginManifest) -> Result<(), String> {
        if self.plugins.contains_key(&manifest.id) {
            return Err(format!(
                "Plugin with id '{}' is already registered",
                manifest.id
            ));
        }

        self.plugins.insert(
            manifest.id.clone(),
            RegisteredPlugin {
                manifest,
                state: PluginState::Enabled,
            },
        );
        Ok(())
    }

    /// Enables a registered plugin.
    pub fn enable_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        let p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not found", plugin_id))?;
        p.state = PluginState::Enabled;
        Ok(())
    }

    /// Disables a registered plugin.
    pub fn disable_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        let p = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not found", plugin_id))?;
        p.state = PluginState::Disabled;
        Ok(())
    }

    /// Registers a custom operator associated with a plugin.
    pub fn register_operator(&mut self, op: CustomOperator) -> Result<(), String> {
        if !self.plugins.contains_key(&op.plugin_id) {
            return Err(format!(
                "Cannot register operator: plugin '{}' is not registered",
                op.plugin_id
            ));
        }

        self.operators.insert(op.id.clone(), op);
        Ok(())
    }

    /// Validates whether an operator is eligible to run (plugin is enabled and holds required permissions).
    pub fn validate_operator_execution(&self, op_id: &str) -> Result<&CustomOperator, String> {
        let op = self
            .operators
            .get(op_id)
            .ok_or_else(|| format!("Operator '{}' not found", op_id))?;

        let plugin = self
            .plugins
            .get(&op.plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not found", op.plugin_id))?;

        if plugin.state != PluginState::Enabled {
            return Err(format!("Plugin '{}' is disabled", plugin.manifest.name));
        }

        if let Some(req_perm) = op.required_permission
            && !plugin.manifest.permissions.contains(&req_perm)
        {
            return Err(format!(
                "Security Sandbox Violation: Plugin '{}' lacks required permission '{}'",
                plugin.manifest.name,
                req_perm.name()
            ));
        }

        Ok(op)
    }

    /// Dispatches an event hook to all active listeners and logs telemetry.
    pub fn dispatch_event(&mut self, event: EventHook) {
        let log_entry = format!("Event: {:?}", event);
        self.event_log.push(log_entry);
        if self.event_log.len() > 100 {
            self.event_log.remove(0);
        }
    }

    /// Returns an iterator over all registered plugins.
    pub fn plugins(&self) -> impl Iterator<Item = &RegisteredPlugin> {
        self.plugins.values()
    }

    /// Returns the total count of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Returns the count of currently enabled plugins.
    pub fn active_plugin_count(&self) -> usize {
        self.plugins
            .values()
            .filter(|p| p.state == PluginState::Enabled)
            .count()
    }

    /// Returns an iterator over all registered operators.
    pub fn operators(&self) -> impl Iterator<Item = &CustomOperator> {
        self.operators.values()
    }

    /// Returns recent event logs.
    pub fn event_logs(&self) -> &[String] {
        &self.event_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registration_and_permission_sandbox() {
        let mut mgr = ExtensionManager::new();

        let manifest = PluginManifest::new(
            "org.forge3d.batch_export",
            "Batch Exporter",
            "1.0.0",
            "Forge3D Team",
        )
        .with_permission(PluginPermission::FileWrite);

        mgr.register_plugin(manifest).unwrap();
        assert_eq!(mgr.plugin_count(), 1);
        assert_eq!(mgr.active_plugin_count(), 1);

        // Register operator requiring FileWrite
        let op_valid = CustomOperator {
            id: "batch_export_all".to_string(),
            plugin_id: "org.forge3d.batch_export".to_string(),
            title: "Batch Export All".to_string(),
            category: "Export".to_string(),
            required_permission: Some(PluginPermission::FileWrite),
        };
        mgr.register_operator(op_valid).unwrap();

        // Register operator requiring Network (not granted!)
        let op_unauthorized = CustomOperator {
            id: "cloud_sync".to_string(),
            plugin_id: "org.forge3d.batch_export".to_string(),
            title: "Cloud Sync".to_string(),
            category: "Network".to_string(),
            required_permission: Some(PluginPermission::Network),
        };
        mgr.register_operator(op_unauthorized).unwrap();

        // Allowed execution
        assert!(mgr.validate_operator_execution("batch_export_all").is_ok());

        // Blocked execution: lacks permission
        let err = mgr.validate_operator_execution("cloud_sync").unwrap_err();
        assert!(err.contains("Security Sandbox Violation"));

        // Disable plugin
        mgr.disable_plugin("org.forge3d.batch_export").unwrap();
        assert_eq!(mgr.active_plugin_count(), 0);

        // Blocked execution: plugin disabled
        let err = mgr
            .validate_operator_execution("batch_export_all")
            .unwrap_err();
        assert!(err.contains("is disabled"));
    }

    #[test]
    fn test_event_hook_dispatch() {
        let mut mgr = ExtensionManager::new();
        mgr.dispatch_event(EventHook::SceneLoaded);
        mgr.dispatch_event(EventHook::ProjectSaved("test.forge".to_string()));

        assert_eq!(mgr.event_logs().len(), 2);
        assert!(mgr.event_logs()[0].contains("SceneLoaded"));
        assert!(mgr.event_logs()[1].contains("test.forge"));
    }
}
