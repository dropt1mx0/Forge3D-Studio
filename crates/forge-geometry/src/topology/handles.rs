use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type-safe, compact handle referencing a vertex in a `HalfEdgeMesh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VertexHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl VertexHandle {
    pub const NULL: Self = Self::new(u32::MAX, 0);

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline]
    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        Self {
            index: index as u32,
            generation: 0,
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        !self.is_null()
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn raw_index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for VertexHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}:g{}", self.index, self.generation)
    }
}

/// Type-safe, compact handle referencing a directed half-edge in a `HalfEdgeMesh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdgeHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl HalfEdgeHandle {
    pub const NULL: Self = Self::new(u32::MAX, 0);

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline]
    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        Self {
            index: index as u32,
            generation: 0,
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        !self.is_null()
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn raw_index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for HalfEdgeHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "h{}:g{}", self.index, self.generation)
    }
}

/// Type-safe, compact handle referencing an undirected edge in a `HalfEdgeMesh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl EdgeHandle {
    pub const NULL: Self = Self::new(u32::MAX, 0);

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline]
    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        Self {
            index: index as u32,
            generation: 0,
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        !self.is_null()
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn raw_index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for EdgeHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}:g{}", self.index, self.generation)
    }
}

/// Type-safe, compact handle referencing a polygon face in a `HalfEdgeMesh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FaceHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl FaceHandle {
    pub const NULL: Self = Self::new(u32::MAX, 0);

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline]
    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        Self {
            index: index as u32,
            generation: 0,
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        !self.is_null()
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn raw_index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for FaceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "f{}:g{}", self.index, self.generation)
    }
}

impl Default for VertexHandle {
    fn default() -> Self {
        Self::NULL
    }
}

impl Default for HalfEdgeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

impl Default for EdgeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

impl Default for FaceHandle {
    fn default() -> Self {
        Self::NULL
    }
}
