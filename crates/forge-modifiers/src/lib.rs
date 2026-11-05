//! # Forge3D Modifiers
//!
//! Non-destructive modifier stack architecture and procedural geometric modifiers
//! (Mirror, Subdivision Surface, Solidify, Array).

pub mod array;
pub mod bevel;
pub mod decimate;
pub mod displace;
pub mod mirror;
pub mod solidify;
pub mod stack;
pub mod subdiv;
pub mod weighted_normal;

pub use array::ArrayModifier;
pub use bevel::BevelModifier;
pub use decimate::DecimateModifier;
pub use displace::DisplaceModifier;
pub use mirror::MirrorModifier;
pub use solidify::SolidifyModifier;
pub use stack::{ModifierEntry, ModifierStack, ModifierType};
pub use subdiv::SubdivisionModifier;
pub use weighted_normal::WeightedNormalModifier;
