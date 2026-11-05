use forge_geometry::Mesh;
use serde::{Deserialize, Serialize};

use crate::array::ArrayModifier;
use crate::bevel::BevelModifier;
use crate::decimate::DecimateModifier;
use crate::displace::DisplaceModifier;
use crate::mirror::MirrorModifier;
use crate::solidify::SolidifyModifier;
use crate::subdiv::SubdivisionModifier;
use crate::weighted_normal::WeightedNormalModifier;

/// Supported modifier types in Forge3D.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModifierType {
    Mirror(MirrorModifier),
    Subdivide(SubdivisionModifier),
    Solidify(SolidifyModifier),
    Array(ArrayModifier),
    Bevel(BevelModifier),
    Decimate(DecimateModifier),
    Displace(DisplaceModifier),
    WeightedNormal(WeightedNormalModifier),
}

impl ModifierType {
    pub fn name(&self) -> &str {
        match self {
            Self::Mirror(_) => "Mirror",
            Self::Subdivide(_) => "Subdivision Surface",
            Self::Solidify(_) => "Solidify",
            Self::Array(_) => "Array",
            Self::Bevel(_) => "Bevel",
            Self::Decimate(_) => "Decimate",
            Self::Displace(_) => "Displace",
            Self::WeightedNormal(_) => "Weighted Normal",
        }
    }

    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        match self {
            Self::Mirror(m) => m.apply(mesh),
            Self::Subdivide(s) => s.apply(mesh),
            Self::Solidify(s) => s.apply(mesh),
            Self::Array(a) => a.apply(mesh),
            Self::Bevel(b) => b.apply(mesh),
            Self::Decimate(d) => d.apply(mesh),
            Self::Displace(dp) => dp.apply(mesh),
            Self::WeightedNormal(wn) => wn.apply(mesh),
        }
    }
}

/// An entry in an object's modifier stack with enabled toggle and name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierEntry {
    pub name: String,
    pub enabled: bool,
    pub modifier: ModifierType,
}

impl ModifierEntry {
    pub fn new(modifier: ModifierType) -> Self {
        let name = modifier.name().to_string();
        Self {
            name,
            enabled: true,
            modifier,
        }
    }
}

/// An ordered stack of non-destructive modifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModifierStack {
    pub modifiers: Vec<ModifierEntry>,
}

impl ModifierStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.modifiers.len()
    }

    pub fn add(&mut self, modifier: ModifierType) {
        self.modifiers.push(ModifierEntry::new(modifier));
    }

    pub fn remove(&mut self, index: usize) -> Option<ModifierEntry> {
        if index < self.modifiers.len() {
            Some(self.modifiers.remove(index))
        } else {
            None
        }
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.modifiers.len() {
            self.modifiers.swap(index, index - 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.modifiers.len() {
            self.modifiers.swap(index, index + 1);
        }
    }

    /// Evaluates all active modifiers sequentially onto `base_mesh`.
    pub fn evaluate(&self, base_mesh: &Mesh) -> Mesh {
        let mut current_mesh = base_mesh.clone();
        for entry in &self.modifiers {
            if entry.enabled {
                current_mesh = entry.modifier.apply(&current_mesh);
            }
        }
        current_mesh
    }

    /// Permanently applies the modifier at `index` to `base_mesh`, baking it and removing from stack.
    pub fn apply_to_mesh(&mut self, index: usize, base_mesh: &mut Mesh) -> bool {
        if index < self.modifiers.len() {
            let entry = self.modifiers.remove(index);
            if entry.enabled {
                *base_mesh = entry.modifier.apply(base_mesh);
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geometry::primitives::generate_cube;

    #[test]
    fn test_modifier_stack_pipeline() {
        let cube = generate_cube(1.0);
        let mut stack = ModifierStack::new();
        stack.add(ModifierType::Mirror(MirrorModifier::new_x()));
        stack.add(ModifierType::Array(ArrayModifier::new(
            2,
            forge_math::Vec3::new(0.0, 2.0, 0.0),
        )));

        let result = stack.evaluate(&cube);
        // Initial 24 verts -> mirror doubles to 48 -> array doubles to 96
        assert_eq!(result.vertices.len(), cube.vertices.len() * 2 * 2);
    }

    #[test]
    fn test_bevel_modifier_cube() {
        let cube = generate_cube(1.0);
        let bevel = BevelModifier::new(0.05);
        let result = bevel.apply(&cube);
        assert!(!result.vertices.is_empty());
        assert!(!result.indices.is_empty());
    }

    #[test]
    fn test_displace_modifier() {
        let cube = generate_cube(1.0);
        let displace = DisplaceModifier::new(0.2);
        let result = displace.apply(&cube);
        assert_eq!(result.vertices.len(), cube.vertices.len());
        assert_eq!(result.indices.len(), cube.indices.len());
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_decimate_modifier() {
        let cube = generate_cube(1.0);
        let decimate = DecimateModifier::new(0.5);
        let result = decimate.apply(&cube);
        assert!(!result.vertices.is_empty());
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_weighted_normal_modifier() {
        let cube = generate_cube(1.0);
        let wn = WeightedNormalModifier::new();
        let result = wn.apply(&cube);
        assert_eq!(result.vertices.len(), cube.vertices.len());
        assert_eq!(result.indices.len(), cube.indices.len());
        assert!(result.validate().is_ok());
    }
}
