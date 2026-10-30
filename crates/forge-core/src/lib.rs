//! # Forge3D Core
//!
//! Foundational, application-independent identifiers, traits, and error types.
//!
//! Strictly contains NO GPU or UI dependencies.

pub mod error;
pub mod id;
pub mod traits;

pub use error::{CoreError, Result};
pub use id::{AssetId, ObjectId};
pub use traits::{Identifiable, Named};
