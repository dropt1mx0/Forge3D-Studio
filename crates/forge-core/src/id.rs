use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Unique identifier for a scene object or node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ObjectId(u64);

static OBJECT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ObjectId {
    /// Generates a new unique `ObjectId`.
    pub fn new() -> Self {
        Self(OBJECT_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Constructs an `ObjectId` from an explicit raw integer.
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw integer value of the ID.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Obj#{}", self.0)
    }
}

/// Unique identifier for an asset (mesh, material, texture, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AssetId(u64);

static ASSET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl AssetId {
    /// Generates a new unique `AssetId`.
    pub fn new() -> Self {
        Self(ASSET_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Constructs an `AssetId` from an explicit raw integer.
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw integer value of the ID.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Asset#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_object_ids() {
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_raw_roundtrip() {
        let id = ObjectId::from_raw(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "Obj#42");
    }
}
