use forge_math::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::edge::HalfEdgeEdge;
use super::error::TopologyError;
use super::face::HalfEdgeFace;
use super::half_edge::HalfEdge;
use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};
use super::vertex::HalfEdgeVertex;

/// High-performance Half-Edge Mesh data structure.
///
/// Implements full adjacency for vertices, directed half-edges, undirected edges, and polygon faces.
/// Uses typed integer handles with generation counters to guard against dangling references
/// and handle re-use errors.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfEdgeMesh {
    pub(crate) vertices: Vec<HalfEdgeVertex>,
    pub(crate) half_edges: Vec<HalfEdge>,
    pub(crate) edges: Vec<HalfEdgeEdge>,
    pub(crate) faces: Vec<HalfEdgeFace>,

    pub(crate) free_vertices: Vec<u32>,
    pub(crate) free_half_edges: Vec<u32>,
    pub(crate) free_edges: Vec<u32>,
    pub(crate) free_faces: Vec<u32>,

    pub(crate) active_vertices: usize,
    pub(crate) active_half_edges: usize,
    pub(crate) active_edges: usize,
    pub(crate) active_faces: usize,
}

impl HalfEdgeMesh {
    /// Creates an empty `HalfEdgeMesh`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty `HalfEdgeMesh` with pre-allocated capacities.
    pub fn with_capacity(v_cap: usize, f_cap: usize) -> Self {
        let e_cap = (f_cap * 3) / 2;
        let h_cap = e_cap * 2;
        Self {
            vertices: Vec::with_capacity(v_cap),
            half_edges: Vec::with_capacity(h_cap),
            edges: Vec::with_capacity(e_cap),
            faces: Vec::with_capacity(f_cap),
            free_vertices: Vec::new(),
            free_half_edges: Vec::new(),
            free_edges: Vec::new(),
            free_faces: Vec::new(),
            active_vertices: 0,
            active_half_edges: 0,
            active_edges: 0,
            active_faces: 0,
        }
    }

    // --- Capacity & Counts ---

    #[inline]
    pub fn num_vertices(&self) -> usize {
        self.active_vertices
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.active_vertices
    }

    #[inline]
    pub fn num_half_edges(&self) -> usize {
        self.active_half_edges
    }

    #[inline]
    pub fn half_edge_count(&self) -> usize {
        self.active_half_edges
    }

    #[inline]
    pub fn num_edges(&self) -> usize {
        self.active_edges
    }

    #[inline]
    pub fn edge_count(&self) -> usize {
        self.active_edges
    }

    #[inline]
    pub fn num_faces(&self) -> usize {
        self.active_faces
    }

    #[inline]
    pub fn face_count(&self) -> usize {
        self.active_faces
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.active_vertices == 0 && self.active_faces == 0
    }

    /// Performs validation of topological integrity and generates a `ValidationReport`.
    #[inline]
    pub fn validate(&self) -> super::validation::ValidationReport {
        super::validation::TopologyValidator::validate(self)
    }

    /// Checks if the mesh satisfies all critical topological integrity invariants.
    #[inline]
    pub fn is_valid(&self) -> bool {
        super::validation::TopologyValidator::is_valid(self)
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.half_edges.clear();
        self.edges.clear();
        self.faces.clear();
        self.free_vertices.clear();
        self.free_half_edges.clear();
        self.free_edges.clear();
        self.free_faces.clear();
        self.active_vertices = 0;
        self.active_half_edges = 0;
        self.active_edges = 0;
        self.active_faces = 0;
    }

    // --- Handle Validation ---

    #[inline]
    pub fn is_valid_vertex(&self, h: VertexHandle) -> bool {
        let idx = h.index();
        idx < self.vertices.len()
            && !self.vertices[idx].deleted
            && self.vertices[idx].generation == h.generation
    }

    #[inline]
    pub fn is_valid_half_edge(&self, h: HalfEdgeHandle) -> bool {
        let idx = h.index();
        idx < self.half_edges.len()
            && !self.half_edges[idx].deleted
            && self.half_edges[idx].generation == h.generation
    }

    #[inline]
    pub fn is_valid_edge(&self, h: EdgeHandle) -> bool {
        let idx = h.index();
        idx < self.edges.len()
            && !self.edges[idx].deleted
            && self.edges[idx].generation == h.generation
    }

    #[inline]
    pub fn is_valid_face(&self, h: FaceHandle) -> bool {
        let idx = h.index();
        idx < self.faces.len()
            && !self.faces[idx].deleted
            && self.faces[idx].generation == h.generation
    }

    // --- Element Accessors ---

    #[inline]
    pub fn vertex(&self, h: VertexHandle) -> Option<&HalfEdgeVertex> {
        if self.is_valid_vertex(h) {
            Some(&self.vertices[h.index()])
        } else {
            None
        }
    }

    #[inline]
    pub fn vertex_mut(&mut self, h: VertexHandle) -> Option<&mut HalfEdgeVertex> {
        if self.is_valid_vertex(h) {
            let idx = h.index();
            Some(&mut self.vertices[idx])
        } else {
            None
        }
    }

    #[inline]
    pub fn half_edge(&self, h: HalfEdgeHandle) -> Option<&HalfEdge> {
        if self.is_valid_half_edge(h) {
            Some(&self.half_edges[h.index()])
        } else {
            None
        }
    }

    #[inline]
    pub fn half_edge_mut(&mut self, h: HalfEdgeHandle) -> Option<&mut HalfEdge> {
        if self.is_valid_half_edge(h) {
            let idx = h.index();
            Some(&mut self.half_edges[idx])
        } else {
            None
        }
    }

    #[inline]
    pub fn edge(&self, h: EdgeHandle) -> Option<&HalfEdgeEdge> {
        if self.is_valid_edge(h) {
            Some(&self.edges[h.index()])
        } else {
            None
        }
    }

    #[inline]
    pub fn edge_mut(&mut self, h: EdgeHandle) -> Option<&mut HalfEdgeEdge> {
        if self.is_valid_edge(h) {
            let idx = h.index();
            Some(&mut self.edges[idx])
        } else {
            None
        }
    }

    #[inline]
    pub fn face(&self, h: FaceHandle) -> Option<&HalfEdgeFace> {
        if self.is_valid_face(h) {
            Some(&self.faces[h.index()])
        } else {
            None
        }
    }

    #[inline]
    pub fn face_mut(&mut self, h: FaceHandle) -> Option<&mut HalfEdgeFace> {
        if self.is_valid_face(h) {
            let idx = h.index();
            Some(&mut self.faces[idx])
        } else {
            None
        }
    }

    // --- Element Allocation ---

    pub fn add_vertex(&mut self, position: Vec3) -> VertexHandle {
        if let Some(idx) = self.free_vertices.pop() {
            let i = idx as usize;
            let generation = self.vertices[i].generation;
            self.vertices[i] = HalfEdgeVertex {
                position,
                half_edge: None,
                normal: None,
                uv: None,
                color: None,
                generation,
                deleted: false,
            };
            self.active_vertices += 1;
            VertexHandle::new(idx, generation)
        } else {
            let idx = self.vertices.len() as u32;
            let v = HalfEdgeVertex::new(position);
            self.vertices.push(v);
            self.active_vertices += 1;
            VertexHandle::new(idx, 0)
        }
    }

    pub fn add_half_edge(
        &mut self,
        origin: VertexHandle,
        edge: EdgeHandle,
        face: Option<FaceHandle>,
    ) -> HalfEdgeHandle {
        if let Some(idx) = self.free_half_edges.pop() {
            let i = idx as usize;
            let generation = self.half_edges[i].generation;
            self.half_edges[i] = HalfEdge {
                origin,
                twin: HalfEdgeHandle::new(u32::MAX, 0),
                next: HalfEdgeHandle::new(u32::MAX, 0),
                prev: HalfEdgeHandle::new(u32::MAX, 0),
                face,
                edge,
                uv: None,
                normal: None,
                generation,
                deleted: false,
            };
            self.active_half_edges += 1;
            HalfEdgeHandle::new(idx, generation)
        } else {
            let idx = self.half_edges.len() as u32;
            let h = HalfEdge::new(origin, edge, face);
            self.half_edges.push(h);
            self.active_half_edges += 1;
            HalfEdgeHandle::new(idx, 0)
        }
    }

    pub fn add_edge(&mut self, half_edge: HalfEdgeHandle) -> EdgeHandle {
        if let Some(idx) = self.free_edges.pop() {
            let i = idx as usize;
            let generation = self.edges[i].generation;
            self.edges[i] = HalfEdgeEdge {
                half_edge,
                crease_weight: 0.0,
                generation,
                deleted: false,
            };
            self.active_edges += 1;
            EdgeHandle::new(idx, generation)
        } else {
            let idx = self.edges.len() as u32;
            let e = HalfEdgeEdge::new(half_edge);
            self.edges.push(e);
            self.active_edges += 1;
            EdgeHandle::new(idx, 0)
        }
    }

    pub fn add_face(&mut self, half_edge: HalfEdgeHandle) -> FaceHandle {
        if let Some(idx) = self.free_faces.pop() {
            let i = idx as usize;
            let generation = self.faces[i].generation;
            self.faces[i] = HalfEdgeFace {
                half_edge,
                normal: None,
                material_id: None,
                generation,
                deleted: false,
            };
            self.active_faces += 1;
            FaceHandle::new(idx, generation)
        } else {
            let idx = self.faces.len() as u32;
            let f = HalfEdgeFace::new(half_edge);
            self.faces.push(f);
            self.active_faces += 1;
            FaceHandle::new(idx, 0)
        }
    }

    // --- Deletion & Recycled Slots ---

    pub(crate) fn free_vertex(&mut self, h: VertexHandle) -> Result<(), TopologyError> {
        if !self.is_valid_vertex(h) {
            return Err(TopologyError::InvalidVertex(h));
        }
        let idx = h.index();
        self.vertices[idx].deleted = true;
        self.vertices[idx].generation = self.vertices[idx].generation.wrapping_add(1);
        self.free_vertices.push(h.raw_index());
        self.active_vertices -= 1;
        Ok(())
    }

    pub(crate) fn free_half_edge(&mut self, h: HalfEdgeHandle) -> Result<(), TopologyError> {
        if !self.is_valid_half_edge(h) {
            return Err(TopologyError::InvalidHalfEdge(h));
        }
        let idx = h.index();
        self.half_edges[idx].deleted = true;
        self.half_edges[idx].generation = self.half_edges[idx].generation.wrapping_add(1);
        self.free_half_edges.push(h.raw_index());
        self.active_half_edges -= 1;
        Ok(())
    }

    pub(crate) fn free_edge(&mut self, h: EdgeHandle) -> Result<(), TopologyError> {
        if !self.is_valid_edge(h) {
            return Err(TopologyError::InvalidEdge(h));
        }
        let idx = h.index();
        self.edges[idx].deleted = true;
        self.edges[idx].generation = self.edges[idx].generation.wrapping_add(1);
        self.free_edges.push(h.raw_index());
        self.active_edges -= 1;
        Ok(())
    }

    pub(crate) fn free_face(&mut self, h: FaceHandle) -> Result<(), TopologyError> {
        if !self.is_valid_face(h) {
            return Err(TopologyError::InvalidFace(h));
        }
        let idx = h.index();
        self.faces[idx].deleted = true;
        self.faces[idx].generation = self.faces[idx].generation.wrapping_add(1);
        self.free_faces.push(h.raw_index());
        self.active_faces -= 1;
        Ok(())
    }

    #[inline]
    pub fn remove_vertex(&mut self, h: VertexHandle) -> Result<(), TopologyError> {
        self.free_vertex(h)
    }

    #[inline]
    pub fn remove_half_edge(&mut self, h: HalfEdgeHandle) -> Result<(), TopologyError> {
        self.free_half_edge(h)
    }

    #[inline]
    pub fn remove_edge(&mut self, h: EdgeHandle) -> Result<(), TopologyError> {
        self.free_edge(h)
    }

    // --- Iterators over Active Elements ---

    pub fn vertices(&self) -> impl Iterator<Item = VertexHandle> + '_ {
        self.vertices
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.deleted)
            .map(|(i, v)| VertexHandle::new(i as u32, v.generation))
    }

    pub fn half_edges(&self) -> impl Iterator<Item = HalfEdgeHandle> + '_ {
        self.half_edges
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.deleted)
            .map(|(i, h)| HalfEdgeHandle::new(i as u32, h.generation))
    }

    pub fn edges(&self) -> impl Iterator<Item = EdgeHandle> + '_ {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.deleted)
            .map(|(i, e)| EdgeHandle::new(i as u32, e.generation))
    }

    pub fn faces(&self) -> impl Iterator<Item = FaceHandle> + '_ {
        self.faces
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.deleted)
            .map(|(i, f)| FaceHandle::new(i as u32, f.generation))
    }

    /// Performs basic validation of structural connectivity invariants.
    pub fn verify_basic_connectivity(&self) -> Result<(), TopologyError> {
        // 1. Verify vertex connectivity
        for vh in self.vertices() {
            let v = self.vertex(vh).ok_or(TopologyError::InvalidVertex(vh))?;
            if let Some(he_h) = v.half_edge
                && !self.is_valid_half_edge(he_h)
            {
                return Err(TopologyError::InvariantViolation(format!(
                    "Vertex {vh} outgoing half-edge {he_h} is invalid"
                )));
            }
        }

        // 2. Verify face connectivity & cycle closure
        for fh in self.faces() {
            let f = self.face(fh).ok_or(TopologyError::InvalidFace(fh))?;
            if !self.is_valid_half_edge(f.half_edge) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Face {fh} representative half-edge {} is invalid",
                    f.half_edge
                )));
            }

            let start_h = f.half_edge;
            let mut curr_h = start_h;
            let mut count = 0;
            loop {
                let he = self.half_edge(curr_h).ok_or_else(|| {
                    TopologyError::InvariantViolation(format!(
                        "Face {fh} contains invalid half-edge {curr_h}"
                    ))
                })?;

                if he.face != Some(fh) {
                    return Err(TopologyError::InvariantViolation(format!(
                        "Half-edge {curr_h} face pointer mismatch in face {fh}"
                    )));
                }

                curr_h = he.next;
                count += 1;
                if curr_h == start_h {
                    break;
                }
                if count > self.half_edges.len() {
                    return Err(TopologyError::InvariantViolation(format!(
                        "Cycle around face {fh} is not closed or exceeded maximum length"
                    )));
                }
            }

            if count < 3 {
                return Err(TopologyError::Degenerate(format!(
                    "Face {fh} cycle contains fewer than 3 half-edges ({count})"
                )));
            }
        }

        // 3. Verify edge connectivity
        for eh in self.edges() {
            let e = self.edge(eh).ok_or(TopologyError::InvalidEdge(eh))?;
            if !self.is_valid_half_edge(e.half_edge) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Edge {eh} references invalid half-edge {}",
                    e.half_edge
                )));
            }
        }

        // 4. Verify half-edge connectivity
        for he_h in self.half_edges() {
            let he = self
                .half_edge(he_h)
                .ok_or(TopologyError::InvalidHalfEdge(he_h))?;

            // Valid origin
            if !self.is_valid_vertex(he.origin) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} references invalid origin vertex {}",
                    he.origin
                )));
            }

            // Valid edge
            if !self.is_valid_edge(he.edge) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} references invalid edge {}",
                    he.edge
                )));
            }

            // Twin involution: twin.twin == self
            if !self.is_valid_half_edge(he.twin) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} references invalid twin {}",
                    he.twin
                )));
            }
            let twin = self.half_edge(he.twin).unwrap();
            if twin.twin != he_h {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} twin involution mismatch: twin({}).twin == {} != {he_h}",
                    he.twin, twin.twin
                )));
            }

            // Next / prev mutually consistent
            if !self.is_valid_half_edge(he.next) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} references invalid next {}",
                    he.next
                )));
            }
            let next_he = self.half_edge(he.next).unwrap();
            if next_he.prev != he_h {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} next-prev mismatch: next({}).prev == {} != {he_h}",
                    he.next, next_he.prev
                )));
            }

            if !self.is_valid_half_edge(he.prev) {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} references invalid prev {}",
                    he.prev
                )));
            }
            let prev_he = self.half_edge(he.prev).unwrap();
            if prev_he.next != he_h {
                return Err(TopologyError::InvariantViolation(format!(
                    "Half-edge {he_h} prev-next mismatch: prev({}).next == {} != {he_h}",
                    he.prev, prev_he.next
                )));
            }

            // Boundary half-edges must have face == None
            if he.is_boundary() && he.face.is_some() {
                return Err(TopologyError::InvariantViolation(format!(
                    "Boundary half-edge {he_h} unexpectedly has face {:?}",
                    he.face
                )));
            }
        }

        Ok(())
    }
}
