use std::collections::HashSet;

use forge_math::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::error::TopologyError;
use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};
use super::mesh::HalfEdgeMesh;

/// The typed result of a successful `split_edge` topological operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SplitEdgeResult {
    /// The newly created topological vertex along the edge.
    pub vertex: VertexHandle,
    /// The original edge handle, which now represents the segment from `v0` to `v_new`.
    pub original_edge: EdgeHandle,
    /// The newly created edge handle, which represents the segment from `v_new` to `v1`.
    pub new_edge: EdgeHandle,
    /// The two newly created directed half-edges: `(h0b, h1b)`.
    /// - `h0b` is directed from `v_new` to `v1` in face `f0`.
    /// - `h1b` is directed from `v_new` to `v0` in face `f1`.
    pub new_half_edges: (HalfEdgeHandle, HalfEdgeHandle),
    /// The incident faces affected by the split: `(f0, f1)`.
    pub affected_faces: (Option<FaceHandle>, Option<FaceHandle>),
}

impl SplitEdgeResult {
    #[inline]
    pub fn vertex(&self) -> VertexHandle {
        self.vertex
    }

    #[inline]
    pub fn original_edge(&self) -> EdgeHandle {
        self.original_edge
    }

    #[inline]
    pub fn new_edge(&self) -> EdgeHandle {
        self.new_edge
    }

    #[inline]
    pub fn edges(&self) -> (EdgeHandle, EdgeHandle) {
        (self.original_edge, self.new_edge)
    }

    #[inline]
    pub fn new_half_edges(&self) -> (HalfEdgeHandle, HalfEdgeHandle) {
        self.new_half_edges
    }

    #[inline]
    pub fn affected_faces(&self) -> (Option<FaceHandle>, Option<FaceHandle>) {
        self.affected_faces
    }
}

/// The typed result of a successful `flip_edge` topological operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FlipEdgeResult {
    /// The edge that was flipped (now connecting the opposite vertices).
    pub edge: EdgeHandle,
    /// The two triangular faces affected by the flip: `(f0, f1)`.
    pub faces: (FaceHandle, FaceHandle),
    /// The four vertices involved in the flip: `(v0, v1, v2, v3)`.
    /// - `v0, v1`: original endpoints of the edge before flip.
    /// - `v2, v3`: opposite vertices that now form the new diagonal.
    pub vertices: (VertexHandle, VertexHandle, VertexHandle, VertexHandle),
    /// The endpoints of the new diagonal: `(v2, v3)`.
    pub new_diagonal: (VertexHandle, VertexHandle),
    /// The two directed half-edges of the flipped edge: `(h0, h1)`.
    pub half_edges: (HalfEdgeHandle, HalfEdgeHandle),
}

impl FlipEdgeResult {
    #[inline]
    pub fn edge(&self) -> EdgeHandle {
        self.edge
    }

    #[inline]
    pub fn faces(&self) -> (FaceHandle, FaceHandle) {
        self.faces
    }

    #[inline]
    pub fn vertices(&self) -> (VertexHandle, VertexHandle, VertexHandle, VertexHandle) {
        self.vertices
    }

    #[inline]
    pub fn new_diagonal(&self) -> (VertexHandle, VertexHandle) {
        self.new_diagonal
    }

    #[inline]
    pub fn half_edges(&self) -> (HalfEdgeHandle, HalfEdgeHandle) {
        self.half_edges
    }
}

/// The typed result of a successful `collapse_edge` topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CollapseEdgeResult {
    /// The surviving vertex that now represents the merged endpoints.
    pub surviving_vertex: VertexHandle,
    /// The vertex that was removed/merged into `surviving_vertex`.
    pub removed_vertex: VertexHandle,
    /// The edge that was collapsed and removed.
    pub collapsed_edge: EdgeHandle,
    /// The faces that became degenerate and were removed (e.g. incident triangles).
    pub removed_faces: Vec<FaceHandle>,
    /// The edges that were removed during the collapse (the collapsed edge plus merged triangle edges).
    pub removed_edges: Vec<EdgeHandle>,
    /// The half-edges that were removed during the collapse.
    pub removed_half_edges: Vec<HalfEdgeHandle>,
    /// The remaining active edges incident to `surviving_vertex` affected by the merge.
    pub affected_edges: Vec<EdgeHandle>,
}

impl CollapseEdgeResult {
    #[inline]
    pub fn surviving_vertex(&self) -> VertexHandle {
        self.surviving_vertex
    }

    #[inline]
    pub fn vertex(&self) -> VertexHandle {
        self.surviving_vertex
    }

    #[inline]
    pub fn removed_vertex(&self) -> VertexHandle {
        self.removed_vertex
    }

    #[inline]
    pub fn collapsed_edge(&self) -> EdgeHandle {
        self.collapsed_edge
    }

    #[inline]
    pub fn removed_faces(&self) -> &[FaceHandle] {
        &self.removed_faces
    }

    #[inline]
    pub fn removed_edges(&self) -> &[EdgeHandle] {
        &self.removed_edges
    }

    #[inline]
    pub fn removed_half_edges(&self) -> &[HalfEdgeHandle] {
        &self.removed_half_edges
    }

    #[inline]
    pub fn affected_edges(&self) -> &[EdgeHandle] {
        &self.affected_edges
    }

    /// Total count of all elements removed by the operation (vertex + edges + half-edges + faces).
    #[inline]
    pub fn removed_elements_count(&self) -> usize {
        1 + self.removed_edges.len() + self.removed_half_edges.len() + self.removed_faces.len()
    }
}

impl std::ops::Deref for CollapseEdgeResult {
    type Target = VertexHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.surviving_vertex
    }
}

/// The typed result of a successful `split_face` topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SplitFaceResult {
    /// The original face handle (retained for the first sub-face cycle).
    pub original_face: FaceHandle,
    /// The newly created face handle (allocated for the second sub-face cycle).
    pub new_face: FaceHandle,
    /// The newly created interior edge handle dividing the original face.
    pub new_edge: EdgeHandle,
    /// The two newly created directed half-edges of the new edge: `(h_ab, h_ba)`.
    /// - `h_ab`: directed from `vertex_a` to `vertex_b` in `new_face`.
    /// - `h_ba`: directed from `vertex_b` to `vertex_a` in `original_face`.
    pub new_half_edges: (HalfEdgeHandle, HalfEdgeHandle),
    /// The ordered list of vertices forming the cycle of `original_face`.
    pub face1_vertices: Vec<VertexHandle>,
    /// The ordered list of vertices forming the cycle of `new_face`.
    pub face2_vertices: Vec<VertexHandle>,
}

impl SplitFaceResult {
    #[inline]
    pub fn original_face(&self) -> FaceHandle {
        self.original_face
    }

    #[inline]
    pub fn new_face(&self) -> FaceHandle {
        self.new_face
    }

    #[inline]
    pub fn faces(&self) -> (FaceHandle, FaceHandle) {
        (self.original_face, self.new_face)
    }

    #[inline]
    pub fn new_edge(&self) -> EdgeHandle {
        self.new_edge
    }

    #[inline]
    pub fn edge(&self) -> EdgeHandle {
        self.new_edge
    }

    #[inline]
    pub fn new_half_edges(&self) -> (HalfEdgeHandle, HalfEdgeHandle) {
        self.new_half_edges
    }

    #[inline]
    pub fn face1_vertices(&self) -> &[VertexHandle] {
        &self.face1_vertices
    }

    #[inline]
    pub fn face2_vertices(&self) -> &[VertexHandle] {
        &self.face2_vertices
    }
}

impl From<SplitFaceResult> for (FaceHandle, EdgeHandle) {
    #[inline]
    fn from(res: SplitFaceResult) -> Self {
        (res.new_face, res.new_edge)
    }
}

/// The typed result of a successful `dissolve_edge` (merge faces) topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MergeFacesResult {
    /// The surviving face handle that now encompasses the merged polygon.
    pub surviving_face: FaceHandle,
    /// The face handle that was removed and freed during the merge.
    pub removed_face: FaceHandle,
    /// The edge handle that was dissolved and freed.
    pub dissolved_edge: EdgeHandle,
    /// The ordered list of vertices along the outer boundary of the merged face.
    pub boundary_vertices: Vec<VertexHandle>,
}

impl MergeFacesResult {
    #[inline]
    pub fn surviving_face(&self) -> FaceHandle {
        self.surviving_face
    }

    #[inline]
    pub fn removed_face(&self) -> FaceHandle {
        self.removed_face
    }

    #[inline]
    pub fn dissolved_edge(&self) -> EdgeHandle {
        self.dissolved_edge
    }

    #[inline]
    pub fn edge(&self) -> EdgeHandle {
        self.dissolved_edge
    }

    #[inline]
    pub fn boundary_vertices(&self) -> &[VertexHandle] {
        &self.boundary_vertices
    }

    #[inline]
    pub fn vertices(&self) -> &[VertexHandle] {
        &self.boundary_vertices
    }
}

impl std::ops::Deref for MergeFacesResult {
    type Target = FaceHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.surviving_face
    }
}

impl From<MergeFacesResult> for FaceHandle {
    #[inline]
    fn from(res: MergeFacesResult) -> Self {
        res.surviving_face
    }
}

/// The typed result of a successful `poke_face` topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PokeFaceResult {
    /// The original face handle, which is retained and transformed into one of the fan triangles.
    pub original_face: FaceHandle,
    /// The newly created center vertex handle.
    pub new_vertex: VertexHandle,
    /// The additional triangular faces created to form the fan (length = N - 1).
    pub new_faces: Vec<FaceHandle>,
    /// The radial edges connecting the center vertex to each boundary vertex (length = N).
    pub new_edges: Vec<EdgeHandle>,
    /// The directed radial half-edges created for the fan (length = 2 * N).
    pub new_half_edges: Vec<HalfEdgeHandle>,
}

impl PokeFaceResult {
    #[inline]
    pub fn original_face(&self) -> FaceHandle {
        self.original_face
    }

    #[inline]
    pub fn new_vertex(&self) -> VertexHandle {
        self.new_vertex
    }

    #[inline]
    pub fn vertex(&self) -> VertexHandle {
        self.new_vertex
    }

    #[inline]
    pub fn new_faces(&self) -> &[FaceHandle] {
        &self.new_faces
    }

    #[inline]
    pub fn new_edges(&self) -> &[EdgeHandle] {
        &self.new_edges
    }

    #[inline]
    pub fn new_half_edges(&self) -> &[HalfEdgeHandle] {
        &self.new_half_edges
    }
}

impl std::ops::Deref for PokeFaceResult {
    type Target = VertexHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.new_vertex
    }
}

impl From<PokeFaceResult> for VertexHandle {
    #[inline]
    fn from(res: PokeFaceResult) -> Self {
        res.new_vertex
    }
}

/// The typed result of a successful `extrude_face` topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ExtrudeFaceResult {
    /// The original face handle, preserved as the bottom face.
    pub original_face: FaceHandle,
    /// The newly created extruded top face handle.
    pub top_face: FaceHandle,
    /// The newly created vertex handles forming the top face (length = N).
    pub new_vertices: Vec<VertexHandle>,
    /// The newly created edge handles (top edges + pillar edges, length = 2 * N).
    pub new_edges: Vec<EdgeHandle>,
    /// The newly created face handles (top face + side faces, length = 1 + N).
    pub new_faces: Vec<FaceHandle>,
    /// The newly created half-edge handles (top half-edges + pillar half-edges, length = 4 * N).
    pub new_half_edges: Vec<HalfEdgeHandle>,
}

impl ExtrudeFaceResult {
    #[inline]
    pub fn original_face(&self) -> FaceHandle {
        self.original_face
    }

    #[inline]
    pub fn top_face(&self) -> FaceHandle {
        self.top_face
    }

    #[inline]
    pub fn face(&self) -> FaceHandle {
        self.top_face
    }

    #[inline]
    pub fn new_vertices(&self) -> &[VertexHandle] {
        &self.new_vertices
    }

    #[inline]
    pub fn new_edges(&self) -> &[EdgeHandle] {
        &self.new_edges
    }

    #[inline]
    pub fn new_faces(&self) -> &[FaceHandle] {
        &self.new_faces
    }

    #[inline]
    pub fn new_half_edges(&self) -> &[HalfEdgeHandle] {
        &self.new_half_edges
    }
}

impl std::ops::Deref for ExtrudeFaceResult {
    type Target = FaceHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.top_face
    }
}

impl From<ExtrudeFaceResult> for FaceHandle {
    #[inline]
    fn from(res: ExtrudeFaceResult) -> Self {
        res.top_face
    }
}

/// The typed result of a successful `inset_face` topological operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InsetFaceResult {
    /// The original face handle, preserved as the first side face (side face 0) in the surrounding region.
    pub original_face: FaceHandle,
    /// The newly created inner face handle.
    pub inner_face: FaceHandle,
    /// The newly created inner vertex handles (length = N).
    pub new_vertices: Vec<VertexHandle>,
    /// The newly created edge handles (inner edges + radial edges, length = 2 * N).
    pub new_edges: Vec<EdgeHandle>,
    /// The newly created side face handles (length = N - 1).
    pub new_side_faces: Vec<FaceHandle>,
    /// The newly created half-edge handles (inner half-edges + radial half-edges, length = 4 * N).
    pub new_half_edges: Vec<HalfEdgeHandle>,
}

impl InsetFaceResult {
    #[inline]
    pub fn original_face(&self) -> FaceHandle {
        self.original_face
    }

    #[inline]
    pub fn inner_face(&self) -> FaceHandle {
        self.inner_face
    }

    #[inline]
    pub fn face(&self) -> FaceHandle {
        self.inner_face
    }

    #[inline]
    pub fn new_vertices(&self) -> &[VertexHandle] {
        &self.new_vertices
    }

    #[inline]
    pub fn new_edges(&self) -> &[EdgeHandle] {
        &self.new_edges
    }

    #[inline]
    pub fn new_side_faces(&self) -> &[FaceHandle] {
        &self.new_side_faces
    }

    /// Returns all side faces forming the surrounding region (length = N).
    ///
    /// The first side face is `original_face`, followed by `new_side_faces`.
    #[inline]
    pub fn side_faces(&self) -> Vec<FaceHandle> {
        let mut faces = Vec::with_capacity(1 + self.new_side_faces.len());
        faces.push(self.original_face);
        faces.extend_from_slice(&self.new_side_faces);
        faces
    }

    #[inline]
    pub fn new_half_edges(&self) -> &[HalfEdgeHandle] {
        &self.new_half_edges
    }
}

impl std::ops::Deref for InsetFaceResult {
    type Target = FaceHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner_face
    }
}

impl From<InsetFaceResult> for FaceHandle {
    #[inline]
    fn from(res: InsetFaceResult) -> Self {
        res.inner_face
    }
}

impl HalfEdgeMesh {
    /// Inserts a new vertex along edge `e` at linear interpolation parameter `t` in (0, 1).
    ///
    /// Subdivides the edge into two edges and updates the incident face/boundary cycles.
    /// Transactionally rolls back if an invariant is violated.
    pub fn split_edge(
        &mut self,
        edge: EdgeHandle,
        t: f32,
    ) -> Result<SplitEdgeResult, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }
        if !t.is_finite() || t <= 0.0 || t >= 1.0 {
            return Err(TopologyError::InvalidOperation(format!(
                "Split edge parameter t ({t}) must be strictly within (0.0, 1.0)"
            )));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let v0 = self.half_edge(h0).unwrap().origin;
        let v1 = self.half_edge(h1).unwrap().origin;

        let p0 = self.vertex(v0).unwrap().position;
        let p1 = self.vertex(v1).unwrap().position;
        let p_new = p0 + (p1 - p0) * t;

        // Allocate new vertex
        let v_new = self.add_vertex(p_new);

        // Interpolate vertex normal if available
        if let (Some(n0), Some(n1)) = (
            self.vertex(v0).unwrap().normal,
            self.vertex(v1).unwrap().normal,
        ) {
            let n_new = ((1.0 - t) * n0 + t * n1).normalize_or_zero();
            self.vertex_mut(v_new).unwrap().normal = Some(n_new);
        }

        // Interpolate vertex UV if available
        if let (Some(uv0), Some(uv1)) = (self.vertex(v0).unwrap().uv, self.vertex(v1).unwrap().uv) {
            let uv_new = (1.0 - t) * uv0 + t * uv1;
            self.vertex_mut(v_new).unwrap().uv = Some(uv_new);
        }

        // Interpolate vertex RGBA color if available
        if let (Some(c0), Some(c1)) = (
            self.vertex(v0).unwrap().color,
            self.vertex(v1).unwrap().color,
        ) {
            let c_new = [
                (1.0 - t) * c0[0] + t * c1[0],
                (1.0 - t) * c0[1] + t * c1[1],
                (1.0 - t) * c0[2] + t * c1[2],
                (1.0 - t) * c0[3] + t * c1[3],
            ];
            self.vertex_mut(v_new).unwrap().color = Some(c_new);
        }

        let f0 = self.half_edge(h0).unwrap().face;
        let f1 = self.half_edge(h1).unwrap().face;

        let h0_next = self.half_edge(h0).unwrap().next;
        let h1_next = self.half_edge(h1).unwrap().next;

        // Interpolate corner UV and normal on Side 0 (face f0)
        // h0 is v0 -> v1; h0_next starts at v1
        let uv0_f0 = self
            .half_edge(h0)
            .unwrap()
            .uv
            .or(self.vertex(v0).unwrap().uv);
        let uv1_f0 = self
            .half_edge(h0_next)
            .unwrap()
            .uv
            .or(self.vertex(v1).unwrap().uv);
        let uv_corner_f0 = match (uv0_f0, uv1_f0) {
            (Some(u0), Some(u1)) => Some((1.0 - t) * u0 + t * u1),
            _ => None,
        };

        let norm0_f0 = self
            .half_edge(h0)
            .unwrap()
            .normal
            .or(self.vertex(v0).unwrap().normal);
        let norm1_f0 = self
            .half_edge(h0_next)
            .unwrap()
            .normal
            .or(self.vertex(v1).unwrap().normal);
        let norm_corner_f0 = match (norm0_f0, norm1_f0) {
            (Some(n0), Some(n1)) => Some(((1.0 - t) * n0 + t * n1).normalize_or_zero()),
            _ => None,
        };

        // Interpolate corner UV and normal on Side 1 (face f1)
        // h1 is v1 -> v0; h1_next starts at v0
        // parameter t is from v0 to v1: at v0, UV is uv_at_v0; at v1, UV is uv_at_v1
        let uv0_f1 = self
            .half_edge(h1_next)
            .unwrap()
            .uv
            .or(self.vertex(v0).unwrap().uv);
        let uv1_f1 = self
            .half_edge(h1)
            .unwrap()
            .uv
            .or(self.vertex(v1).unwrap().uv);
        let uv_corner_f1 = match (uv0_f1, uv1_f1) {
            (Some(u0), Some(u1)) => Some((1.0 - t) * u0 + t * u1),
            _ => None,
        };

        let norm0_f1 = self
            .half_edge(h1_next)
            .unwrap()
            .normal
            .or(self.vertex(v0).unwrap().normal);
        let norm1_f1 = self
            .half_edge(h1)
            .unwrap()
            .normal
            .or(self.vertex(v1).unwrap().normal);
        let norm_corner_f1 = match (norm0_f1, norm1_f1) {
            (Some(n0), Some(n1)) => Some(((1.0 - t) * n0 + t * n1).normalize_or_zero()),
            _ => None,
        };

        // Create new edge connecting v_new and v1
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let e_new = self.add_edge(dummy_he);

        // Create new half-edge h0b (v_new -> v1)
        let h0b = self.add_half_edge(v_new, e_new, f0);
        self.half_edge_mut(h0b).unwrap().uv = uv_corner_f0;
        self.half_edge_mut(h0b).unwrap().normal = norm_corner_f0;

        // Create new half-edge h1b (v_new -> v0)
        let h1b = self.add_half_edge(v_new, edge, f1);
        self.half_edge_mut(h1b).unwrap().uv = uv_corner_f1;
        self.half_edge_mut(h1b).unwrap().normal = norm_corner_f1;

        self.edge_mut(edge).unwrap().half_edge = h0;
        self.edge_mut(e_new).unwrap().half_edge = h0b;

        // Link twins
        // original edge connects h0 (v0 -> v_new) and h1b (v_new -> v0)
        self.half_edge_mut(h0).unwrap().twin = h1b;
        self.half_edge_mut(h1b).unwrap().twin = h0;

        // e_new connects h0b (v_new -> v1) and h1 (v1 -> v_new)
        self.half_edge_mut(h0b).unwrap().twin = h1;
        self.half_edge_mut(h1).unwrap().twin = h0b;
        self.half_edge_mut(h1).unwrap().edge = e_new;

        // Rewire cycle around f0: h0 -> h0b -> h0_next
        self.half_edge_mut(h0).unwrap().next = h0b;
        self.half_edge_mut(h0b).unwrap().prev = h0;
        self.half_edge_mut(h0b).unwrap().next = h0_next;
        self.half_edge_mut(h0_next).unwrap().prev = h0b;

        // Rewire cycle around f1: h1 -> h1b -> h1_next
        self.half_edge_mut(h1).unwrap().next = h1b;
        self.half_edge_mut(h1b).unwrap().prev = h1;
        self.half_edge_mut(h1b).unwrap().next = h1_next;
        self.half_edge_mut(h1_next).unwrap().prev = h1b;

        // Outgoing half-edges for vertices: prefer boundary half-edge if on boundary
        let v_new_out = if f0.is_none() {
            h0b
        } else if f1.is_none() {
            h1b
        } else {
            h0b
        };
        self.vertex_mut(v_new).unwrap().half_edge = Some(v_new_out);

        // Transactional invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(SplitEdgeResult {
            vertex: v_new,
            original_edge: edge,
            new_edge: e_new,
            new_half_edges: (h0b, h1b),
            affected_faces: (f0, f1),
        })
    }

    /// Flips an internal edge between two adjacent triangles.
    ///
    /// Preserves orientations, normals, and cycle integrity.
    /// Transactionally rolls back if an invariant is violated.
    pub fn flip_edge(&mut self, edge: EdgeHandle) -> Result<FlipEdgeResult, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let f0 = self.half_edge(h0).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation(
                "Cannot flip boundary edge: edge must have two incident faces".into(),
            )
        })?;
        let f1 = self.half_edge(h1).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation(
                "Cannot flip boundary edge: edge must have two incident faces".into(),
            )
        })?;

        if f0 == f1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot flip edge belonging to the same face on both sides".into(),
            ));
        }

        // Both incident faces must be triangles
        let f0_hes = self.face_half_edges(f0);
        let f1_hes = self.face_half_edges(f1);
        if f0_hes.len() != 3 || f1_hes.len() != 3 {
            return Err(TopologyError::InvalidOperation(
                "Flip edge is only supported between two triangular faces".into(),
            ));
        }

        // Identify vertices of triangles
        // Triangle 0 (v0, v1, v2):
        // h0: v0 -> v1
        // h0_next: v1 -> v2
        // h0_prev: v2 -> v0
        let v0 = self.half_edge(h0).unwrap().origin;
        let h0_next = self.half_edge(h0).unwrap().next;
        let v1 = self.half_edge(h0_next).unwrap().origin;
        let h0_prev = self.half_edge(h0).unwrap().prev;
        let v2 = self.half_edge(h0_prev).unwrap().origin;

        // Triangle 1 (v1, v0, v3):
        // h1: v1 -> v0
        // h1_next: v0 -> v3
        // h1_prev: v3 -> v1
        let h1_next = self.half_edge(h1).unwrap().next;
        let h1_prev = self.half_edge(h1).unwrap().prev;
        let v3 = self.half_edge(h1_prev).unwrap().origin;

        if v2 == v3 {
            return Err(TopologyError::InvalidOperation(
                "Degenerate edge flip: opposite vertices coincide".into(),
            ));
        }

        // Check if an edge connecting v2 and v3 already exists
        for eh in self.incident_edges(v2) {
            if matches!(
                self.edge_vertices(eh),
                Some((va, vb)) if (va == v2 && vb == v3) || (va == v3 && vb == v2)
            ) {
                return Err(TopologyError::InvalidOperation(
                    "Cannot flip edge: an edge connecting opposite vertices already exists".into(),
                ));
            }
        }

        // Geometric convexity check: quad (v0, v3, v1, v2) must be strictly convex
        let p0 = self.vertex(v0).unwrap().position;
        let p1 = self.vertex(v1).unwrap().position;
        let p2 = self.vertex(v2).unwrap().position;
        let p3 = self.vertex(v3).unwrap().position;

        let n_old0 = (p1 - p0).cross(p2 - p0);
        let n_old1 = (p0 - p1).cross(p3 - p1);

        if n_old0.length_squared() < 1e-12 || n_old1.length_squared() < 1e-12 {
            return Err(TopologyError::InvalidOperation(
                "Cannot flip edge: incident triangles are degenerate or collinear".into(),
            ));
        }

        // New triangles with flipped diagonal (v2, v3):
        // Triangle f0 (v2, v3, v1):
        let n_new0 = (p3 - p2).cross(p1 - p2);
        // Triangle f1 (v3, v2, v0):
        let n_new1 = (p2 - p3).cross(p0 - p3);

        if n_new0.length_squared() < 1e-12 || n_new1.length_squared() < 1e-12 {
            return Err(TopologyError::InvalidOperation(
                "Cannot flip edge: resulting triangles would be degenerate or collinear".into(),
            ));
        }

        if n_new0.dot(n_old0) <= 0.0 || n_new1.dot(n_old1) <= 0.0 || n_new0.dot(n_new1) <= 0.0 {
            return Err(TopologyError::InvalidOperation(
                "Cannot flip edge: quad is non-convex or resulting triangles would invert/fold"
                    .into(),
            ));
        }

        // Rewire origins: edge now connects v2 and v3
        self.half_edge_mut(h0).unwrap().origin = v2;
        self.half_edge_mut(h1).unwrap().origin = v3;

        // Preserve wedge/corner attributes for new diagonal
        let uv_v2_f0 = self.half_edge(h0_prev).unwrap().uv;
        let norm_v2_f0 = self.half_edge(h0_prev).unwrap().normal;
        self.half_edge_mut(h0).unwrap().uv = uv_v2_f0;
        self.half_edge_mut(h0).unwrap().normal = norm_v2_f0;

        let uv_v3_f1 = self.half_edge(h1_prev).unwrap().uv;
        let norm_v3_f1 = self.half_edge(h1_prev).unwrap().normal;
        self.half_edge_mut(h1).unwrap().uv = uv_v3_f1;
        self.half_edge_mut(h1).unwrap().normal = norm_v3_f1;

        // Triangle f0: v2 -> v3 -> v1 -> v2
        // h0 (v2 -> v3) -> h1_prev (v3 -> v1) -> h0_next (v1 -> v2) -> h0
        self.half_edge_mut(h0).unwrap().next = h1_prev;
        self.half_edge_mut(h0).unwrap().prev = h0_next;

        self.half_edge_mut(h1_prev).unwrap().next = h0_next;
        self.half_edge_mut(h1_prev).unwrap().prev = h0;
        self.half_edge_mut(h1_prev).unwrap().face = Some(f0);

        self.half_edge_mut(h0_next).unwrap().next = h0;
        self.half_edge_mut(h0_next).unwrap().prev = h1_prev;
        self.half_edge_mut(h0_next).unwrap().face = Some(f0);

        // Triangle f1: v3 -> v2 -> v0 -> v3
        // h1 (v3 -> v2) -> h0_prev (v2 -> v0) -> h1_next (v0 -> v3) -> h1
        self.half_edge_mut(h1).unwrap().next = h0_prev;
        self.half_edge_mut(h1).unwrap().prev = h1_next;

        self.half_edge_mut(h0_prev).unwrap().next = h1_next;
        self.half_edge_mut(h0_prev).unwrap().prev = h1;
        self.half_edge_mut(h0_prev).unwrap().face = Some(f1);

        self.half_edge_mut(h1_next).unwrap().next = h1;
        self.half_edge_mut(h1_next).unwrap().prev = h0_prev;
        self.half_edge_mut(h1_next).unwrap().face = Some(f1);

        // Representative half-edges
        self.face_mut(f0).unwrap().half_edge = h0;
        self.face_mut(f1).unwrap().half_edge = h1;

        // Outgoing half-edges for vertices (prefer boundary half-edge if on boundary)
        for &vh in &[v0, v1, v2, v3] {
            let mut bnd = None;
            let mut any_out = None;
            for (idx, he) in self.half_edges.iter().enumerate() {
                if !he.deleted && he.origin == vh {
                    let he_h = HalfEdgeHandle::new(idx as u32, he.generation);
                    if he.is_boundary() {
                        bnd = Some(he_h);
                        break;
                    } else if any_out.is_none() {
                        any_out = Some(he_h);
                    }
                }
            }
            self.vertex_mut(vh).unwrap().half_edge = bnd.or(any_out);
        }

        // Recalculate face normals
        let norm0 = self.face_normal(f0);
        let norm1 = self.face_normal(f1);
        self.face_mut(f0).unwrap().normal = Some(norm0);
        self.face_mut(f1).unwrap().normal = Some(norm1);

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(FlipEdgeResult {
            edge,
            faces: (f0, f1),
            vertices: (v0, v1, v2, v3),
            new_diagonal: (v2, v3),
            half_edges: (h0, h1),
        })
    }

    /// Connects two non-adjacent vertices of face `face` with a new internal edge,
    /// splitting it into two faces (`face` and a newly created face).
    ///
    /// Preserves UV seams, corner attributes, and material ID.
    /// Transactionally rolls back if an invariant is violated.
    pub fn split_face(
        &mut self,
        face: FaceHandle,
        vertex_a: VertexHandle,
        vertex_b: VertexHandle,
    ) -> Result<SplitFaceResult, TopologyError> {
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }
        if !self.is_valid_vertex(vertex_a) {
            return Err(TopologyError::InvalidVertex(vertex_a));
        }
        if !self.is_valid_vertex(vertex_b) {
            return Err(TopologyError::InvalidVertex(vertex_b));
        }
        if vertex_a == vertex_b {
            return Err(TopologyError::InvalidOperation(
                "Cannot split face using the same vertex twice".into(),
            ));
        }

        // Validate face loop cycle is closed and valid
        let f_hes = self.face_loop_safe(face)?;

        let pos_a = f_hes
            .iter()
            .position(|&h| self.half_edge(h).unwrap().origin == vertex_a)
            .ok_or_else(|| {
                TopologyError::InvalidOperation(format!(
                    "Vertex {vertex_a} does not belong to {face}"
                ))
            })?;
        let pos_b = f_hes
            .iter()
            .position(|&h| self.half_edge(h).unwrap().origin == vertex_b)
            .ok_or_else(|| {
                TopologyError::InvalidOperation(format!(
                    "Vertex {vertex_b} does not belong to {face}"
                ))
            })?;

        let k = f_hes.len();
        if (pos_a + 1) % k == pos_b || (pos_b + 1) % k == pos_a {
            return Err(TopologyError::InvalidOperation(
                "Cannot split face between adjacent vertices (edge already exists)".into(),
            ));
        }

        // Prevent duplicate edge if an edge already connects vertex_a and vertex_b
        for &eh in &self.incident_edges(vertex_a) {
            if matches!(
                self.edge_vertices(eh),
                Some((va, vb)) if (va == vertex_a && vb == vertex_b) || (va == vertex_b && vb == vertex_a)
            ) {
                return Err(TopologyError::InvalidOperation(format!(
                    "Cannot split face: an edge connecting {vertex_a} and {vertex_b} already exists"
                )));
            }
        }

        // Minimum sides check: both resulting cycles must have >= 3 vertices
        let count_ab = ((pos_b + k - pos_a) % k) + 1;
        let count_ba = ((pos_a + k - pos_b) % k) + 1;
        if count_ab < 3 || count_ba < 3 {
            return Err(TopologyError::InvalidOperation(
                "Split face requires both resulting faces to have at least 3 vertices".into(),
            ));
        }

        let backup = self.clone();

        let h_a = f_hes[pos_a]; // starts at vertex_a
        let h_b = f_hes[pos_b]; // starts at vertex_b

        let h_a_prev = self.half_edge(h_a).unwrap().prev;
        let h_b_prev = self.half_edge(h_b).unwrap().prev;

        // Allocate new edge and new face
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let e_new = self.add_edge(dummy_he);
        let f_new = self.add_face(dummy_he);

        // Inherit material_id from original face to new face
        let mat_id = self.face(face).unwrap().material_id;
        self.face_mut(f_new).unwrap().material_id = mat_id;

        // h_ab runs vertex_a -> vertex_b (belongs to f_new)
        let h_ab = self.add_half_edge(vertex_a, e_new, Some(f_new));
        // h_ba runs vertex_b -> vertex_a (belongs to original face)
        let h_ba = self.add_half_edge(vertex_b, e_new, Some(face));

        self.edge_mut(e_new).unwrap().half_edge = h_ab;

        self.half_edge_mut(h_ab).unwrap().twin = h_ba;
        self.half_edge_mut(h_ba).unwrap().twin = h_ab;

        // Transfer corner/wedge attributes (UVs and normals):
        // h_ab originates at vertex_a in f_new: inherits from h_a
        let uv_a = self.half_edge(h_a).unwrap().uv;
        let norm_a = self.half_edge(h_a).unwrap().normal;
        self.half_edge_mut(h_ab).unwrap().uv = uv_a;
        self.half_edge_mut(h_ab).unwrap().normal = norm_a;

        // h_ba originates at vertex_b in original face: inherits from h_b
        let uv_b = self.half_edge(h_b).unwrap().uv;
        let norm_b = self.half_edge(h_b).unwrap().normal;
        self.half_edge_mut(h_ba).unwrap().uv = uv_b;
        self.half_edge_mut(h_ba).unwrap().normal = norm_b;

        // Rewire face: h_a .. h_b_prev -> h_ba -> h_a
        self.half_edge_mut(h_b_prev).unwrap().next = h_ba;
        self.half_edge_mut(h_ba).unwrap().prev = h_b_prev;
        self.half_edge_mut(h_ba).unwrap().next = h_a;
        self.half_edge_mut(h_a).unwrap().prev = h_ba;
        self.face_mut(face).unwrap().half_edge = h_a;

        // Rewire f_new: h_b .. h_a_prev -> h_ab -> h_b
        self.half_edge_mut(h_a_prev).unwrap().next = h_ab;
        self.half_edge_mut(h_ab).unwrap().prev = h_a_prev;
        self.half_edge_mut(h_ab).unwrap().next = h_b;
        self.half_edge_mut(h_b).unwrap().prev = h_ab;
        self.face_mut(f_new).unwrap().half_edge = h_b;

        // Update face pointers for half-edges in f_new cycle
        let mut curr = h_b;
        while curr != h_ab {
            self.half_edge_mut(curr).unwrap().face = Some(f_new);
            curr = self.half_edge(curr).unwrap().next;
        }

        // Update face pointers for half-edges in original face cycle
        let mut curr = h_a;
        while curr != h_ba {
            self.half_edge_mut(curr).unwrap().face = Some(face);
            curr = self.half_edge(curr).unwrap().next;
        }

        // Refresh outgoing half-edges for vertex_a and vertex_b
        self.refresh_vertex_outgoing_half_edge(vertex_a);
        self.refresh_vertex_outgoing_half_edge(vertex_b);

        // Recalculate face normals
        let norm_f = self.face_normal(face);
        let norm_f_new = self.face_normal(f_new);
        self.face_mut(face).unwrap().normal = Some(norm_f);
        self.face_mut(f_new).unwrap().normal = Some(norm_f_new);

        // Geometric verification: check for non-zero surface area
        let area_f = self.face_area(face);
        let area_f_new = self.face_area(f_new);
        if !area_f.is_finite() || area_f <= 1e-8 || !area_f_new.is_finite() || area_f_new <= 1e-8 {
            *self = backup;
            return Err(TopologyError::InvalidOperation(
                "Cannot split face: resulting face would be degenerate or have zero area".into(),
            ));
        }

        // Orientation check: resulting faces must not be inverted relative to the original face
        let norm_orig = backup.face_normal(face);
        if norm_orig.length_squared() >= 1e-12
            && (norm_f.dot(norm_orig) <= 0.0 || norm_f_new.dot(norm_orig) <= 0.0)
        {
            *self = backup;
            return Err(TopologyError::InvalidOperation(
                "Cannot split face: resulting face normal is inverted relative to original face"
                    .into(),
            ));
        }

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        let face1_vertices = self.face_vertices(face);
        let face2_vertices = self.face_vertices(f_new);

        Ok(SplitFaceResult {
            original_face: face,
            new_face: f_new,
            new_edge: e_new,
            new_half_edges: (h_ab, h_ba),
            face1_vertices,
            face2_vertices,
        })
    }

    /// Dissolves an internal edge between two faces, merging them into a single polygon face.
    ///
    /// The first incident face `f0` is retained as the surviving face, and its `material_id`
    /// is kept as the deterministic material policy. The second incident face `f1`, the edge,
    /// and its two half-edges are deleted and their slots freed.
    ///
    /// Transactionally rolls back if an invariant is violated.
    pub fn dissolve_edge(&mut self, edge: EdgeHandle) -> Result<MergeFacesResult, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }

        if self.is_edge_boundary(edge) {
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve boundary edge".into(),
            ));
        }

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        if !self.is_valid_half_edge(h0) || !self.is_valid_half_edge(h1) {
            return Err(TopologyError::InvalidOperation(
                "Edge references invalid half-edges".into(),
            ));
        }

        let f0 = self.half_edge(h0).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation("Cannot dissolve boundary edge".into())
        })?;
        let f1 = self.half_edge(h1).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation("Cannot dissolve boundary edge".into())
        })?;

        if !self.is_valid_face(f0) {
            return Err(TopologyError::InvalidFace(f0));
        }
        if !self.is_valid_face(f1) {
            return Err(TopologyError::InvalidFace(f1));
        }

        if f0 == f1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot merge edge belonging to the same face on both sides".into(),
            ));
        }

        // Validate both face cycles are closed and valid
        let _f0_hes = self.face_loop_safe(f0)?;
        let _f1_hes = self.face_loop_safe(f1)?;

        // Check if faces share multiple edges
        let f0_edges = self.face_edges(f0);
        let f1_edges = self.face_edges(f1);
        let shared_edge_count = f0_edges.iter().filter(|e| f1_edges.contains(e)).count();
        if shared_edge_count > 1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve edge: incident faces share multiple edges".into(),
            ));
        }

        let v0 = self.half_edge(h0).unwrap().origin;
        let v1 = self.half_edge(h1).unwrap().origin;

        let h0_prev = self.half_edge(h0).unwrap().prev;
        let h0_next = self.half_edge(h0).unwrap().next;
        let h1_prev = self.half_edge(h1).unwrap().prev;
        let h1_next = self.half_edge(h1).unwrap().next;

        // Build expected boundary vertices in merged cycle
        let mut boundary_vertices = Vec::new();
        let mut curr = h0_next;
        while curr != h0 {
            boundary_vertices.push(self.half_edge(curr).unwrap().origin);
            curr = self.half_edge(curr).unwrap().next;
        }
        let mut curr = h1_next;
        while curr != h1 {
            boundary_vertices.push(self.half_edge(curr).unwrap().origin);
            curr = self.half_edge(curr).unwrap().next;
        }

        // Reject if fewer than 3 vertices
        if boundary_vertices.len() < 3 {
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve edge: merged face would have fewer than 3 vertices".into(),
            ));
        }

        // Check for duplicate consecutive vertices
        let n = boundary_vertices.len();
        for i in 0..n {
            if boundary_vertices[i] == boundary_vertices[(i + 1) % n] {
                return Err(TopologyError::InvalidOperation(
                    "Cannot dissolve edge: merged face boundary contains duplicate consecutive vertices".into(),
                ));
            }
        }

        // Check for duplicate vertices in simple polygon cycle
        let mut seen = HashSet::new();
        for &v in &boundary_vertices {
            if !seen.insert(v) {
                return Err(TopologyError::InvalidOperation(
                    "Cannot dissolve edge: merged face boundary would contain duplicate vertices"
                        .into(),
                ));
            }
        }

        // Geometric checks on orientation before merge:
        let norm0 = self.face_normal(f0);
        let norm1 = self.face_normal(f1);
        if norm0.length_squared() >= 1e-12
            && norm1.length_squared() >= 1e-12
            && norm0.dot(norm1) <= 0.0
        {
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve edge: incident faces fold backwards (dot product <= 0)".into(),
            ));
        }

        let backup = self.clone();

        // Splice cycles together bypassing h0 and h1
        self.half_edge_mut(h0_prev).unwrap().next = h1_next;
        self.half_edge_mut(h1_next).unwrap().prev = h0_prev;

        self.half_edge_mut(h1_prev).unwrap().next = h0_next;
        self.half_edge_mut(h0_next).unwrap().prev = h1_prev;

        // Reassign all half-edges in the merged cycle to f0
        let mut curr = h0_next;
        loop {
            self.half_edge_mut(curr).unwrap().face = Some(f0);
            curr = self.half_edge(curr).unwrap().next;
            if curr == h0_next {
                break;
            }
        }

        // Set f0's representative half-edge
        self.face_mut(f0).unwrap().half_edge = h0_next;

        // Delete h0, h1, edge, and f1
        self.free_half_edge(h0)?;
        self.free_half_edge(h1)?;
        self.free_edge(edge)?;
        self.free_face(f1)?;

        // Refresh vertex outgoing pointers for v0, v1, and all boundary vertices
        self.refresh_vertex_outgoing_half_edge(v0);
        self.refresh_vertex_outgoing_half_edge(v1);
        for &v in &boundary_vertices {
            self.refresh_vertex_outgoing_half_edge(v);
        }

        // Recalculate face normal
        let norm = self.face_normal(f0);
        self.face_mut(f0).unwrap().normal = Some(norm);

        // Geometric validation on merged face
        let area_merged = self.face_area(f0);
        if !area_merged.is_finite() || area_merged <= 1e-8 {
            *self = backup;
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve edge: merged face would be degenerate or have zero area".into(),
            ));
        }

        if norm0.length_squared() >= 1e-12 && norm.dot(norm0) <= 0.0 {
            *self = backup;
            return Err(TopologyError::InvalidOperation(
                "Cannot dissolve edge: merged face normal inverted relative to original face"
                    .into(),
            ));
        }

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(MergeFacesResult {
            surviving_face: f0,
            removed_face: f1,
            dissolved_edge: edge,
            boundary_vertices,
        })
    }

    /// Alias for [`dissolve_edge`](Self::dissolve_edge). Merges the two adjacent faces sharing `edge`.
    #[inline]
    pub fn merge_faces(&mut self, edge: EdgeHandle) -> Result<MergeFacesResult, TopologyError> {
        self.dissolve_edge(edge)
    }

    /// Inserts a new vertex inside a convex polygonal face and connects it to every boundary
    /// vertex, producing a triangle fan.
    ///
    /// Preserves the original face handle for one of the triangles and creates N - 1 new faces.
    /// Transactionally rolls back if an invariant is violated.
    pub fn poke_face(
        &mut self,
        face: FaceHandle,
        position: Vec3,
    ) -> Result<PokeFaceResult, TopologyError> {
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }

        if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
            return Err(TopologyError::InvalidOperation(
                "Poke face position contains non-finite coordinates".into(),
            ));
        }

        let f_hes = self.face_loop_safe(face)?;
        let n = f_hes.len();
        if n < 3 {
            return Err(TopologyError::InvalidOperation(
                "Face must have at least 3 vertices to poke".into(),
            ));
        }

        let bnd_vertices: Vec<VertexHandle> = f_hes
            .iter()
            .map(|&h| self.half_edge(h).unwrap().origin)
            .collect();

        let bnd_positions: Vec<Vec3> = bnd_vertices
            .iter()
            .map(|&v| self.vertex(v).unwrap().position)
            .collect();

        let norm_face = self.face_normal(face);
        if norm_face.length_squared() < 1e-12 {
            return Err(TopologyError::InvalidOperation(
                "Cannot poke degenerate face with zero area or undefined normal".into(),
            ));
        }

        // Planarity check: boundary vertices must lie close to the face plane
        let center = self.face_center(face);
        for (i, &p) in bnd_positions.iter().enumerate() {
            let dist_to_plane = ((p - center).dot(norm_face)).abs();
            if dist_to_plane > 1e-3 {
                return Err(TopologyError::InvalidOperation(format!(
                    "Cannot poke non-planar face: vertex {i} distance to plane is {dist_to_plane}"
                )));
            }
        }

        // Position must also lie in the face plane
        let pos_dist_to_plane = ((position - center).dot(norm_face)).abs();
        if pos_dist_to_plane > 1e-3 {
            return Err(TopologyError::InvalidOperation(format!(
                "Cannot poke face: position is not coplanar with face (distance {pos_dist_to_plane})"
            )));
        }

        // Convexity check: all corners must turn counter-clockwise
        for i in 0..n {
            let p_curr = bnd_positions[i];
            let p_next = bnd_positions[(i + 1) % n];
            let p_next2 = bnd_positions[(i + 2) % n];
            let edge1 = p_next - p_curr;
            let edge2 = p_next2 - p_next;
            let turn = edge1.cross(edge2).dot(norm_face);
            if turn <= 1e-6 {
                return Err(TopologyError::InvalidOperation(
                    "Cannot poke face: polygon is not strictly convex".into(),
                ));
            }
        }

        // Inside polygon and non-degenerate triangle check
        for i in 0..n {
            let p_curr = bnd_positions[i];
            let p_next = bnd_positions[(i + 1) % n];
            let edge = p_next - p_curr;
            let to_pos = position - p_curr;
            let cross = edge.cross(to_pos);
            let signed_area_2x = cross.dot(norm_face);
            if signed_area_2x <= 1e-5 {
                return Err(TopologyError::InvalidOperation(
                    "Cannot poke face: position lies outside or on the boundary of the face".into(),
                ));
            }
            let tri_area = 0.5 * cross.length();
            if tri_area <= 1e-6 {
                return Err(TopologyError::InvalidOperation(
                    "Cannot poke face: generated triangle would be degenerate or have zero area"
                        .into(),
                ));
            }
        }

        let backup = self.clone();

        // 1. Create new center vertex
        let vc = self.add_vertex(position);

        // Interpolate vertex attributes for vc
        self.vertex_mut(vc).unwrap().normal = Some(norm_face);

        let mut uv_sum = forge_math::Vec2::ZERO;
        let mut uv_count = 0;
        for &v in &bnd_vertices {
            if let Some(uv) = self.vertex(v).unwrap().uv {
                uv_sum += uv;
                uv_count += 1;
            }
        }
        if uv_count == n {
            self.vertex_mut(vc).unwrap().uv = Some(uv_sum / (n as f32));
        }

        let mut col_sum = [0.0f32; 4];
        let mut col_count = 0;
        for &v in &bnd_vertices {
            if let Some(col) = self.vertex(v).unwrap().color {
                for c in 0..4 {
                    col_sum[c] += col[c];
                }
                col_count += 1;
            }
        }
        if col_count == n {
            for val in &mut col_sum {
                *val /= n as f32;
            }
            self.vertex_mut(vc).unwrap().color = Some(col_sum);
        }

        // 2. Prepare face handles: face 0 is the original face, faces 1..N-1 are newly added
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let orig_mat_id = self.face(face).unwrap().material_id;

        let mut faces = Vec::with_capacity(n);
        faces.push(face);
        for _ in 1..n {
            let f_new = self.add_face(dummy_he);
            self.face_mut(f_new).unwrap().material_id = orig_mat_id;
            faces.push(f_new);
        }

        // 3. Create N radial edges
        let e_rad: Vec<EdgeHandle> = (0..n).map(|_| self.add_edge(dummy_he)).collect();

        // 4. Create radial half-edges:
        // r_in[i] originates at bnd_vertices[i], directed to vc, belongs to face (i + n - 1) % n
        // r_out[i] originates at vc, directed to bnd_vertices[i], belongs to face i
        let r_in: Vec<HalfEdgeHandle> = (0..n)
            .map(|i| self.add_half_edge(bnd_vertices[i], e_rad[i], Some(faces[(i + n - 1) % n])))
            .collect();

        let r_out: Vec<HalfEdgeHandle> = (0..n)
            .map(|i| self.add_half_edge(vc, e_rad[i], Some(faces[i])))
            .collect();

        // Twin setup and edge half-edge assignment
        for i in 0..n {
            self.half_edge_mut(r_in[i]).unwrap().twin = r_out[i];
            self.half_edge_mut(r_out[i]).unwrap().twin = r_in[i];
            self.edge_mut(e_rad[i]).unwrap().half_edge = r_out[i];
        }

        // 5. Transfer corner attributes to new radial half-edges
        let vc_uv = self.vertex(vc).unwrap().uv;
        for i in 0..n {
            // In face i:
            // h_i originates at bnd_vertices[i] (already has corner attributes)
            // r_in[(i + 1) % n] originates at bnd_vertices[(i + 1) % n] in face i:
            let h_next_orig = f_hes[(i + 1) % n];
            let uv_next = self.half_edge(h_next_orig).unwrap().uv;
            let norm_next = self.half_edge(h_next_orig).unwrap().normal;
            self.half_edge_mut(r_in[(i + 1) % n]).unwrap().uv = uv_next;
            self.half_edge_mut(r_in[(i + 1) % n]).unwrap().normal = norm_next;

            // r_out[i] originates at vc in face i:
            self.half_edge_mut(r_out[i]).unwrap().uv = vc_uv;
            self.half_edge_mut(r_out[i]).unwrap().normal = Some(norm_face);
        }

        // 6. Connect next/prev links for each triangle i:
        // Triangle i cycle: h_i -> r_in[(i + 1) % n] -> r_out[i] -> h_i
        for i in 0..n {
            let h_i = f_hes[i];
            let r_in_next = r_in[(i + 1) % n];
            let r_out_curr = r_out[i];
            let f_curr = faces[i];

            self.half_edge_mut(h_i).unwrap().next = r_in_next;
            self.half_edge_mut(r_in_next).unwrap().prev = h_i;

            self.half_edge_mut(r_in_next).unwrap().next = r_out_curr;
            self.half_edge_mut(r_out_curr).unwrap().prev = r_in_next;

            self.half_edge_mut(r_out_curr).unwrap().next = h_i;
            self.half_edge_mut(h_i).unwrap().prev = r_out_curr;

            self.half_edge_mut(h_i).unwrap().face = Some(f_curr);
            self.half_edge_mut(r_in_next).unwrap().face = Some(f_curr);
            self.half_edge_mut(r_out_curr).unwrap().face = Some(f_curr);

            self.face_mut(f_curr).unwrap().half_edge = h_i;
        }

        // 7. Refresh outgoing half-edges for vc and boundary vertices
        self.vertex_mut(vc).unwrap().half_edge = Some(r_out[0]);
        for &v in &bnd_vertices {
            self.refresh_vertex_outgoing_half_edge(v);
        }

        // 8. Recalculate face normals for all fan triangles
        for &f in &faces {
            let norm = self.face_normal(f);
            self.face_mut(f).unwrap().normal = Some(norm);
        }

        // 9. Invariant validation
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        let mut all_radial_hes = Vec::with_capacity(2 * n);
        for i in 0..n {
            all_radial_hes.push(r_in[i]);
            all_radial_hes.push(r_out[i]);
        }

        Ok(PokeFaceResult {
            original_face: face,
            new_vertex: vc,
            new_faces: faces[1..].to_vec(),
            new_edges: e_rad,
            new_half_edges: all_radial_hes,
        })
    }

    /// Extrudes face `face` along its normal by `distance`.
    ///
    /// Creates a translated top face and side quad faces connecting each original boundary edge
    /// to its corresponding translated edge, while preserving `face` as the bottom face.
    /// Transactionally rolls back if an invariant is violated.
    pub fn extrude_face(
        &mut self,
        face: FaceHandle,
        distance: f32,
    ) -> Result<ExtrudeFaceResult, TopologyError> {
        // 1. Pre-validation
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }

        if !distance.is_finite() {
            return Err(TopologyError::InvalidOperation(
                "Extrusion distance must be finite".into(),
            ));
        }

        if distance.abs() < 1e-6 {
            return Err(TopologyError::InvalidOperation(
                "Extrusion distance cannot be zero or near-zero".into(),
            ));
        }

        let h_cycle = self.face_loop_safe(face)?;
        let n = h_cycle.len();
        if n < 3 {
            return Err(TopologyError::Degenerate(format!(
                "Face {face} has fewer than 3 vertices ({n})"
            )));
        }

        let norm_face = self.face_normal(face);
        if norm_face.length_squared() < 1e-12 {
            return Err(TopologyError::Degenerate(
                "Cannot extrude degenerate face with zero area or undefined normal".into(),
            ));
        }

        let area = self.face_area(face);
        if area <= 1e-8 {
            return Err(TopologyError::Degenerate(format!(
                "Cannot extrude degenerate face {face} with near-zero area ({area})"
            )));
        }

        // Collect original vertices and check that boundary edges are not shared with adjacent faces
        let mut orig_v = Vec::with_capacity(n);
        let mut orig_pos = Vec::with_capacity(n);
        let mut orig_uvs = Vec::with_capacity(n);
        let mut orig_norms = Vec::with_capacity(n);

        for &h in &h_cycle {
            let he = self.half_edge(h).unwrap();
            let v = he.origin;
            orig_v.push(v);
            let v_data = self.vertex(v).unwrap();
            orig_pos.push(v_data.position);
            orig_uvs.push(he.uv.or(v_data.uv));
            orig_norms.push(he.normal.or(v_data.normal).unwrap_or(norm_face));

            let twin_h = he.twin;
            if self.half_edge(twin_h).unwrap().face.is_some() {
                return Err(TopologyError::NonManifold(format!(
                    "Face {face} cannot be extruded while preserving bottom face: edge is shared with an adjacent face"
                )));
            }
        }

        // Compute translated top positions and verify coordinate finiteness
        let offset = norm_face * distance;
        let mut top_pos = Vec::with_capacity(n);
        for &p in &orig_pos {
            let p_new = p + offset;
            if !p_new.x.is_finite() || !p_new.y.is_finite() || !p_new.z.is_finite() {
                return Err(TopologyError::Degenerate(
                    "Generated vertex positions contain non-finite coordinates".into(),
                ));
            }
            top_pos.push(p_new);
        }

        // Snapshot state for atomic rollback
        let backup = self.clone();

        // 2. Allocate new vertices
        let mut top_v = Vec::with_capacity(n);
        for i in 0..n {
            let (uv, color, normal) = {
                let ov = self.vertex(orig_v[i]).unwrap();
                (ov.uv, ov.color, ov.normal)
            };
            let v_new = self.add_vertex(top_pos[i]);
            let v_mut = self.vertex_mut(v_new).unwrap();
            v_mut.uv = uv;
            v_mut.color = color;
            v_mut.normal = normal.or(Some(norm_face));
            top_v.push(v_new);
        }

        // 3. Allocate new faces: 1 top face + N side faces
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let orig_mat_id = self.face(face).unwrap().material_id;

        let top_face = self.add_face(dummy_he);
        self.face_mut(top_face).unwrap().material_id = orig_mat_id;

        let mut side_faces = Vec::with_capacity(n);
        for _ in 0..n {
            let sf = self.add_face(dummy_he);
            self.face_mut(sf).unwrap().material_id = orig_mat_id;
            side_faces.push(sf);
        }

        // 4. Allocate new edges: N top edges + N pillar edges
        let mut top_edges = Vec::with_capacity(n);
        for _ in 0..n {
            top_edges.push(self.add_edge(dummy_he));
        }

        let mut pillar_edges = Vec::with_capacity(n);
        for _ in 0..n {
            pillar_edges.push(self.add_edge(dummy_he));
        }

        // 5. Allocate new half-edges
        // Top edges: 2*N half-edges (h_top and t_top)
        let mut h_top = Vec::with_capacity(n);
        let mut t_top = Vec::with_capacity(n);
        for i in 0..n {
            let next_i = (i + 1) % n;
            let ht = self.add_half_edge(top_v[i], top_edges[i], Some(top_face));
            let tt = self.add_half_edge(top_v[next_i], top_edges[i], Some(side_faces[i]));
            h_top.push(ht);
            t_top.push(tt);
        }

        // Pillar edges: 2*N half-edges (p_up and p_down)
        let mut p_up = Vec::with_capacity(n);
        let mut p_down = Vec::with_capacity(n);
        for i in 0..n {
            let prev_side_i = (i + n - 1) % n;
            let pu = self.add_half_edge(orig_v[i], pillar_edges[i], Some(side_faces[prev_side_i]));
            let pd = self.add_half_edge(top_v[i], pillar_edges[i], Some(side_faces[i]));
            p_up.push(pu);
            p_down.push(pd);
        }

        // 6. Connect twins
        for i in 0..n {
            // Top edges
            self.half_edge_mut(h_top[i]).unwrap().twin = t_top[i];
            self.half_edge_mut(t_top[i]).unwrap().twin = h_top[i];
            self.edge_mut(top_edges[i]).unwrap().half_edge = h_top[i];

            // Pillar edges
            self.half_edge_mut(p_up[i]).unwrap().twin = p_down[i];
            self.half_edge_mut(p_down[i]).unwrap().twin = p_up[i];
            self.edge_mut(pillar_edges[i]).unwrap().half_edge = p_up[i];
        }

        // 7. Wire Top Face cycle
        for i in 0..n {
            let next_i = (i + 1) % n;
            let prev_i = (i + n - 1) % n;
            let ht = self.half_edge_mut(h_top[i]).unwrap();
            ht.next = h_top[next_i];
            ht.prev = h_top[prev_i];
            ht.uv = orig_uvs[i];
            ht.normal = Some(orig_norms[i]);
        }
        self.face_mut(top_face).unwrap().half_edge = h_top[0];

        // 8. Reconfigure bottom edges:
        // Existing h_cycle[i] had origin orig_v[i] -> orig_v[(i+1)%n].
        // Its twin orig_twins[i] had origin orig_v[(i+1)%n] -> orig_v[i].
        //
        // Bottom face (face) will use orig_twins[i] so that its cycle is reversed (outward normal -norm_face).
        // Side face i will use h_cycle[i] (orig_v[i] -> orig_v[(i+1)%n]).
        let mut orig_twins = Vec::with_capacity(n);
        for &h in &h_cycle {
            let tw = self.half_edge(h).unwrap().twin;
            orig_twins.push(tw);
        }

        // Wire Bottom Face cycle
        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let next_i = (i + 1) % n;
            let tw = self.half_edge_mut(orig_twins[i]).unwrap();
            tw.face = Some(face);
            tw.next = orig_twins[prev_i];
            tw.prev = orig_twins[next_i];
            // Corner attributes for bottom face at origin of tw (orig_v[(i+1)%n])
            let corner_v_idx = (i + 1) % n;
            tw.uv = orig_uvs[corner_v_idx];
            tw.normal = Some(-orig_norms[corner_v_idx]);
        }
        self.face_mut(face).unwrap().half_edge = orig_twins[0];

        // 9. Wire Side Face i cycles:
        // Cycle: h_cycle[i] -> p_up[(i+1)%n] -> t_top[i] -> p_down[i] -> h_cycle[i]
        for i in 0..n {
            let next_i = (i + 1) % n;
            let sf = side_faces[i];

            let h_bot = h_cycle[i];
            let p_right = p_up[next_i];
            let h_t = t_top[i];
            let p_left = p_down[i];

            // Set face references
            self.half_edge_mut(h_bot).unwrap().face = Some(sf);
            self.half_edge_mut(p_right).unwrap().face = Some(sf);
            self.half_edge_mut(h_t).unwrap().face = Some(sf);
            self.half_edge_mut(p_left).unwrap().face = Some(sf);

            // Set cycle links
            self.half_edge_mut(h_bot).unwrap().next = p_right;
            self.half_edge_mut(h_bot).unwrap().prev = p_left;

            self.half_edge_mut(p_right).unwrap().next = h_t;
            self.half_edge_mut(p_right).unwrap().prev = h_bot;

            self.half_edge_mut(h_t).unwrap().next = p_left;
            self.half_edge_mut(h_t).unwrap().prev = p_right;

            self.half_edge_mut(p_left).unwrap().next = h_bot;
            self.half_edge_mut(p_left).unwrap().prev = h_t;

            self.face_mut(sf).unwrap().half_edge = h_bot;
        }

        // Set side face normals and corner attributes
        for i in 0..n {
            let next_i = (i + 1) % n;
            let sf = side_faces[i];
            let sn = self.face_normal(sf);
            self.face_mut(sf).unwrap().normal = Some(sn);

            let h_bot = h_cycle[i];
            let p_right = p_up[next_i];
            let h_t = t_top[i];
            let p_left = p_down[i];

            self.half_edge_mut(h_bot).unwrap().uv = orig_uvs[i];
            self.half_edge_mut(h_bot).unwrap().normal = Some(sn);

            self.half_edge_mut(p_right).unwrap().uv = orig_uvs[next_i];
            self.half_edge_mut(p_right).unwrap().normal = Some(sn);

            self.half_edge_mut(h_t).unwrap().uv = orig_uvs[next_i];
            self.half_edge_mut(h_t).unwrap().normal = Some(sn);

            self.half_edge_mut(p_left).unwrap().uv = orig_uvs[i];
            self.half_edge_mut(p_left).unwrap().normal = Some(sn);
        }

        // 10. Update vertex outgoing half-edges
        for i in 0..n {
            self.vertex_mut(top_v[i]).unwrap().half_edge = Some(h_top[i]);
            self.vertex_mut(orig_v[i]).unwrap().half_edge = Some(h_cycle[i]);
        }

        // 11. Recalculate face normals for top and bottom faces
        let norm_top = self.face_normal(top_face);
        self.face_mut(top_face).unwrap().normal = Some(norm_top);

        let norm_bot = self.face_normal(face);
        self.face_mut(face).unwrap().normal = Some(norm_bot);

        // 12. Verify topological and geometric invariants
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        // 13. Construct result
        let mut new_faces = Vec::with_capacity(1 + n);
        new_faces.push(top_face);
        new_faces.extend_from_slice(&side_faces);

        let mut new_edges = Vec::with_capacity(2 * n);
        new_edges.extend_from_slice(&top_edges);
        new_edges.extend_from_slice(&pillar_edges);

        let mut new_half_edges = Vec::with_capacity(4 * n);
        new_half_edges.extend_from_slice(&h_top);
        new_half_edges.extend_from_slice(&t_top);
        new_half_edges.extend_from_slice(&p_up);
        new_half_edges.extend_from_slice(&p_down);

        Ok(ExtrudeFaceResult {
            original_face: face,
            top_face,
            new_vertices: top_v,
            new_edges,
            new_faces,
            new_half_edges,
        })
    }

    /// Creates an inset region inside a single convex planar face by `offset`.
    ///
    /// Generates a smaller inner face and side quad faces connecting its boundary to the original
    /// face boundary, preserving `face` as the first side face (side face 0) in the surrounding region.
    /// Transactionally rolls back if an invariant is violated.
    pub fn inset_face(
        &mut self,
        face: FaceHandle,
        offset: f32,
    ) -> Result<InsetFaceResult, TopologyError> {
        // 1. Pre-validation
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }

        if !offset.is_finite() || offset <= 0.0 {
            return Err(TopologyError::InvalidOperation(
                "Inset offset must be finite and strictly positive".into(),
            ));
        }

        let h_cycle = self.face_loop_safe(face)?;
        let n = h_cycle.len();
        if n < 3 {
            return Err(TopologyError::Degenerate(format!(
                "Face {face} has fewer than 3 vertices ({n})"
            )));
        }

        let norm_face = self.face_normal(face);
        if norm_face.length_squared() < 1e-12 {
            return Err(TopologyError::Degenerate(
                "Cannot inset degenerate face with zero area or undefined normal".into(),
            ));
        }

        let orig_area = self.face_area(face);
        if orig_area <= 1e-8 {
            return Err(TopologyError::Degenerate(format!(
                "Cannot inset degenerate face {face} with near-zero area ({orig_area})"
            )));
        }

        // Collect boundary vertex handles, positions, and attributes
        let mut orig_v = Vec::with_capacity(n);
        let mut orig_pos = Vec::with_capacity(n);
        let mut orig_uvs = Vec::with_capacity(n);
        let mut orig_norms = Vec::with_capacity(n);

        for &h in &h_cycle {
            let he = self.half_edge(h).unwrap();
            let v = he.origin;
            orig_v.push(v);
            let v_data = self.vertex(v).unwrap();
            orig_pos.push(v_data.position);
            orig_uvs.push(he.uv.or(v_data.uv));
            orig_norms.push(he.normal.or(v_data.normal).unwrap_or(norm_face));
        }

        // Planarity check: all vertices must lie on the face plane
        let p0 = orig_pos[0];
        for &pos in &orig_pos[1..] {
            let dist_to_plane = ((pos - p0).dot(norm_face)).abs();
            if dist_to_plane > 1e-3 {
                return Err(TopologyError::InvalidOperation(
                    "Face is not sufficiently planar for inset".into(),
                ));
            }
        }

        // Edge vectors, lengths, tangents, and strict convexity check
        let mut edge_tangents = Vec::with_capacity(n);
        let mut edge_inward_normals = Vec::with_capacity(n);

        for i in 0..n {
            let next_i = (i + 1) % n;
            let edge_vec = orig_pos[next_i] - orig_pos[i];
            let len = edge_vec.length();
            if len < 1e-6 {
                return Err(TopologyError::Degenerate(
                    "Face contains degenerate zero-length edge".into(),
                ));
            }
            let tangent = edge_vec / len;
            let inward_norm = norm_face.cross(tangent).normalize_or_zero();
            edge_tangents.push(tangent);
            edge_inward_normals.push(inward_norm);
        }

        // Check strict convexity at every corner: (t_{prev} x t_{curr}) . norm_face > 1e-5
        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let turn = edge_tangents[prev_i].cross(edge_tangents[i]).dot(norm_face);
            if turn <= 1e-5 {
                return Err(TopologyError::InvalidOperation(
                    "Face is not strictly convex: turn angle is non-positive or collinear".into(),
                ));
            }
        }

        // Compute 3D positions of inner vertices via exact parallel edge offset:
        // delta_v = offset * (m_{prev} + m_{curr}) / (1 + m_{prev} . m_{curr})
        let mut inner_pos = Vec::with_capacity(n);
        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let m_prev = edge_inward_normals[prev_i];
            let m_curr = edge_inward_normals[i];
            let denom = 1.0 + m_prev.dot(m_curr);
            if denom <= 1e-6 {
                return Err(TopologyError::InvalidOperation(
                    "Face corners degenerate or collinear".into(),
                ));
            }
            let delta = (m_prev + m_curr) * (offset / denom);
            let p_inner = orig_pos[i] + delta;
            if !p_inner.x.is_finite() || !p_inner.y.is_finite() || !p_inner.z.is_finite() {
                return Err(TopologyError::Degenerate(
                    "Generated inner vertex positions contain non-finite coordinates".into(),
                ));
            }
            inner_pos.push(p_inner);
        }

        // Validate inner polygon:
        // 1. Strict convexity and CCW orientation of inner polygon
        let mut inner_tangents = Vec::with_capacity(n);
        for i in 0..n {
            let next_i = (i + 1) % n;
            let edge_vec = inner_pos[next_i] - inner_pos[i];
            let len = edge_vec.length();
            if len < 1e-5 {
                return Err(TopologyError::InvalidOperation(
                    "Offset too large: inner edge collapsed".into(),
                ));
            }
            inner_tangents.push(edge_vec / len);
        }

        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let turn = inner_tangents[prev_i]
                .cross(inner_tangents[i])
                .dot(norm_face);
            if turn <= 1e-5 {
                return Err(TopologyError::InvalidOperation(
                    "Offset too large: inner polygon inverted or self-intersecting".into(),
                ));
            }
        }

        // 2. Area of inner polygon
        let mut inner_area_vec = Vec3::ZERO;
        for i in 0..n {
            let next_i = (i + 1) % n;
            inner_area_vec += inner_pos[i].cross(inner_pos[next_i]);
        }
        let inner_area = 0.5 * inner_area_vec.dot(norm_face);
        if inner_area <= 1e-6 {
            return Err(TopologyError::InvalidOperation(
                "Offset too large: inner polygon area collapsed".into(),
            ));
        }

        // 3. Side face non-degeneracy
        for i in 0..n {
            let next_i = (i + 1) % n;
            // Quad vertices: orig_pos[i], orig_pos[next_i], inner_pos[next_i], inner_pos[i]
            let a1 = 0.5
                * (orig_pos[next_i] - orig_pos[i])
                    .cross(inner_pos[next_i] - orig_pos[i])
                    .dot(norm_face);
            let a2 = 0.5
                * (inner_pos[next_i] - orig_pos[i])
                    .cross(inner_pos[i] - orig_pos[i])
                    .dot(norm_face);
            let side_area = a1 + a2;
            if a1 <= 1e-5 || a2 <= 1e-5 || side_area <= 1e-6 {
                return Err(TopologyError::InvalidOperation(
                    "Offset too large: side face collapsed, inverted, or self-intersecting".into(),
                ));
            }
        }

        // Snapshot state for atomic rollback
        let backup = self.clone();

        // 2. Allocate new inner vertices
        let mut inner_v = Vec::with_capacity(n);
        for i in 0..n {
            let (uv, color, normal) = {
                let ov = self.vertex(orig_v[i]).unwrap();
                (ov.uv, ov.color, ov.normal)
            };
            let v_new = self.add_vertex(inner_pos[i]);
            let v_mut = self.vertex_mut(v_new).unwrap();
            v_mut.uv = uv;
            v_mut.color = color;
            v_mut.normal = normal.or(Some(norm_face));
            inner_v.push(v_new);
        }

        // 3. Allocate faces:
        // - inner_face
        // - side faces: side face 0 is `face` (preserved original face handle)
        //               side faces 1..N-1 are newly added
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let orig_mat_id = self.face(face).unwrap().material_id;

        let inner_face = self.add_face(dummy_he);
        self.face_mut(inner_face).unwrap().material_id = orig_mat_id;

        let mut side_faces = Vec::with_capacity(n);
        side_faces.push(face); // side face 0 is the original face
        for _ in 1..n {
            let sf = self.add_face(dummy_he);
            self.face_mut(sf).unwrap().material_id = orig_mat_id;
            side_faces.push(sf);
        }

        // 4. Allocate edges:
        // - N inner edges (u_i -> u_{(i+1)%n})
        // - N radial edges (v_i -> u_i)
        let mut inner_edges = Vec::with_capacity(n);
        for _ in 0..n {
            inner_edges.push(self.add_edge(dummy_he));
        }

        let mut radial_edges = Vec::with_capacity(n);
        for _ in 0..n {
            radial_edges.push(self.add_edge(dummy_he));
        }

        // 5. Allocate half-edges:
        // - Inner edges: h_in[i] (u_i -> u_{(i+1)%n} in inner_face)
        //                t_in[i] (u_{(i+1)%n} -> u_i in side_faces[i])
        let mut h_in = Vec::with_capacity(n);
        let mut t_in = Vec::with_capacity(n);
        for i in 0..n {
            let next_i = (i + 1) % n;
            let hi = self.add_half_edge(inner_v[i], inner_edges[i], Some(inner_face));
            let ti = self.add_half_edge(inner_v[next_i], inner_edges[i], Some(side_faces[i]));
            h_in.push(hi);
            t_in.push(ti);
        }

        // - Radial edges: r_up[i] (v_i -> u_i in side_faces[(i+n-1)%n])
        //                 r_down[i] (u_i -> v_i in side_faces[i])
        let mut r_up = Vec::with_capacity(n);
        let mut r_down = Vec::with_capacity(n);
        for i in 0..n {
            let prev_side_i = (i + n - 1) % n;
            let ru = self.add_half_edge(orig_v[i], radial_edges[i], Some(side_faces[prev_side_i]));
            let rd = self.add_half_edge(inner_v[i], radial_edges[i], Some(side_faces[i]));
            r_up.push(ru);
            r_down.push(rd);
        }

        // 6. Connect twins
        for i in 0..n {
            // Inner edges
            self.half_edge_mut(h_in[i]).unwrap().twin = t_in[i];
            self.half_edge_mut(t_in[i]).unwrap().twin = h_in[i];
            self.edge_mut(inner_edges[i]).unwrap().half_edge = h_in[i];

            // Radial edges
            self.half_edge_mut(r_up[i]).unwrap().twin = r_down[i];
            self.half_edge_mut(r_down[i]).unwrap().twin = r_up[i];
            self.edge_mut(radial_edges[i]).unwrap().half_edge = r_up[i];
        }

        // 7. Wire Inner Face cycle
        for i in 0..n {
            let next_i = (i + 1) % n;
            let prev_i = (i + n - 1) % n;
            let hi = self.half_edge_mut(h_in[i]).unwrap();
            hi.next = h_in[next_i];
            hi.prev = h_in[prev_i];
            hi.uv = orig_uvs[i];
            hi.normal = Some(orig_norms[i]);
        }
        self.face_mut(inner_face).unwrap().half_edge = h_in[0];

        // 8. Wire Side Face i cycles:
        // Cycle: h_cycle[i] -> r_up[(i+1)%n] -> t_in[i] -> r_down[i] -> h_cycle[i]
        for i in 0..n {
            let next_i = (i + 1) % n;
            let sf = side_faces[i];

            let h_bot = h_cycle[i];
            let r_right = r_up[next_i];
            let h_top = t_in[i];
            let r_left = r_down[i];

            // Set face references
            self.half_edge_mut(h_bot).unwrap().face = Some(sf);
            self.half_edge_mut(r_right).unwrap().face = Some(sf);
            self.half_edge_mut(h_top).unwrap().face = Some(sf);
            self.half_edge_mut(r_left).unwrap().face = Some(sf);

            // Set cycle links
            self.half_edge_mut(h_bot).unwrap().next = r_right;
            self.half_edge_mut(h_bot).unwrap().prev = r_left;

            self.half_edge_mut(r_right).unwrap().next = h_top;
            self.half_edge_mut(r_right).unwrap().prev = h_bot;

            self.half_edge_mut(h_top).unwrap().next = r_left;
            self.half_edge_mut(h_top).unwrap().prev = r_right;

            self.half_edge_mut(r_left).unwrap().next = h_bot;
            self.half_edge_mut(r_left).unwrap().prev = h_top;

            self.face_mut(sf).unwrap().half_edge = h_bot;

            // Corner attributes for side face
            self.half_edge_mut(h_bot).unwrap().uv = orig_uvs[i];
            self.half_edge_mut(h_bot).unwrap().normal = Some(orig_norms[i]);

            self.half_edge_mut(r_right).unwrap().uv = orig_uvs[next_i];
            self.half_edge_mut(r_right).unwrap().normal = Some(orig_norms[next_i]);

            self.half_edge_mut(h_top).unwrap().uv = orig_uvs[next_i];
            self.half_edge_mut(h_top).unwrap().normal = Some(orig_norms[next_i]);

            self.half_edge_mut(r_left).unwrap().uv = orig_uvs[i];
            self.half_edge_mut(r_left).unwrap().normal = Some(orig_norms[i]);
        }

        // 9. Update vertex outgoing half-edges
        for i in 0..n {
            self.vertex_mut(inner_v[i]).unwrap().half_edge = Some(h_in[i]);
            self.vertex_mut(orig_v[i]).unwrap().half_edge = Some(h_cycle[i]);
        }

        // 10. Recalculate face normals
        let norm_in = self.face_normal(inner_face);
        self.face_mut(inner_face).unwrap().normal = Some(norm_in);

        for &sf in &side_faces {
            let sn = self.face_normal(sf);
            self.face_mut(sf).unwrap().normal = Some(sn);
        }

        // 11. Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        // 12. Return result
        let mut new_edges = Vec::with_capacity(2 * n);
        new_edges.extend_from_slice(&inner_edges);
        new_edges.extend_from_slice(&radial_edges);

        let mut new_half_edges = Vec::with_capacity(4 * n);
        new_half_edges.extend_from_slice(&h_in);
        new_half_edges.extend_from_slice(&t_in);
        new_half_edges.extend_from_slice(&r_up);
        new_half_edges.extend_from_slice(&r_down);

        Ok(InsetFaceResult {
            original_face: face,
            inner_face,
            new_vertices: inner_v,
            new_edges,
            new_side_faces: side_faces[1..].to_vec(),
            new_half_edges,
        })
    }

    /// Topologically removes a face from the mesh.
    ///
    /// Converts shared interior edges into boundary edges, removes dangling boundary edges,
    /// and stitches the resulting boundary loops into valid closed cycles.
    /// Transactionally rolls back if an invariant is violated.
    pub fn remove_face(&mut self, face: FaceHandle) -> Result<(), TopologyError> {
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }

        let backup = self.clone();

        let f_hes = self.face_half_edges(face);
        if f_hes.is_empty() {
            self.free_face(face)?;
            return Ok(());
        }

        let face_verts: Vec<VertexHandle> = f_hes
            .iter()
            .map(|&h| self.half_edge(h).unwrap().origin)
            .collect();

        let mut edges_to_remove = Vec::new();
        let mut half_edges_to_remove = Vec::new();

        for &he_h in &f_hes {
            let he = self.half_edge(he_h).unwrap();
            let twin_h = he.twin;
            let twin = self.half_edge(twin_h).unwrap();
            let edge_h = he.edge;

            if twin.face.is_none() {
                // Both sides will have no face -> remove edge and both half-edges
                edges_to_remove.push(edge_h);
                half_edges_to_remove.push(he_h);
                half_edges_to_remove.push(twin_h);
            } else {
                // Twin has an incident face -> this half-edge becomes a boundary half-edge
                self.half_edge_mut(he_h).unwrap().face = None;
            }
        }

        for e in edges_to_remove {
            let _ = self.free_edge(e);
        }
        for h in half_edges_to_remove {
            let _ = self.free_half_edge(h);
        }

        self.free_face(face)?;

        // Re-stitch all boundary half-edges into valid boundary loops
        let all_boundary_hes: Vec<HalfEdgeHandle> = self
            .half_edges()
            .filter(|&h| {
                if let Some(he) = self.half_edge(h) {
                    he.is_boundary()
                } else {
                    false
                }
            })
            .collect();

        for &bnd_he_h in &all_boundary_hes {
            let twin_h = self.half_edge(bnd_he_h).unwrap().twin;
            let mut curr_h = twin_h;
            let mut found_next_bnd = None;

            for _ in 0..self.half_edges.len() {
                let prev_h = self.half_edge(curr_h).unwrap().prev;
                let candidate_bnd = self.half_edge(prev_h).unwrap().twin;
                if self.half_edge(candidate_bnd).unwrap().face.is_none() {
                    found_next_bnd = Some(candidate_bnd);
                    break;
                }
                curr_h = candidate_bnd;
            }

            if let Some(next_bnd_h) = found_next_bnd {
                self.half_edge_mut(bnd_he_h).unwrap().next = next_bnd_h;
                self.half_edge_mut(next_bnd_h).unwrap().prev = bnd_he_h;
            } else {
                let u_origin = self.half_edge(twin_h).unwrap().origin;
                for &other_bnd in &all_boundary_hes {
                    if self.half_edge(other_bnd).unwrap().origin == u_origin {
                        self.half_edge_mut(bnd_he_h).unwrap().next = other_bnd;
                        self.half_edge_mut(other_bnd).unwrap().prev = bnd_he_h;
                        break;
                    }
                }
            }
        }

        // Update vertex outgoing half-edges
        for vh in face_verts {
            if !self.is_valid_vertex(vh) {
                continue;
            }
            let mut assigned = None;
            for (idx, he) in self.half_edges.iter().enumerate() {
                if !he.deleted && he.origin == vh {
                    let he_h = HalfEdgeHandle::new(idx as u32, he.generation);
                    if he.is_boundary() {
                        assigned = Some(he_h);
                        break;
                    } else if assigned.is_none() {
                        assigned = Some(he_h);
                    }
                }
            }
            self.vertex_mut(vh).unwrap().half_edge = assigned;
        }

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(())
    }

    /// Alias for `remove_face`.
    #[inline]
    pub fn delete_face(&mut self, face: FaceHandle) -> Result<(), TopologyError> {
        self.remove_face(face)
    }

    fn refresh_vertex_outgoing_half_edge(&mut self, v: VertexHandle) {
        if !self.is_valid_vertex(v) {
            return;
        }
        let mut bnd = None;
        let mut any_out = None;
        for (idx, he) in self.half_edges.iter().enumerate() {
            if !he.deleted && he.origin == v {
                let he_h = HalfEdgeHandle::new(idx as u32, he.generation);
                if he.is_boundary() {
                    bnd = Some(he_h);
                    break;
                } else if any_out.is_none() {
                    any_out = Some(he_h);
                }
            }
        }
        self.vertex_mut(v).unwrap().half_edge = bnd.or(any_out);
    }

    /// Collapses edge `edge` by merging its endpoints into a single vertex at interpolation parameter `t` in [0, 1].
    ///
    /// Preserves 2-manifold invariants using the topological Link Condition.
    /// Transactionally rolls back if an invariant is violated.
    pub fn collapse_edge(
        &mut self,
        edge: EdgeHandle,
        t: f32,
    ) -> Result<CollapseEdgeResult, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err(TopologyError::InvalidOperation(format!(
                "Collapse edge parameter t ({t}) must be within [0.0, 1.0]"
            )));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let v0 = self.half_edge(h0).unwrap().origin;
        let v1 = self.half_edge(h1).unwrap().origin;

        if v0 == v1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse self-loop edge".into(),
            ));
        }

        // Boundary vertex pinching check:
        // If both vertices are on the mesh boundary, the edge itself must be a boundary edge
        if self.is_vertex_boundary(v0)
            && self.is_vertex_boundary(v1)
            && !self.is_edge_boundary(edge)
        {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse internal edge connecting two boundary vertices (pinching violation)".into(),
            ));
        }

        // Topological Link Condition:
        // Set of common 1-ring neighbors N(v0) ∩ N(v1) must strictly equal the opposite vertices
        // of faces sharing edge `e`.
        let adj_v0: HashSet<VertexHandle> = self.adjacent_vertices(v0).into_iter().collect();
        let adj_v1: HashSet<VertexHandle> = self.adjacent_vertices(v1).into_iter().collect();
        let common_neighbors: HashSet<VertexHandle> =
            adj_v0.intersection(&adj_v1).copied().collect();

        let mut expected_common = HashSet::new();
        let mut v_opp0_opt = None;
        let mut v_opp1_opt = None;
        let mut faces_to_remove_count = 0;

        if let Some(f0) = self.half_edge(h0).unwrap().face {
            let f0_hes = self.face_half_edges(f0);
            if f0_hes.len() == 3 {
                let h0_prev = self.half_edge(h0).unwrap().prev;
                let vo = self.half_edge(h0_prev).unwrap().origin;
                expected_common.insert(vo);
                v_opp0_opt = Some(vo);
                faces_to_remove_count += 1;
            }
        }
        if let Some(f1) = self.half_edge(h1).unwrap().face {
            let f1_hes = self.face_half_edges(f1);
            if f1_hes.len() == 3 {
                let h1_prev = self.half_edge(h1).unwrap().prev;
                let vo = self.half_edge(h1_prev).unwrap().origin;
                expected_common.insert(vo);
                v_opp1_opt = Some(vo);
                faces_to_remove_count += 1;
            }
        }

        // If both faces are triangles and have the same opposite vertex (e.g. 2 triangles glued back-to-back)
        if matches!((v_opp0_opt, v_opp1_opt), (Some(vo0), Some(vo1)) if vo0 == vo1) {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse edge: incident triangles share all vertices".into(),
            ));
        }

        if common_neighbors != expected_common {
            return Err(TopologyError::InvalidOperation(
                "Link condition violated: edge collapse would create non-manifold geometry or duplicate edges".into(),
            ));
        }

        // Prevent collapsing if no faces would remain
        if self.num_faces() <= faces_to_remove_count {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse edge: mesh would have no faces remaining".into(),
            ));
        }

        // Closed manifold mesh with <= 4 vertices (e.g. tetrahedron) cannot be collapsed
        if self.boundary_loops().is_empty() && self.num_vertices() <= 4 {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse edge: closed manifold mesh must have at least 4 vertices".into(),
            ));
        }

        // Check for isolated wire edge / antenna creation:
        // If an incident triangle has both other edges on the boundary, collapsing base edge creates a 0-face wire edge
        if let Some(f0) = self.half_edge(h0).unwrap().face {
            let f0_hes = self.face_half_edges(f0);
            if f0_hes.len() == 3 {
                let h0_next = self.half_edge(h0).unwrap().next;
                let h0_prev = self.half_edge(h0).unwrap().prev;
                let twin_next = self.half_edge(h0_next).unwrap().twin;
                let twin_prev = self.half_edge(h0_prev).unwrap().twin;
                if self.half_edge(twin_next).unwrap().face.is_none()
                    && self.half_edge(twin_prev).unwrap().face.is_none()
                {
                    return Err(TopologyError::InvalidOperation(
                        "Cannot collapse edge: would create isolated non-manifold wire edge".into(),
                    ));
                }
            }
        }
        if let Some(f1) = self.half_edge(h1).unwrap().face {
            let f1_hes = self.face_half_edges(f1);
            if f1_hes.len() == 3 {
                let h1_next = self.half_edge(h1).unwrap().next;
                let h1_prev = self.half_edge(h1).unwrap().prev;
                let twin_next = self.half_edge(h1_next).unwrap().twin;
                let twin_prev = self.half_edge(h1_prev).unwrap().twin;
                if self.half_edge(twin_next).unwrap().face.is_none()
                    && self.half_edge(twin_prev).unwrap().face.is_none()
                {
                    return Err(TopologyError::InvalidOperation(
                        "Cannot collapse edge: would create isolated non-manifold wire edge".into(),
                    ));
                }
            }
        }

        // Check if edge is boundary and on a degenerate boundary loop of length <= 2
        if self.half_edge(h0).unwrap().face.is_none()
            && self.boundary_loop(h0).map(|l| l.len()).unwrap_or(0) <= 2
        {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse boundary edge on degenerate boundary loop".into(),
            ));
        }
        if self.half_edge(h1).unwrap().face.is_none()
            && self.boundary_loop(h1).map(|l| l.len()).unwrap_or(0) <= 2
        {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse boundary edge on degenerate boundary loop".into(),
            ));
        }

        // Geometric position interpolation: P = P0 + (P1 - P0) * t
        let p0 = self.vertex(v0).unwrap().position;
        let p1 = self.vertex(v1).unwrap().position;
        let p_merged = p0 + (p1 - p0) * t;
        self.vertex_mut(v0).unwrap().position = p_merged;

        // Normal interpolation
        let n0 = self.vertex(v0).unwrap().normal;
        let n1 = self.vertex(v1).unwrap().normal;
        let n_merged = match (n0, n1) {
            (Some(a), Some(b)) => {
                let blended = (1.0 - t) * a + t * b;
                if blended.length_squared() > 1e-12 {
                    Some(blended.normalize())
                } else {
                    Some(blended)
                }
            }
            (Some(a), None) if t < 0.5 => Some(a),
            (None, Some(b)) if t >= 0.5 => Some(b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        self.vertex_mut(v0).unwrap().normal = n_merged;

        // UV interpolation
        let uv0 = self.vertex(v0).unwrap().uv;
        let uv1 = self.vertex(v1).unwrap().uv;
        let uv_merged = match (uv0, uv1) {
            (Some(a), Some(b)) => Some((1.0 - t) * a + t * b),
            (Some(a), None) if t < 0.5 => Some(a),
            (None, Some(b)) if t >= 0.5 => Some(b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        self.vertex_mut(v0).unwrap().uv = uv_merged;

        // Color RGBA interpolation
        let c0 = self.vertex(v0).unwrap().color;
        let c1 = self.vertex(v1).unwrap().color;
        let c_merged = match (c0, c1) {
            (Some(a), Some(b)) => Some([
                (1.0 - t) * a[0] + t * b[0],
                (1.0 - t) * a[1] + t * b[1],
                (1.0 - t) * a[2] + t * b[2],
                (1.0 - t) * a[3] + t * b[3],
            ]),
            (Some(a), None) if t < 0.5 => Some(a),
            (None, Some(b)) if t >= 0.5 => Some(b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        self.vertex_mut(v0).unwrap().color = c_merged;

        let mut removed_faces = Vec::new();
        let mut removed_edges = Vec::new();
        let mut removed_half_edges = Vec::new();

        // Mutate Side 0 (h0)
        if let Some(f0) = self.half_edge(h0).unwrap().face {
            let f0_hes = self.face_half_edges(f0);
            if f0_hes.len() == 3 {
                let h0_next = self.half_edge(h0).unwrap().next;
                let h0_prev = self.half_edge(h0).unwrap().prev;

                let twin_next = self.half_edge(h0_next).unwrap().twin;
                let twin_prev = self.half_edge(h0_prev).unwrap().twin;

                let edge_to_remove = self.half_edge(h0_next).unwrap().edge;
                let edge_to_keep = self.half_edge(h0_prev).unwrap().edge;

                let v_opp = self.half_edge(h0_prev).unwrap().origin;

                self.half_edge_mut(twin_next).unwrap().twin = twin_prev;
                self.half_edge_mut(twin_prev).unwrap().twin = twin_next;

                self.half_edge_mut(twin_next).unwrap().edge = edge_to_keep;
                self.half_edge_mut(twin_prev).unwrap().edge = edge_to_keep;
                self.edge_mut(edge_to_keep).unwrap().half_edge = twin_prev;

                if self.vertex(v_opp).unwrap().half_edge == Some(h0_prev) {
                    self.vertex_mut(v_opp).unwrap().half_edge = Some(twin_next);
                }

                self.free_half_edge(h0_next)?;
                self.free_half_edge(h0_prev)?;
                self.free_edge(edge_to_remove)?;
                self.free_face(f0)?;

                removed_half_edges.push(h0_next);
                removed_half_edges.push(h0_prev);
                removed_edges.push(edge_to_remove);
                removed_faces.push(f0);
            } else {
                let h0_next = self.half_edge(h0).unwrap().next;
                let h0_prev = self.half_edge(h0).unwrap().prev;
                self.half_edge_mut(h0_prev).unwrap().next = h0_next;
                self.half_edge_mut(h0_next).unwrap().prev = h0_prev;
                if self.face(f0).unwrap().half_edge == h0 {
                    self.face_mut(f0).unwrap().half_edge = h0_next;
                }
            }
        } else {
            let h0_next = self.half_edge(h0).unwrap().next;
            let h0_prev = self.half_edge(h0).unwrap().prev;
            self.half_edge_mut(h0_prev).unwrap().next = h0_next;
            self.half_edge_mut(h0_next).unwrap().prev = h0_prev;
        }

        // Mutate Side 1 (h1)
        if let Some(f1) = self.half_edge(h1).unwrap().face {
            let f1_hes = self.face_half_edges(f1);
            if f1_hes.len() == 3 {
                let h1_next = self.half_edge(h1).unwrap().next;
                let h1_prev = self.half_edge(h1).unwrap().prev;

                let twin_next = self.half_edge(h1_next).unwrap().twin;
                let twin_prev = self.half_edge(h1_prev).unwrap().twin;

                let edge_to_remove = self.half_edge(h1_next).unwrap().edge;
                let edge_to_keep = self.half_edge(h1_prev).unwrap().edge;

                let v_opp = self.half_edge(h1_prev).unwrap().origin;

                self.half_edge_mut(twin_next).unwrap().twin = twin_prev;
                self.half_edge_mut(twin_prev).unwrap().twin = twin_next;

                self.half_edge_mut(twin_next).unwrap().edge = edge_to_keep;
                self.half_edge_mut(twin_prev).unwrap().edge = edge_to_keep;
                self.edge_mut(edge_to_keep).unwrap().half_edge = twin_prev;

                if self.vertex(v_opp).unwrap().half_edge == Some(h1_prev) {
                    self.vertex_mut(v_opp).unwrap().half_edge = Some(twin_next);
                }

                self.free_half_edge(h1_next)?;
                self.free_half_edge(h1_prev)?;
                self.free_edge(edge_to_remove)?;
                self.free_face(f1)?;

                removed_half_edges.push(h1_next);
                removed_half_edges.push(h1_prev);
                removed_edges.push(edge_to_remove);
                removed_faces.push(f1);
            } else {
                let h1_next = self.half_edge(h1).unwrap().next;
                let h1_prev = self.half_edge(h1).unwrap().prev;
                self.half_edge_mut(h1_prev).unwrap().next = h1_next;
                self.half_edge_mut(h1_next).unwrap().prev = h1_prev;
                if self.face(f1).unwrap().half_edge == h1 {
                    self.face_mut(f1).unwrap().half_edge = h1_next;
                }
            }
        } else {
            let h1_next = self.half_edge(h1).unwrap().next;
            let h1_prev = self.half_edge(h1).unwrap().prev;
            self.half_edge_mut(h1_prev).unwrap().next = h1_next;
            self.half_edge_mut(h1_next).unwrap().prev = h1_prev;
        }

        // Retarget all remaining active half-edges originating from v1 to originate from v0
        for he in &mut self.half_edges {
            if !he.deleted && he.origin == v1 {
                he.origin = v0;
            }
        }

        // Remove h0, h1, edge, and v1
        self.free_half_edge(h0)?;
        self.free_half_edge(h1)?;
        self.free_edge(edge)?;
        self.free_vertex(v1)?;

        removed_half_edges.push(h0);
        removed_half_edges.push(h1);
        removed_edges.push(edge);

        // Refresh outgoing half-edges for v0 and opposite vertices
        self.refresh_vertex_outgoing_half_edge(v0);
        if let Some(vo0) = v_opp0_opt {
            self.refresh_vertex_outgoing_half_edge(vo0);
        }
        if let Some(vo1) = v_opp1_opt {
            self.refresh_vertex_outgoing_half_edge(vo1);
        }

        // Recalculate face normals of all adjacent faces to v0
        for f in self.adjacent_faces(v0) {
            let n = self.face_normal(f);
            if let Some(face_ref) = self.face_mut(f) {
                face_ref.normal = Some(n);
            }
        }

        // Transactional invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        let affected_edges = self.incident_edges(v0);
        Ok(CollapseEdgeResult {
            surviving_vertex: v0,
            removed_vertex: v1,
            collapsed_edge: edge,
            removed_faces,
            removed_edges,
            removed_half_edges,
            affected_edges,
        })
    }

    /// Removes an isolated vertex that has no incident edges.
    pub fn remove_isolated_vertex(&mut self, vertex: VertexHandle) -> Result<(), TopologyError> {
        if !self.is_valid_vertex(vertex) {
            return Err(TopologyError::InvalidVertex(vertex));
        }

        if let Some(vert) = self.vertex(vertex)
            && vert.half_edge.is_some()
        {
            return Err(TopologyError::InvalidOperation(format!(
                "Cannot remove vertex {vertex}: it has incident edges"
            )));
        }

        self.free_vertex(vertex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::TopologyValidator;
    use crate::topology::builder::TopologyBuilder;
    use forge_math::{Vec2, Vec3};

    fn make_two_quads() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 0.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
            Vec3::new(1.0, 1.0, 0.0), // 4
            Vec3::new(2.0, 1.0, 0.0), // 5
        ];
        let faces = vec![
            vec![0, 1, 4, 3], // Face 0
            vec![1, 2, 5, 4], // Face 1
        ];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Helper: finds the edge connecting two vertices by their original index
    fn find_edge_between(mesh: &HalfEdgeMesh, v0_idx: usize, v1_idx: usize) -> EdgeHandle {
        mesh.edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == v0_idx && v1.index() == v1_idx)
                    || (v0.index() == v1_idx && v1.index() == v0_idx)
            })
            .expect("edge connecting specified vertices")
    }

    // Test 1: Split Edge in the middle of an interior edge
    #[test]
    fn test_1_split_edge_interior_midpoint() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let res = mesh.split_edge(e_mid, 0.5).expect("split edge");
        assert!(mesh.is_valid_vertex(res.vertex));
        assert!(mesh.is_valid_edge(res.original_edge));
        assert!(mesh.is_valid_edge(res.new_edge));

        let pos = mesh.vertex(res.vertex).unwrap().position;
        assert!((pos - Vec3::new(1.0, 0.5, 0.0)).length() < 1e-5);

        assert!(res.affected_faces.0.is_some());
        assert!(res.affected_faces.1.is_some());

        assert_eq!(mesh.num_vertices(), 7);
        assert_eq!(mesh.num_edges(), 8);
    }

    // Test 2: Split Edge with t = 0.25
    #[test]
    fn test_2_split_edge_t_0_25() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let (h0, _) = mesh.edge_half_edges(e_mid).unwrap();
        let v0 = mesh.half_edge(h0).unwrap().origin;
        let v1 = mesh
            .half_edge(mesh.half_edge(h0).unwrap().twin)
            .unwrap()
            .origin;
        let p0 = mesh.vertex(v0).unwrap().position;
        let p1 = mesh.vertex(v1).unwrap().position;

        let res = mesh.split_edge(e_mid, 0.25).unwrap();
        let pos = mesh.vertex(res.vertex).unwrap().position;
        let expected = p0 + (p1 - p0) * 0.25;
        assert!((pos - expected).length() < 1e-5);
    }

    // Test 3: Split Edge with t = 0.75
    #[test]
    fn test_3_split_edge_t_0_75() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let (h0, _) = mesh.edge_half_edges(e_mid).unwrap();
        let v0 = mesh.half_edge(h0).unwrap().origin;
        let v1 = mesh
            .half_edge(mesh.half_edge(h0).unwrap().twin)
            .unwrap()
            .origin;
        let p0 = mesh.vertex(v0).unwrap().position;
        let p1 = mesh.vertex(v1).unwrap().position;

        let res = mesh.split_edge(e_mid, 0.75).unwrap();
        let pos = mesh.vertex(res.vertex).unwrap().position;
        let expected = p0 + (p1 - p0) * 0.75;
        assert!((pos - expected).length() < 1e-5);
    }

    // Test 4: Split Edge of a boundary edge
    #[test]
    fn test_4_split_edge_boundary_edge() {
        let mut mesh = make_two_quads();
        let e_bnd = find_edge_between(&mesh, 0, 3);
        assert!(mesh.is_edge_boundary(e_bnd));

        let res = mesh.split_edge(e_bnd, 0.5).unwrap();
        assert!(mesh.is_edge_boundary(res.original_edge));
        assert!(mesh.is_edge_boundary(res.new_edge));
        assert!(mesh.is_vertex_boundary(res.vertex));

        // One face is Some, the other is None (virtual boundary)
        assert!(
            (res.affected_faces.0.is_some() && res.affected_faces.1.is_none())
                || (res.affected_faces.0.is_none() && res.affected_faces.1.is_some())
        );

        let bnd_loops = mesh.boundary_loops();
        assert_eq!(bnd_loops.len(), 1);
        assert_eq!(bnd_loops[0].len(), 7); // 6 + 1 boundary half-edges
    }

    // Test 5: Split Edge of an edge belonging to a triangle
    #[test]
    fn test_5_split_edge_triangle_edge() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

        let e = mesh.edges().next().unwrap();
        let res = mesh.split_edge(e, 0.5).unwrap();

        let f = mesh.faces().next().unwrap();
        assert_eq!(mesh.face_vertex_count(f), 4);
        assert!(mesh.face_vertices(f).contains(&res.vertex));
    }

    // Test 6: Split Edge shared between two faces
    #[test]
    fn test_6_split_edge_shared_two_faces() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let res = mesh.split_edge(e_mid, 0.5).unwrap();
        let f0 = res.affected_faces.0.unwrap();
        let f1 = res.affected_faces.1.unwrap();

        assert_ne!(f0, f1);
        assert_eq!(mesh.face_vertex_count(f0), 5);
        assert_eq!(mesh.face_vertex_count(f1), 5);

        assert!(mesh.face_vertices(f0).contains(&res.vertex));
        assert!(mesh.face_vertices(f1).contains(&res.vertex));
    }

    // Test 7: Verification of position interpolation
    #[test]
    fn test_7_check_position_interpolation() {
        let positions = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(5.0, 6.0, 7.0),
            Vec3::new(3.0, 8.0, 5.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

        let e = find_edge_between(&mesh, 0, 1);
        let t = 0.4;
        let res = mesh.split_edge(e, t).unwrap();

        let pos = mesh.vertex(res.vertex).unwrap().position;
        let p0 = Vec3::new(1.0, 2.0, 3.0);
        let p1 = Vec3::new(5.0, 6.0, 7.0);
        let expected = p0 + (p1 - p0) * t;
        assert!((pos - expected).length() < 1e-5);
    }

    // Test 8: Verification of attribute interpolation (normal, UV, color)
    #[test]
    fn test_8_check_attribute_interpolation() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let (h0, _) = mesh.edge_half_edges(e_mid).unwrap();
        let v0 = mesh.half_edge(h0).unwrap().origin;
        let v1 = mesh
            .half_edge(mesh.half_edge(h0).unwrap().twin)
            .unwrap()
            .origin;

        // Assign attributes to v0 and v1
        mesh.vertex_mut(v0).unwrap().normal = Some(Vec3::new(0.0, 1.0, 0.0));
        mesh.vertex_mut(v1).unwrap().normal = Some(Vec3::new(0.0, 0.0, 1.0));

        mesh.vertex_mut(v0).unwrap().uv = Some(Vec2::new(0.2, 0.4));
        mesh.vertex_mut(v1).unwrap().uv = Some(Vec2::new(0.6, 0.8));

        mesh.vertex_mut(v0).unwrap().color = Some([1.0, 0.0, 0.0, 1.0]);
        mesh.vertex_mut(v1).unwrap().color = Some([0.0, 1.0, 0.0, 1.0]);

        let t = 0.5;
        let res = mesh.split_edge(e_mid, t).unwrap();
        let v_new = mesh.vertex(res.vertex).unwrap();

        // Normal must be normalized
        let n_expected = (Vec3::new(0.0, 1.0, 0.0) + Vec3::new(0.0, 0.0, 1.0)).normalize();
        assert!((v_new.normal.unwrap() - n_expected).length() < 1e-5);

        // UV interpolation
        assert!((v_new.uv.unwrap() - Vec2::new(0.4, 0.6)).length() < 1e-5);

        // Color interpolation
        let c = v_new.color.unwrap();
        assert!((c[0] - 0.5).abs() < 1e-5);
        assert!((c[1] - 0.5).abs() < 1e-5);
        assert!((c[2] - 0.0).abs() < 1e-5);
        assert!((c[3] - 1.0).abs() < 1e-5);
    }

    // Test 9: Preservation of UV seams
    #[test]
    fn test_9_preserve_uv_seams() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let (h0, h1) = mesh.edge_half_edges(e_mid).unwrap();
        let h0_next = mesh.half_edge(h0).unwrap().next;
        let h1_next = mesh.half_edge(h1).unwrap().next;

        // Assign different wedge UVs on both sides (a UV seam along e_mid)
        mesh.half_edge_mut(h0).unwrap().uv = Some(Vec2::new(0.0, 0.0));
        mesh.half_edge_mut(h0_next).unwrap().uv = Some(Vec2::new(1.0, 0.0));

        mesh.half_edge_mut(h1).unwrap().uv = Some(Vec2::new(0.0, 1.0));
        mesh.half_edge_mut(h1_next).unwrap().uv = Some(Vec2::new(0.0, 0.5));

        let res = mesh.split_edge(e_mid, 0.5).unwrap();
        let (h0b, h1b) = res.new_half_edges;

        let uv_side0 = mesh.half_edge(h0b).unwrap().uv.unwrap();
        let uv_side1 = mesh.half_edge(h1b).unwrap().uv.unwrap();

        assert!((uv_side0 - Vec2::new(0.5, 0.0)).length() < 1e-5);
        assert!((uv_side1 - Vec2::new(0.0, 0.75)).length() < 1e-5);

        // The UV seam is preserved: the two corners on opposite sides have different coordinates
        assert_ne!(uv_side0, uv_side1);
    }

    // Test 10: Verification of Twin reciprocity after split
    #[test]
    fn test_10_verify_twins_after_split() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let res = mesh.split_edge(e_mid, 0.5).unwrap();
        let (h0b, h1b) = res.new_half_edges;

        // Check mutual twin property for all half-edges in the mesh
        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(
                twin.twin, h,
                "twin reciprocity must hold for all half-edges"
            );
        }

        let (h0, h1) = (
            mesh.half_edge(h1b).unwrap().twin,
            mesh.half_edge(h0b).unwrap().twin,
        );
        assert_eq!(mesh.half_edge(h0).unwrap().twin, h1b);
        assert_eq!(mesh.half_edge(h1).unwrap().twin, h0b);
    }

    // Test 11: Verification of Next and Prev cycles after split
    #[test]
    fn test_11_verify_next_and_prev() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);

        let res = mesh.split_edge(e_mid, 0.5).unwrap();
        let (h0b, h1b) = res.new_half_edges;

        let h0 = mesh.half_edge(h0b).unwrap().prev;
        let h1 = mesh.half_edge(h1b).unwrap().prev;

        assert_eq!(mesh.half_edge(h0).unwrap().next, h0b);
        assert_eq!(mesh.half_edge(h1).unwrap().next, h1b);

        // Verify full cycle closure and prev/next consistency around affected faces
        for f in [res.affected_faces.0, res.affected_faces.1]
            .into_iter()
            .flatten()
        {
            let hes = mesh.face_half_edges(f);
            assert_eq!(hes.len(), 5);
            for i in 0..hes.len() {
                let curr = hes[i];
                let next = hes[(i + 1) % hes.len()];
                assert_eq!(mesh.half_edge(curr).unwrap().next, next);
                assert_eq!(mesh.half_edge(next).unwrap().prev, curr);
            }
        }
    }

    // Test 12: Verification of Boundary Loop integrity
    #[test]
    fn test_12_boundary_loop_preservation() {
        let mut mesh = make_two_quads();
        let initial_loops = mesh.boundary_loops();
        assert_eq!(initial_loops.len(), 1);
        let initial_len = initial_loops[0].len();

        let e_bnd = find_edge_between(&mesh, 2, 5);
        assert!(mesh.is_edge_boundary(e_bnd));

        let _res = mesh.split_edge(e_bnd, 0.3).unwrap();

        let after_loops = mesh.boundary_loops();
        assert_eq!(after_loops.len(), 1);
        assert_eq!(after_loops[0].len(), initial_len + 1);
    }

    // Test 13: Verification of mesh validity via TopologyValidator
    #[test]
    fn test_13_topology_validator_pass() {
        let mut mesh = make_two_quads();
        let e_int = find_edge_between(&mesh, 1, 4);
        mesh.split_edge(e_int, 0.5).unwrap();

        let e_bnd = find_edge_between(&mesh, 0, 1);
        mesh.split_edge(e_bnd, 0.3).unwrap();

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    // Test 14: Handling of invalid EdgeHandle
    #[test]
    fn test_14_invalid_edge_handle() {
        let mut mesh = make_two_quads();

        let res_null = mesh.split_edge(EdgeHandle::NULL, 0.5);
        assert!(matches!(res_null, Err(TopologyError::InvalidEdge(_))));

        let res_oob = mesh.split_edge(EdgeHandle::new(9999, 0), 0.5);
        assert!(matches!(res_oob, Err(TopologyError::InvalidEdge(_))));

        let valid_e = mesh.edges().next().unwrap();
        let wrong_gen = EdgeHandle::new(valid_e.raw_index(), valid_e.generation() + 1);
        let res_gen = mesh.split_edge(wrong_gen, 0.5);
        assert!(matches!(res_gen, Err(TopologyError::InvalidEdge(_))));
    }

    // Test 15: Handling of invalid parameter t
    #[test]
    fn test_15_invalid_t_values() {
        let mut mesh = make_two_quads();
        let e = mesh.edges().next().unwrap();

        for &bad_t in &[
            0.0,
            1.0,
            -0.5,
            1.5,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let res = mesh.split_edge(e, bad_t);
            assert!(
                matches!(res, Err(TopologyError::InvalidOperation(_))),
                "bad t ({bad_t}) must return InvalidOperation"
            );
        }
    }

    // Test 16: Verification of atomicity on error
    #[test]
    fn test_16_atomic_rollback_on_error() {
        let mut mesh = make_two_quads();
        let mesh_backup = mesh.clone();

        let e = mesh.edges().next().unwrap();

        // 1. Error on invalid t
        assert!(mesh.split_edge(e, -0.1).is_err());
        assert_eq!(mesh, mesh_backup);

        // 2. Error on invalid handle
        assert!(mesh.split_edge(EdgeHandle::NULL, 0.5).is_err());
        assert_eq!(mesh, mesh_backup);

        // 3. Topology must remain completely intact
        assert!(mesh.check_invariants().is_ok());
    }

    fn make_two_triangles() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(2.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 2.0, 0.0), // 2
            Vec3::new(0.0, 2.0, 0.0), // 3
        ];
        let faces = vec![
            vec![0, 1, 2], // Face 0
            vec![0, 2, 3], // Face 1
        ];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    fn make_tetrahedron() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 1.732, 0.0),
            Vec3::new(1.0, 0.577, 1.633),
        ];
        let faces = vec![vec![0, 2, 1], vec![0, 1, 3], vec![1, 2, 3], vec![2, 0, 3]];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Flip Edge Test 1: Flip edge between two adjacent triangles
    #[test]
    fn test_flip_1_adjacent_triangles() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e).expect("flip succeeds");

        assert_eq!(res.edge(), e);
        assert!(mesh.is_valid_face(res.faces().0));
        assert!(mesh.is_valid_face(res.faces().1));
        assert_ne!(res.faces().0, res.faces().1);
        assert!(mesh.is_valid_edge(res.edge()));
    }

    // Flip Edge Test 2: Check new diagonal connects opposite vertices v2 and v3
    #[test]
    fn test_flip_2_new_diagonal_opposite_vertices() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e).unwrap();

        let (diag_a, diag_b) = res.new_diagonal();
        let idx_a = diag_a.index();
        let idx_b = diag_b.index();
        assert!((idx_a == 1 && idx_b == 3) || (idx_a == 3 && idx_b == 1));

        let (va, vb) = mesh.edge_vertices(e).unwrap();
        assert!((va.index() == 1 && vb.index() == 3) || (va.index() == 3 && vb.index() == 1));

        let (h0, h1) = res.half_edges();
        let orig0 = mesh.half_edge(h0).unwrap().origin;
        let orig1 = mesh.half_edge(h1).unwrap().origin;
        assert_eq!(orig0, diag_a);
        assert_eq!(orig1, diag_b);
    }

    // Flip Edge Test 3: Check vertex count preserved
    #[test]
    fn test_flip_3_vertex_count_preserved() {
        let mut mesh = make_two_triangles();
        let initial_v = mesh.num_vertices();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.flip_edge(e).unwrap();
        assert_eq!(mesh.num_vertices(), initial_v);
    }

    // Flip Edge Test 4: Check edge count preserved
    #[test]
    fn test_flip_4_edge_count_preserved() {
        let mut mesh = make_two_triangles();
        let initial_e = mesh.num_edges();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.flip_edge(e).unwrap();
        assert_eq!(mesh.num_edges(), initial_e);
    }

    // Flip Edge Test 5: Check face count preserved
    #[test]
    fn test_flip_5_face_count_preserved() {
        let mut mesh = make_two_triangles();
        let initial_f = mesh.num_faces();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.flip_edge(e).unwrap();
        assert_eq!(mesh.num_faces(), initial_f);
    }

    // Flip Edge Test 6: Check half-edge count preserved
    #[test]
    fn test_flip_6_half_edge_count_preserved() {
        let mut mesh = make_two_triangles();
        let initial_he = mesh.half_edges.len();
        let initial_active_he = mesh.half_edges().count();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.flip_edge(e).unwrap();
        assert_eq!(mesh.half_edges.len(), initial_he);
        assert_eq!(mesh.half_edges().count(), initial_active_he);
    }

    // Flip Edge Test 7: Check Twin reciprocity after flip
    #[test]
    fn test_flip_7_twin_reciprocity() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e).unwrap();
        let (h0, h1) = res.half_edges();

        assert_eq!(mesh.half_edge(h0).unwrap().twin, h1);
        assert_eq!(mesh.half_edge(h1).unwrap().twin, h0);

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(
                twin.twin, h,
                "Twin reciprocity must hold for all half-edges"
            );
        }
    }

    // Flip Edge Test 8: Check Next and Prev forms closed 3-cycles around F0 and F1
    #[test]
    fn test_flip_8_next_prev_closed_3_cycles() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e).unwrap();

        for f in [res.faces.0, res.faces.1] {
            let hes = mesh.face_half_edges(f);
            assert_eq!(hes.len(), 3);
            for i in 0..3 {
                let curr = hes[i];
                let next = hes[(i + 1) % 3];
                let prev = hes[(i + 2) % 3];
                assert_eq!(mesh.half_edge(curr).unwrap().next, next);
                assert_eq!(mesh.half_edge(curr).unwrap().prev, prev);
                assert_eq!(mesh.half_edge(next).unwrap().prev, curr);
                assert_eq!(mesh.half_edge(prev).unwrap().next, curr);
                assert_eq!(mesh.half_edge(curr).unwrap().face, Some(f));
            }
        }
    }

    // Flip Edge Test 9: Check both faces contain correct 3-vertex cycles
    #[test]
    fn test_flip_9_correct_face_cycles() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e).unwrap();

        let mut verts0: Vec<usize> = mesh
            .face_vertices(res.faces.0)
            .into_iter()
            .map(|v| v.index())
            .collect();
        let mut verts1: Vec<usize> = mesh
            .face_vertices(res.faces.1)
            .into_iter()
            .map(|v| v.index())
            .collect();

        assert_eq!(verts0.len(), 3);
        assert_eq!(verts1.len(), 3);

        verts0.sort_unstable();
        verts1.sort_unstable();

        // The two faces must be {1, 2, 3} and {0, 1, 3} in either order
        let case_a = verts0 == vec![1, 2, 3] && verts1 == vec![0, 1, 3];
        let case_b = verts0 == vec![0, 1, 3] && verts1 == vec![1, 2, 3];
        assert!(
            case_a || case_b,
            "Faces must contain {{1, 2, 3}} and {{0, 1, 3}}, got: {:?} and {:?}",
            verts0,
            verts1
        );
    }

    // Flip Edge Test 10: Check vertex coordinates remain identical
    #[test]
    fn test_flip_10_vertex_coordinates_identical() {
        let mut mesh = make_two_triangles();
        let initial_positions: Vec<Vec3> = mesh
            .vertices()
            .map(|v| mesh.vertex(v).unwrap().position)
            .collect();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.flip_edge(e).unwrap();

        for (v, orig_pos) in mesh.vertices().zip(initial_positions.iter()) {
            assert_eq!(mesh.vertex(v).unwrap().position, *orig_pos);
        }
    }

    // Flip Edge Test 11: Check preservation of UV seams across the quad
    #[test]
    fn test_flip_11_preserve_uv_seams() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();
        let h0_prev = mesh.half_edge(h0).unwrap().prev;
        let h1_prev = mesh.half_edge(h1).unwrap().prev;

        // Assign corner UVs to the opposite vertices
        mesh.half_edge_mut(h0_prev).unwrap().uv = Some(Vec2::new(0.2, 0.3));
        mesh.half_edge_mut(h1_prev).unwrap().uv = Some(Vec2::new(0.7, 0.8));

        let res = mesh.flip_edge(e).unwrap();
        let (h0_new, h1_new) = res.half_edges;

        // h0 now originates at v2 (previously h0_prev's origin) and inherits its corner UV
        assert_eq!(
            mesh.half_edge(h0_new).unwrap().uv,
            Some(Vec2::new(0.2, 0.3))
        );
        // h1 now originates at v3 (previously h1_prev's origin) and inherits its corner UV
        assert_eq!(
            mesh.half_edge(h1_new).unwrap().uv,
            Some(Vec2::new(0.7, 0.8))
        );
    }

    // Flip Edge Test 12: Attempt flip of boundary edge (returns Err(TopologyError::InvalidOperation))
    #[test]
    fn test_flip_12_boundary_edge_rejected() {
        let mut mesh = make_two_triangles();
        let e_bnd = find_edge_between(&mesh, 0, 1);
        assert!(mesh.is_edge_boundary(e_bnd));
        let res = mesh.flip_edge(e_bnd);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Flip Edge Test 13: Attempt flip of edge on Quad/N-gon (returns Err(TopologyError::InvalidOperation))
    #[test]
    fn test_flip_13_quad_or_ngon_rejected() {
        let mut mesh = make_two_quads();
        let e_mid = find_edge_between(&mesh, 1, 4);
        let res = mesh.flip_edge(e_mid);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Flip Edge Test 14: Handling of invalid EdgeHandle (NULL, out of bounds, wrong generation)
    #[test]
    fn test_flip_14_invalid_edge_handle() {
        let mut mesh = make_two_triangles();

        let res_null = mesh.flip_edge(EdgeHandle::NULL);
        assert!(matches!(res_null, Err(TopologyError::InvalidEdge(_))));

        let res_oob = mesh.flip_edge(EdgeHandle::new(9999, 0));
        assert!(matches!(res_oob, Err(TopologyError::InvalidEdge(_))));

        let valid_e = mesh.edges().next().unwrap();
        let wrong_gen = EdgeHandle::new(valid_e.raw_index(), valid_e.generation() + 1);
        let res_gen = mesh.flip_edge(wrong_gen);
        assert!(matches!(res_gen, Err(TopologyError::InvalidEdge(_))));
    }

    // Flip Edge Test 15: Handling of duplicate new diagonal (e.g. in tetrahedron)
    #[test]
    fn test_flip_15_duplicate_diagonal_rejected() {
        let mut mesh = make_tetrahedron();
        // In the tetrahedron, face 0 is [0, 2, 1] and face 1 is [0, 1, 3]
        // Shared edge connects 0 and 1
        let e_shared = find_edge_between(&mesh, 0, 1);
        let res = mesh.flip_edge(e_shared);
        assert!(
            matches!(res, Err(TopologyError::InvalidOperation(_))),
            "Flipping edge where opposite vertices are already connected must be rejected"
        );
    }

    // Flip Edge Test 16: Handling of non-convex/degenerate quad geometry
    #[test]
    fn test_flip_16_non_convex_or_degenerate_quad() {
        // Non-convex quad (reflex angle at vertex 2)
        let positions_concave = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(2.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 0.2, 0.0), // 2 (indented)
            Vec3::new(1.0, 2.0, 0.0), // 3
        ];
        let faces_concave = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mut mesh_concave = TopologyBuilder::build(&positions_concave, &faces_concave).unwrap();
        let e_shared = find_edge_between(&mesh_concave, 0, 2);
        let res_concave = mesh_concave.flip_edge(e_shared);
        assert!(
            matches!(res_concave, Err(TopologyError::InvalidOperation(_))),
            "Non-convex quad flip must be rejected"
        );

        // Collinear / degenerate triangle
        let positions_collinear = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 0.0, 0.0), // 2 (collinear with 0 and 1)
            Vec3::new(1.0, 1.0, 0.0), // 3
        ];
        let faces_collinear = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mut mesh_collinear =
            TopologyBuilder::build(&positions_collinear, &faces_collinear).unwrap();
        let e_collinear = find_edge_between(&mesh_collinear, 0, 2);
        let res_collinear = mesh_collinear.flip_edge(e_collinear);
        assert!(
            matches!(res_collinear, Err(TopologyError::InvalidOperation(_))),
            "Collinear triangle flip must be rejected"
        );
    }

    // Flip Edge Test 17: Atomicity check on error (mesh unchanged if flip fails)
    #[test]
    fn test_flip_17_atomic_rollback_on_error() {
        let mut mesh = make_two_triangles();
        let mesh_backup = mesh.clone();

        // 1. Error on boundary edge
        let e_bnd = find_edge_between(&mesh, 0, 1);
        assert!(mesh.flip_edge(e_bnd).is_err());
        assert_eq!(mesh, mesh_backup);

        // 2. Error on invalid handle
        assert!(mesh.flip_edge(EdgeHandle::NULL).is_err());
        assert_eq!(mesh, mesh_backup);

        // 3. Topology must remain completely intact
        assert!(mesh.check_invariants().is_ok());
    }

    // Flip Edge Test 18: Final mesh validation via TopologyValidator and check_invariants
    #[test]
    fn test_flip_18_validator_and_invariants() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.flip_edge(e);
        assert!(res.is_ok());

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    fn make_octahedron() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(1.0, 0.0, 0.0),  // 0
            Vec3::new(-1.0, 0.0, 0.0), // 1
            Vec3::new(0.0, 1.0, 0.0),  // 2
            Vec3::new(0.0, -1.0, 0.0), // 3
            Vec3::new(0.0, 0.0, 1.0),  // 4
            Vec3::new(0.0, 0.0, -1.0), // 5
        ];
        let faces = vec![
            vec![0, 2, 4],
            vec![2, 1, 4],
            vec![1, 3, 4],
            vec![3, 0, 4],
            vec![2, 0, 5],
            vec![1, 2, 5],
            vec![3, 1, 5],
            vec![0, 3, 5],
        ];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Collapse Edge Test 1: Collapse Edge of an interior edge
    #[test]
    fn test_collapse_1_interior_edge() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.collapse_edge(e, 0.5).expect("Collapse interior edge");

        assert!(mesh.is_valid_vertex(res.surviving_vertex()));
        assert!(!mesh.is_valid_vertex(res.removed_vertex()));
        assert!(!mesh.is_valid_edge(res.collapsed_edge()));
        assert_eq!(res.removed_faces().len(), 2);
    }

    // Collapse Edge Test 2: Collapse Edge of a boundary edge
    #[test]
    fn test_collapse_2_boundary_edge() {
        let mut mesh = make_two_triangles();
        let e_bnd = find_edge_between(&mesh, 0, 1);
        assert!(mesh.is_edge_boundary(e_bnd));

        let res = mesh
            .collapse_edge(e_bnd, 0.5)
            .expect("Collapse boundary edge");
        assert!(mesh.is_valid_vertex(res.surviving_vertex()));
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.boundary_loops().len(), 1);
        assert_eq!(mesh.boundary_loops()[0].len(), 3);
    }

    // Collapse Edge Test 3: Collapse Edge with t = 0.0
    #[test]
    fn test_collapse_3_t_zero() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, _) = mesh.edge_half_edges(e).unwrap();
        let v0 = mesh.half_edge(h0).unwrap().origin;
        let p0 = mesh.vertex(v0).unwrap().position;

        let res = mesh.collapse_edge(e, 0.0).unwrap();
        let p_res = mesh.vertex(res.surviving_vertex()).unwrap().position;
        assert!((p_res - p0).length() < 1e-5);
    }

    // Collapse Edge Test 4: Collapse Edge with t = 0.5
    #[test]
    fn test_collapse_4_t_half() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();
        let p0 = mesh
            .vertex(mesh.half_edge(h0).unwrap().origin)
            .unwrap()
            .position;
        let p1 = mesh
            .vertex(mesh.half_edge(h1).unwrap().origin)
            .unwrap()
            .position;

        let res = mesh.collapse_edge(e, 0.5).unwrap();
        let p_res = mesh.vertex(res.surviving_vertex()).unwrap().position;
        assert!((p_res - 0.5 * (p0 + p1)).length() < 1e-5);
    }

    // Collapse Edge Test 5: Collapse Edge with t = 1.0
    #[test]
    fn test_collapse_5_t_one() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let (_, h1) = mesh.edge_half_edges(e).unwrap();
        let v1 = mesh.half_edge(h1).unwrap().origin;
        let p1 = mesh.vertex(v1).unwrap().position;

        let res = mesh.collapse_edge(e, 1.0).unwrap();
        let p_res = mesh.vertex(res.surviving_vertex()).unwrap().position;
        assert!((p_res - p1).length() < 1e-5);
    }

    // Collapse Edge Test 6: Check position of resulting vertex
    #[test]
    fn test_collapse_6_position_check() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();
        let p0 = mesh
            .vertex(mesh.half_edge(h0).unwrap().origin)
            .unwrap()
            .position;
        let p1 = mesh
            .vertex(mesh.half_edge(h1).unwrap().origin)
            .unwrap()
            .position;

        let t = 0.25;
        let res = mesh.collapse_edge(e, t).unwrap();
        let p_res = mesh.vertex(res.surviving_vertex()).unwrap().position;
        let expected = p0 + (p1 - p0) * t;
        assert!((p_res - expected).length() < 1e-5);
    }

    // Collapse Edge Test 7: Interpolation of attributes
    #[test]
    fn test_collapse_7_attribute_interpolation() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();
        let v0 = mesh.half_edge(h0).unwrap().origin;
        let v1 = mesh.half_edge(h1).unwrap().origin;

        mesh.vertex_mut(v0).unwrap().normal = Some(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertex_mut(v1).unwrap().normal = Some(Vec3::new(0.0, 1.0, 0.0));

        mesh.vertex_mut(v0).unwrap().uv = Some(Vec2::new(0.1, 0.2));
        mesh.vertex_mut(v1).unwrap().uv = Some(Vec2::new(0.5, 0.8));

        mesh.vertex_mut(v0).unwrap().color = Some([1.0, 0.0, 0.0, 1.0]);
        mesh.vertex_mut(v1).unwrap().color = Some([0.0, 1.0, 0.0, 1.0]);

        let res = mesh.collapse_edge(e, 0.5).unwrap();
        let v_res = mesh.vertex(res.surviving_vertex()).unwrap();

        let n_expected = (Vec3::new(1.0, 0.0, 0.0) + Vec3::new(0.0, 1.0, 0.0)).normalize();
        assert!((v_res.normal.unwrap() - n_expected).length() < 1e-5);

        assert!((v_res.uv.unwrap() - Vec2::new(0.3, 0.5)).length() < 1e-5);

        let c = v_res.color.unwrap();
        assert!((c[0] - 0.5).abs() < 1e-5);
        assert!((c[1] - 0.5).abs() < 1e-5);
        assert!((c[2] - 0.0).abs() < 1e-5);
        assert!((c[3] - 1.0).abs() < 1e-5);
    }

    // Collapse Edge Test 8: Preservation of UV seams
    #[test]
    fn test_collapse_8_preserve_uv_seams() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);

        // Find an unaffected face that shares vertex 0, e.g. [0, 3, 4]
        let f_other = mesh
            .adjacent_faces(VertexHandle::new(0, 0))
            .into_iter()
            .find(|&f| {
                let verts = mesh.face_vertices(f);
                !verts.iter().any(|v| v.index() == 2)
            })
            .expect("unaffected face exists");

        // Set wedge UV on f_other's corner at vertex 0
        let he_in_other = mesh
            .face_half_edges(f_other)
            .into_iter()
            .find(|&h| mesh.half_edge(h).unwrap().origin.index() == 0)
            .unwrap();
        mesh.half_edge_mut(he_in_other).unwrap().uv = Some(Vec2::new(0.42, 0.84));

        let res = mesh.collapse_edge(e, 0.5).unwrap();
        assert!(mesh.is_valid_face(f_other));

        let uv_after = mesh.half_edge(he_in_other).unwrap().uv;
        assert_eq!(uv_after, Some(Vec2::new(0.42, 0.84)));
        assert!(
            res.surviving_vertex().index() == 0 || res.surviving_vertex().index() == 2,
            "Surviving vertex must be one of the collapsed edge endpoints"
        );
    }

    // Collapse Edge Test 9: Removal of degenerate faces
    #[test]
    fn test_collapse_9_remove_degenerate_faces() {
        let mut mesh = make_octahedron();
        assert_eq!(mesh.num_faces(), 8);

        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.collapse_edge(e, 0.5).unwrap();

        assert_eq!(mesh.num_faces(), 6);
        assert_eq!(res.removed_faces().len(), 2);
        for f in res.removed_faces() {
            assert!(!mesh.is_valid_face(*f));
        }
    }

    // Collapse Edge Test 10: Correctness of Twin after operation
    #[test]
    fn test_collapse_10_twin_reciprocity() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.collapse_edge(e, 0.5).unwrap();

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(
                twin.twin, h,
                "Twin reciprocity must hold for all active half-edges"
            );
        }
    }

    // Collapse Edge Test 11: Correctness of Next and Prev cycles
    #[test]
    fn test_collapse_11_next_prev_cycles() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.collapse_edge(e, 0.5).unwrap();

        for f in mesh.faces() {
            let hes = mesh.face_half_edges(f);
            assert_eq!(hes.len(), 3);
            for i in 0..hes.len() {
                let curr = hes[i];
                let next = hes[(i + 1) % hes.len()];
                let prev = hes[(i + hes.len() - 1) % hes.len()];
                assert_eq!(mesh.half_edge(curr).unwrap().next, next);
                assert_eq!(mesh.half_edge(curr).unwrap().prev, prev);
                assert_eq!(mesh.half_edge(next).unwrap().prev, curr);
                assert_eq!(mesh.half_edge(prev).unwrap().next, curr);
            }
        }
    }

    // Collapse Edge Test 12: Preservation of Boundary Loops
    #[test]
    fn test_collapse_12_boundary_loops_preservation() {
        let mut mesh = make_two_triangles();
        assert_eq!(mesh.boundary_loops().len(), 1);
        let initial_len = mesh.boundary_loops()[0].len();

        let e_bnd = find_edge_between(&mesh, 0, 1);
        let res = mesh.collapse_edge(e_bnd, 0.5).unwrap();

        let loops = mesh.boundary_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), initial_len - 1);
        assert_eq!(res.removed_faces().len(), 1);
    }

    // Collapse Edge Test 13: Check active counters
    #[test]
    fn test_collapse_13_active_counters() {
        let mut mesh = make_octahedron();
        let v0 = mesh.num_vertices();
        let e0 = mesh.num_edges();
        let f0 = mesh.num_faces();
        let he0 = mesh.half_edges().count();

        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.collapse_edge(e, 0.5).unwrap();

        assert_eq!(mesh.num_vertices(), v0 - 1);
        assert_eq!(mesh.num_edges(), e0 - 3);
        assert_eq!(mesh.num_faces(), f0 - 2);
        assert_eq!(mesh.half_edges().count(), he0 - 6);
        assert_eq!(res.removed_elements_count(), 1 + 3 + 6 + 2);
    }

    // Collapse Edge Test 14: Check generation and invalidation of removed handles
    #[test]
    fn test_collapse_14_generation_handles() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.collapse_edge(e, 0.5).unwrap();

        assert!(mesh.is_valid_vertex(res.surviving_vertex()));
        assert!(!mesh.is_valid_vertex(res.removed_vertex()));
        assert!(!mesh.is_valid_edge(res.collapsed_edge()));

        for eh in res.removed_edges() {
            assert!(!mesh.is_valid_edge(*eh));
        }
        for fh in res.removed_faces() {
            assert!(!mesh.is_valid_face(*fh));
        }
        for heh in res.removed_half_edges() {
            assert!(!mesh.is_valid_half_edge(*heh));
        }
    }

    // Collapse Edge Test 15: Check absence of duplicate edges
    #[test]
    fn test_collapse_15_no_duplicate_edges() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        mesh.collapse_edge(e, 0.5).unwrap();

        let mut seen_pairs = HashSet::new();
        for eh in mesh.edges() {
            let (va, vb) = mesh.edge_vertices(eh).unwrap();
            let pair = if va.index() < vb.index() {
                (va.index(), vb.index())
            } else {
                (vb.index(), va.index())
            };
            assert!(
                seen_pairs.insert(pair),
                "Duplicate edge detected between vertices {:?}",
                pair
            );
        }
    }

    // Collapse Edge Test 16: Link Condition and pinching rejection
    #[test]
    fn test_collapse_16_link_condition_rejection() {
        // 1. Single triangle: collapsing any edge rejected
        let positions_single = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces_single = vec![vec![0, 1, 2]];
        let mut mesh_single = TopologyBuilder::build(&positions_single, &faces_single).unwrap();
        let e_single = mesh_single.edges().next().unwrap();
        assert!(matches!(
            mesh_single.collapse_edge(e_single, 0.5),
            Err(TopologyError::InvalidOperation(_))
        ));

        // 2. Pinching check: interior edge connecting two boundary vertices
        let mut mesh_two_tri = make_two_triangles();
        let e_diag = find_edge_between(&mesh_two_tri, 0, 2);
        assert!(!mesh_two_tri.is_edge_boundary(e_diag));
        assert!(matches!(
            mesh_two_tri.collapse_edge(e_diag, 0.5),
            Err(TopologyError::InvalidOperation(_))
        ));

        // 3. Tetrahedron: collapsing edge in closed mesh with <= 4 vertices rejected
        let mut mesh_tet = make_tetrahedron();
        let e_tet = mesh_tet.edges().next().unwrap();
        assert!(matches!(
            mesh_tet.collapse_edge(e_tet, 0.5),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Collapse Edge Test 17: Handling of invalid EdgeHandle
    #[test]
    fn test_collapse_17_invalid_edge_handle() {
        let mut mesh = make_octahedron();

        let res_null = mesh.collapse_edge(EdgeHandle::NULL, 0.5);
        assert!(matches!(res_null, Err(TopologyError::InvalidEdge(_))));

        let res_oob = mesh.collapse_edge(EdgeHandle::new(9999, 0), 0.5);
        assert!(matches!(res_oob, Err(TopologyError::InvalidEdge(_))));

        let valid_e = mesh.edges().next().unwrap();
        let wrong_gen = EdgeHandle::new(valid_e.raw_index(), valid_e.generation() + 1);
        let res_gen = mesh.collapse_edge(wrong_gen, 0.5);
        assert!(matches!(res_gen, Err(TopologyError::InvalidEdge(_))));
    }

    // Collapse Edge Test 18: Handling of invalid parameter t
    #[test]
    fn test_collapse_18_invalid_t_values() {
        let mut mesh = make_octahedron();
        let e = mesh.edges().next().unwrap();

        for &bad_t in &[
            -0.1,
            1.1,
            -10.0,
            2.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let res = mesh.collapse_edge(e, bad_t);
            assert!(
                matches!(res, Err(TopologyError::InvalidOperation(_))),
                "bad t ({bad_t}) must return InvalidOperation"
            );
        }
    }

    // Collapse Edge Test 19: Final mesh validity via TopologyValidator and check_invariants
    #[test]
    fn test_collapse_19_final_mesh_validity() {
        let mut mesh = make_octahedron();
        let e = find_edge_between(&mesh, 0, 2);
        let res = mesh.collapse_edge(e, 0.5);
        assert!(res.is_ok());

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    // Collapse Edge Test 20: Atomicity check on error (mesh unchanged if collapse fails)
    #[test]
    fn test_collapse_20_atomic_rollback() {
        let mut mesh = make_octahedron();
        let mesh_backup = mesh.clone();

        // 1. Error on invalid handle
        assert!(mesh.collapse_edge(EdgeHandle::NULL, 0.5).is_err());
        assert_eq!(mesh, mesh_backup);

        // 2. Error on invalid t
        let e = find_edge_between(&mesh, 0, 2);
        assert!(mesh.collapse_edge(e, -1.0).is_err());
        assert_eq!(mesh, mesh_backup);

        // 3. Topology must remain completely intact
        assert!(mesh.check_invariants().is_ok());
    }

    // =========================================================================
    // Phase 6.9: Split Face Unit Tests
    // =========================================================================

    fn make_single_quad() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(2.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 2.0, 0.0), // 2
            Vec3::new(0.0, 2.0, 0.0), // 3
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    fn make_pentagon() -> HalfEdgeMesh {
        let mut positions = Vec::new();
        for i in 0..5 {
            let angle = std::f32::consts::TAU * (i as f32) / 5.0;
            positions.push(Vec3::new(angle.cos(), angle.sin(), 0.0));
        }
        let faces = vec![vec![0, 1, 2, 3, 4]];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    fn make_hexagon() -> HalfEdgeMesh {
        let mut positions = Vec::new();
        for i in 0..6 {
            let angle = std::f32::consts::TAU * (i as f32) / 6.0;
            positions.push(Vec3::new(angle.cos(), angle.sin(), 0.0));
        }
        let faces = vec![vec![0, 1, 2, 3, 4, 5]];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Split Face Test 1: Quad split into two triangles
    #[test]
    fn test_split_face_1_quad_to_triangles() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2).expect("split quad");
        assert_eq!(res.face1_vertices().len(), 3);
        assert_eq!(res.face2_vertices().len(), 3);
        assert_eq!(mesh.num_faces(), 2);
    }

    // Split Face Test 2: Pentagon split into triangle and quad
    #[test]
    fn test_split_face_2_pentagon_to_triangle_and_quad() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2).expect("split pentagon");
        assert_eq!(res.face1_vertices().len(), 3);
        assert_eq!(res.face2_vertices().len(), 4);
        assert_eq!(mesh.num_faces(), 2);
        assert_eq!(mesh.num_edges(), 6);
    }

    // Split Face Test 3: Hexagon split into two quads
    #[test]
    fn test_split_face_3_hexagon_to_two_quads() {
        let mut mesh = make_hexagon();
        let f = mesh.faces().next().unwrap();
        let v1 = VertexHandle::new(1, 0);
        let v4 = VertexHandle::new(4, 0);

        let res = mesh.split_face(f, v1, v4).expect("split hexagon");
        assert_eq!(res.face1_vertices().len(), 4);
        assert_eq!(res.face2_vertices().len(), 4);
        assert_eq!(mesh.num_faces(), 2);
        assert_eq!(mesh.num_edges(), 7);
    }

    // Split Face Test 4: Check new Edge created, not boundary
    #[test]
    fn test_split_face_4_new_edge_created_and_not_boundary() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2).expect("split quad");
        let e_new = res.new_edge();
        assert!(mesh.is_valid_edge(e_new));
        assert!(!mesh.is_edge_boundary(e_new));

        let (h0, h1) = mesh.edge_half_edges(e_new).unwrap();
        assert_eq!((h0, h1), res.new_half_edges());

        let f0 = mesh.half_edge(h0).unwrap().face.unwrap();
        let f1 = mesh.half_edge(h1).unwrap().face.unwrap();
        assert!(
            (f0 == res.original_face() && f1 == res.new_face())
                || (f0 == res.new_face() && f1 == res.original_face())
        );
    }

    // Split Face Test 5: Check two new Face Loops closed and valid
    #[test]
    fn test_split_face_5_face_loops_closed_and_valid() {
        let mut mesh = make_hexagon();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v3 = VertexHandle::new(3, 0);

        let res = mesh.split_face(f, v0, v3).expect("split hexagon");
        let loop1 = mesh.face_loop_safe(res.original_face()).unwrap();
        let loop2 = mesh.face_loop_safe(res.new_face()).unwrap();

        assert_eq!(loop1.len(), 4);
        assert_eq!(loop2.len(), 4);

        for (i, &he) in loop1.iter().enumerate() {
            let next_expected = loop1[(i + 1) % loop1.len()];
            assert_eq!(mesh.half_edge(he).unwrap().next, next_expected);
        }
        for (i, &he) in loop2.iter().enumerate() {
            let next_expected = loop2[(i + 1) % loop2.len()];
            assert_eq!(mesh.half_edge(he).unwrap().next, next_expected);
        }
    }

    // Split Face Test 6: Check Twin reciprocity between new half-edges
    #[test]
    fn test_split_face_6_twin_reciprocity() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2).expect("split quad");
        let (h_ab, h_ba) = res.new_half_edges();

        assert_eq!(mesh.half_edge(h_ab).unwrap().twin, h_ba);
        assert_eq!(mesh.half_edge(h_ba).unwrap().twin, h_ab);
        assert_eq!(mesh.half_edge_twin(h_ab), Some(h_ba));
        assert_eq!(mesh.half_edge_twin(h_ba), Some(h_ab));
    }

    // Split Face Test 7: Check Next and Prev consistency on all half-edges in both cycles
    #[test]
    fn test_split_face_7_next_and_prev_consistency() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2).expect("split pentagon");

        for &face_h in &[res.original_face(), res.new_face()] {
            for he_h in mesh.face_half_edges(face_h) {
                let he = mesh.half_edge(he_h).unwrap();
                let next_he = mesh.half_edge(he.next).unwrap();
                let prev_he = mesh.half_edge(he.prev).unwrap();

                assert_eq!(next_he.prev, he_h);
                assert_eq!(prev_he.next, he_h);
            }
        }
    }

    // Split Face Test 8: Check Face references on all half-edges in both cycles
    #[test]
    fn test_split_face_8_face_references() {
        let mut mesh = make_hexagon();
        let f = mesh.faces().next().unwrap();
        let v1 = VertexHandle::new(1, 0);
        let v4 = VertexHandle::new(4, 0);

        let res = mesh.split_face(f, v1, v4).expect("split hexagon");

        for he_h in mesh.face_half_edges(res.original_face()) {
            assert_eq!(
                mesh.half_edge(he_h).unwrap().face,
                Some(res.original_face())
            );
        }
        for he_h in mesh.face_half_edges(res.new_face()) {
            assert_eq!(mesh.half_edge(he_h).unwrap().face, Some(res.new_face()));
        }
    }

    // Split Face Test 9: Check vertex count preserved
    #[test]
    fn test_split_face_9_vertex_count_preserved() {
        let mut mesh = make_hexagon();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v3 = VertexHandle::new(3, 0);

        let v_count_before = mesh.num_vertices();
        mesh.split_face(f, v0, v3).expect("split hexagon");
        assert_eq!(mesh.num_vertices(), v_count_before);
    }

    // Split Face Test 10: Check edge count (+1) and face count (+1)
    #[test]
    fn test_split_face_10_edge_and_face_count() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let e_before = mesh.num_edges();
        let f_before = mesh.num_faces();
        let he_before = mesh.num_half_edges();

        mesh.split_face(f, v0, v2).expect("split quad");

        assert_eq!(mesh.num_edges(), e_before + 1);
        assert_eq!(mesh.num_faces(), f_before + 1);
        assert_eq!(mesh.num_half_edges(), he_before + 2);
    }

    // Split Face Test 11: Check vertex coordinates preserved
    #[test]
    fn test_split_face_11_vertex_coordinates_preserved() {
        let mut mesh = make_single_quad();
        let positions_before: Vec<Vec3> = mesh
            .vertices()
            .map(|v| mesh.vertex(v).unwrap().position)
            .collect();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        mesh.split_face(f, v0, v2).expect("split quad");

        let positions_after: Vec<Vec3> = mesh
            .vertices()
            .map(|v| mesh.vertex(v).unwrap().position)
            .collect();
        assert_eq!(positions_before, positions_after);
    }

    // Split Face Test 12: Check preservation of UV seams
    #[test]
    fn test_split_face_12_preserve_uv_seams() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let f_hes = mesh.face_half_edges(f);

        // Assign distinct UV coordinates to each corner
        mesh.half_edge_mut(f_hes[0]).unwrap().uv = Some(Vec2::new(0.0, 0.0));
        mesh.half_edge_mut(f_hes[1]).unwrap().uv = Some(Vec2::new(1.0, 0.0));
        mesh.half_edge_mut(f_hes[2]).unwrap().uv = Some(Vec2::new(1.0, 1.0));
        mesh.half_edge_mut(f_hes[3]).unwrap().uv = Some(Vec2::new(0.0, 1.0));

        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);
        let res = mesh.split_face(f, v0, v2).expect("split quad");

        // Existing half-edges must retain their original corner UVs
        assert_eq!(
            mesh.half_edge(f_hes[0]).unwrap().uv,
            Some(Vec2::new(0.0, 0.0))
        );
        assert_eq!(
            mesh.half_edge(f_hes[1]).unwrap().uv,
            Some(Vec2::new(1.0, 0.0))
        );
        assert_eq!(
            mesh.half_edge(f_hes[2]).unwrap().uv,
            Some(Vec2::new(1.0, 1.0))
        );
        assert_eq!(
            mesh.half_edge(f_hes[3]).unwrap().uv,
            Some(Vec2::new(0.0, 1.0))
        );

        // New half-edges inherit corresponding origin corner UVs
        let (h_ab, h_ba) = res.new_half_edges();
        assert_eq!(mesh.half_edge(h_ab).unwrap().uv, Some(Vec2::new(0.0, 0.0)));
        assert_eq!(mesh.half_edge(h_ba).unwrap().uv, Some(Vec2::new(1.0, 1.0)));
    }

    // Split Face Test 13: Check preservation/copy of material_id
    #[test]
    fn test_split_face_13_preserve_material_id() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.face_mut(f).unwrap().material_id = Some(42);

        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);
        let res = mesh.split_face(f, v0, v2).expect("split quad");

        assert_eq!(
            mesh.face(res.original_face()).unwrap().material_id,
            Some(42)
        );
        assert_eq!(mesh.face(res.new_face()).unwrap().material_id, Some(42));
    }

    // Split Face Test 14: Check Boundary Loops unaffected
    #[test]
    fn test_split_face_14_boundary_loops_unaffected() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let bnd_before = mesh.boundary_loops();
        let bnd_he_count_before = mesh.boundary_half_edges().len();

        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);
        mesh.split_face(f, v0, v2).expect("split quad");

        let bnd_after = mesh.boundary_loops();
        let bnd_he_count_after = mesh.boundary_half_edges().len();

        assert_eq!(bnd_after.len(), bnd_before.len());
        assert_eq!(bnd_he_count_after, bnd_he_count_before);
    }

    // Split Face Test 15: Error on identical vertices (vertex_a == vertex_b)
    #[test]
    fn test_split_face_15_error_on_identical_vertices() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);

        let res = mesh.split_face(f, v0, v0);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Split Face Test 16: Error on adjacent vertices
    #[test]
    fn test_split_face_16_error_on_adjacent_vertices() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v1 = VertexHandle::new(1, 0);
        let v3 = VertexHandle::new(3, 0);

        assert!(matches!(
            mesh.split_face(f, v0, v1),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.split_face(f, v0, v3),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Split Face Test 17: Error on vertex not in face
    #[test]
    fn test_split_face_17_error_on_vertex_not_in_face() {
        let mut mesh = make_two_quads();
        let f0 = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0); // v2 belongs to face 1, not face 0

        let res = mesh.split_face(f0, v0, v2);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Split Face Test 18: Error on invalid FaceHandle or VertexHandle
    #[test]
    fn test_split_face_18_error_on_invalid_face_or_vertex() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        assert!(matches!(
            mesh.split_face(FaceHandle::NULL, v0, v2),
            Err(TopologyError::InvalidFace(_))
        ));
        assert!(matches!(
            mesh.split_face(f, VertexHandle::NULL, v2),
            Err(TopologyError::InvalidVertex(_))
        ));
        assert!(matches!(
            mesh.split_face(f, v0, VertexHandle::NULL),
            Err(TopologyError::InvalidVertex(_))
        ));
    }

    // Split Face Test 19: Error on degenerate/collinear resulting face
    #[test]
    fn test_split_face_19_error_on_degenerate_result() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 0.0, 0.0), // 2 (collinear with 0 and 1)
            Vec3::new(1.0, 1.0, 0.0), // 3
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v2 = VertexHandle::new(2, 0);

        let res = mesh.split_face(f, v0, v2);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Split Face Test 20: Atomicity check on error
    #[test]
    fn test_split_face_20_atomic_rollback_on_error() {
        let mut mesh = make_single_quad();
        let backup = mesh.clone();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v1 = VertexHandle::new(1, 0);

        // 1. Invalid face handle
        assert!(mesh.split_face(FaceHandle::NULL, v0, v1).is_err());
        assert_eq!(mesh, backup);

        // 2. Invalid vertex handle
        assert!(mesh.split_face(f, VertexHandle::NULL, v0).is_err());
        assert_eq!(mesh, backup);

        // 3. Identical vertices
        assert!(mesh.split_face(f, v0, v0).is_err());
        assert_eq!(mesh, backup);

        // 4. Adjacent vertices
        assert!(mesh.split_face(f, v0, v1).is_err());
        assert_eq!(mesh, backup);

        // Mesh remains valid
        assert!(mesh.check_invariants().is_ok());
    }

    // Split Face Test 21: Full validation with TopologyValidator and check_invariants
    #[test]
    fn test_split_face_21_topology_validator_and_invariants() {
        let mut mesh = make_hexagon();
        let f = mesh.faces().next().unwrap();
        let v0 = VertexHandle::new(0, 0);
        let v3 = VertexHandle::new(3, 0);

        let res = mesh
            .split_face(f, v0, v3)
            .expect("split hexagon into 2 quads");

        // Further split one of the newly formed quads
        let v1 = VertexHandle::new(1, 0);
        let res2 = mesh.split_face(res.original_face(), v1, v3);
        assert!(res2.is_ok());

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    // =========================================================================
    // Phase 6.10: Dissolve Edge (Merge Faces) Unit Tests
    // =========================================================================

    fn make_triangle_and_quad() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 1.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(2.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 3.0, 0.0), // 3
            Vec3::new(2.0, 3.0, 0.0), // 4
        ];
        let faces = vec![
            vec![0, 1, 2],    // Triangle 0: 0->1->2
            vec![0, 2, 4, 3], // Quad 1: 0->2->4->3
        ];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Dissolve Edge Test 1: Merging two triangles into a quad
    #[test]
    fn test_dissolve_1_two_triangles_to_quad() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).expect("dissolve edge");
        assert_eq!(res.boundary_vertices().len(), 4);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_edges(), 4);
    }

    // Dissolve Edge Test 2: Merging a triangle and a quad into a pentagon
    #[test]
    fn test_dissolve_2_triangle_and_quad_to_pentagon() {
        let mut mesh = make_triangle_and_quad();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).expect("dissolve edge");
        assert_eq!(res.boundary_vertices().len(), 5);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.num_edges(), 5);
    }

    // Dissolve Edge Test 3: Correct merged face loop
    #[test]
    fn test_dissolve_3_correct_merged_face_loop() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).unwrap();
        let loop_hes = mesh.face_loop_safe(res.surviving_face()).unwrap();
        assert_eq!(loop_hes.len(), 4);

        for (i, &he) in loop_hes.iter().enumerate() {
            let next_expected = loop_hes[(i + 1) % loop_hes.len()];
            assert_eq!(mesh.half_edge(he).unwrap().next, next_expected);
        }
    }

    // Dissolve Edge Test 4: Correct half-edge next/prev links
    #[test]
    fn test_dissolve_4_correct_next_prev_links() {
        let mut mesh = make_triangle_and_quad();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).unwrap();

        for he_h in mesh.face_half_edges(res.surviving_face()) {
            let he = mesh.half_edge(he_h).unwrap();
            let next_he = mesh.half_edge(he.next).unwrap();
            let prev_he = mesh.half_edge(he.prev).unwrap();

            assert_eq!(next_he.prev, he_h);
            assert_eq!(prev_he.next, he_h);
        }
    }

    // Dissolve Edge Test 5: Correct face references
    #[test]
    fn test_dissolve_5_correct_face_references() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).unwrap();

        for he_h in mesh.face_half_edges(res.surviving_face()) {
            assert_eq!(
                mesh.half_edge(he_h).unwrap().face,
                Some(res.surviving_face())
            );
        }
    }

    // Dissolve Edge Test 6: Deletion of the shared edge and half-edges
    #[test]
    fn test_dissolve_6_deletion_of_shared_edge_and_half_edges() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();

        let res = mesh.dissolve_edge(e).unwrap();

        assert!(!mesh.is_valid_edge(res.dissolved_edge()));
        assert!(!mesh.is_valid_edge(e));
        assert!(!mesh.is_valid_half_edge(h0));
        assert!(!mesh.is_valid_half_edge(h1));
    }

    // Dissolve Edge Test 7: Deletion of the removed face
    #[test]
    fn test_dissolve_7_deletion_of_removed_face() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).unwrap();

        assert!(!mesh.is_valid_face(res.removed_face()));
        assert!(mesh.is_valid_face(res.surviving_face()));
    }

    // Dissolve Edge Test 8: Active element counters
    #[test]
    fn test_dissolve_8_active_element_counters() {
        let mut mesh = make_two_triangles();
        let v0 = mesh.num_vertices();
        let e0 = mesh.num_edges();
        let f0 = mesh.num_faces();
        let he0 = mesh.num_half_edges();

        let e = find_edge_between(&mesh, 0, 2);
        mesh.dissolve_edge(e).unwrap();

        assert_eq!(mesh.num_vertices(), v0);
        assert_eq!(mesh.num_edges(), e0 - 1);
        assert_eq!(mesh.num_faces(), f0 - 1);
        assert_eq!(mesh.num_half_edges(), he0 - 2);
    }

    // Dissolve Edge Test 9: Preservation of UV seams and corner normals
    #[test]
    fn test_dissolve_9_preservation_of_uv_seams_and_corner_normals() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();

        // Assign distinct UVs to all half-edges that will survive
        for he_h in mesh.half_edges().collect::<Vec<_>>() {
            if he_h != h0 && he_h != h1 {
                let idx = he_h.index() as f32;
                mesh.half_edge_mut(he_h).unwrap().uv = Some(Vec2::new(idx, idx * 0.5));
                mesh.half_edge_mut(he_h).unwrap().normal = Some(Vec3::new(0.0, 0.0, 1.0));
            }
        }

        let res = mesh.dissolve_edge(e).unwrap();

        for he_h in mesh.face_half_edges(res.surviving_face()) {
            let idx = he_h.index() as f32;
            assert_eq!(
                mesh.half_edge(he_h).unwrap().uv,
                Some(Vec2::new(idx, idx * 0.5))
            );
            assert_eq!(
                mesh.half_edge(he_h).unwrap().normal,
                Some(Vec3::new(0.0, 0.0, 1.0))
            );
        }
    }

    // Dissolve Edge Test 10: Preservation of vertex attributes
    #[test]
    fn test_dissolve_10_preservation_of_vertex_attributes() {
        let mut mesh = make_two_triangles();
        let positions_before: Vec<Vec3> = mesh
            .vertices()
            .map(|v| mesh.vertex(v).unwrap().position)
            .collect();

        let e = find_edge_between(&mesh, 0, 2);
        mesh.dissolve_edge(e).unwrap();

        let positions_after: Vec<Vec3> = mesh
            .vertices()
            .map(|v| mesh.vertex(v).unwrap().position)
            .collect();
        assert_eq!(positions_before, positions_after);
    }

    // Dissolve Edge Test 11: Deterministic material behavior
    #[test]
    fn test_dissolve_11_deterministic_material_behavior() {
        let mut mesh = make_two_triangles();
        let e = find_edge_between(&mesh, 0, 2);
        let (h0, h1) = mesh.edge_half_edges(e).unwrap();
        let f0 = mesh.half_edge(h0).unwrap().face.unwrap();
        let f1 = mesh.half_edge(h1).unwrap().face.unwrap();

        mesh.face_mut(f0).unwrap().material_id = Some(111);
        mesh.face_mut(f1).unwrap().material_id = Some(222);

        let res = mesh.dissolve_edge(e).unwrap();
        assert_eq!(res.surviving_face(), f0);
        assert_eq!(
            mesh.face(res.surviving_face()).unwrap().material_id,
            Some(111)
        );
    }

    // Dissolve Edge Test 12: Rejection of boundary edges
    #[test]
    fn test_dissolve_12_rejection_of_boundary_edges() {
        let mut mesh = make_two_triangles();
        let e_bnd = find_edge_between(&mesh, 0, 1);
        assert!(mesh.is_edge_boundary(e_bnd));

        let res = mesh.dissolve_edge(e_bnd);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Dissolve Edge Test 13: Rejection of invalid handles
    #[test]
    fn test_dissolve_13_rejection_of_invalid_handles() {
        let mut mesh = make_two_triangles();
        let res = mesh.dissolve_edge(EdgeHandle::NULL);
        assert!(matches!(res, Err(TopologyError::InvalidEdge(_))));
    }

    // Dissolve Edge Test 14: Rejection of degenerate / folding results
    #[test]
    fn test_dissolve_14_rejection_of_degenerate_results() {
        // Two triangles folded back-to-back (normal dot product < 0)
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(2.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(1.0, 0.2, 0.0), // 3 (folds backwards inside triangle 0)
        ];
        let faces = vec![
            vec![0, 1, 2], // normal +Z
            vec![0, 2, 3], // normal -Z (folded backwards)
        ];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e);
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Dissolve Edge Test 15: Atomic rollback on failure
    #[test]
    fn test_dissolve_15_atomic_rollback_on_failure() {
        let mut mesh = make_two_triangles();
        let backup = mesh.clone();

        // 1. Invalid edge
        assert!(mesh.dissolve_edge(EdgeHandle::NULL).is_err());
        assert_eq!(mesh, backup);

        // 2. Boundary edge
        let e_bnd = find_edge_between(&mesh, 0, 1);
        assert!(mesh.dissolve_edge(e_bnd).is_err());
        assert_eq!(mesh, backup);

        assert!(mesh.check_invariants().is_ok());
    }

    // Dissolve Edge Test 16: Successful validation with TopologyValidator and invariants
    #[test]
    fn test_dissolve_16_topology_validator_and_invariants() {
        let mut mesh = make_triangle_and_quad();
        let e = find_edge_between(&mesh, 0, 2);

        let res = mesh.dissolve_edge(e).unwrap();
        assert!(mesh.is_valid_face(res.surviving_face()));

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    // =========================================================================
    // Phase 6.11: Poke Face Unit Tests
    // =========================================================================

    // Poke Face Test 1: Poking a convex quad into four triangles
    #[test]
    fn test_poke_1_convex_quad_to_four_triangles() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh
            .poke_face(f, Vec3::new(1.0, 1.0, 0.0))
            .expect("poke quad");

        assert_eq!(mesh.num_faces(), 4);
        assert_eq!(res.new_faces().len(), 3);
        assert!(mesh.is_valid_face(res.original_face()));
        assert_eq!(mesh.face_vertices(res.original_face()).len(), 3);
        for &nf in res.new_faces() {
            assert_eq!(mesh.face_vertices(nf).len(), 3);
        }
    }

    // Poke Face Test 2: Poking a convex pentagon into five triangles
    #[test]
    fn test_poke_2_convex_pentagon_to_five_triangles() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::ZERO).expect("poke pentagon");

        assert_eq!(mesh.num_faces(), 5);
        assert_eq!(res.new_faces().len(), 4);
        assert_eq!(mesh.face_vertices(res.original_face()).len(), 3);
        for &nf in res.new_faces() {
            assert_eq!(mesh.face_vertices(nf).len(), 3);
        }
    }

    // Poke Face Test 3: Correct element counts (V+1, E+N, F+N-1, HE+2N)
    #[test]
    fn test_poke_3_element_counts() {
        let mut mesh = make_single_quad();
        let v0 = mesh.num_vertices();
        let e0 = mesh.num_edges();
        let f0 = mesh.num_faces();
        let he0 = mesh.num_half_edges();

        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        assert_eq!(mesh.num_vertices(), v0 + 1);
        assert_eq!(mesh.num_edges(), e0 + 4);
        assert_eq!(mesh.num_faces(), f0 + 3);
        assert_eq!(mesh.num_half_edges(), he0 + 8);
        assert_eq!(res.new_edges().len(), 4);
        assert_eq!(res.new_half_edges().len(), 8);
    }

    // Poke Face Test 4: Correct triangle loops and face references
    #[test]
    fn test_poke_4_triangle_loops_and_face_references() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        let mut all_faces = vec![res.original_face()];
        all_faces.extend_from_slice(res.new_faces());

        for &fh in &all_faces {
            let loop_hes = mesh.face_loop_safe(fh).unwrap();
            assert_eq!(loop_hes.len(), 3);
            for &he_h in &loop_hes {
                assert_eq!(mesh.half_edge(he_h).unwrap().face, Some(fh));
            }
        }
    }

    // Poke Face Test 5: Reciprocal twin relationships
    #[test]
    fn test_poke_5_reciprocal_twin_relationships() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        for &he_h in res.new_half_edges() {
            let twin_h = mesh.half_edge(he_h).unwrap().twin;
            assert_eq!(mesh.half_edge(twin_h).unwrap().twin, he_h);
        }
    }

    // Poke Face Test 6: Consistent next/prev links
    #[test]
    fn test_poke_6_consistent_next_prev_links() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::ZERO).unwrap();

        let mut all_faces = vec![res.original_face()];
        all_faces.extend_from_slice(res.new_faces());

        for &fh in &all_faces {
            for he_h in mesh.face_half_edges(fh) {
                let he = mesh.half_edge(he_h).unwrap();
                let next_he = mesh.half_edge(he.next).unwrap();
                let prev_he = mesh.half_edge(he.prev).unwrap();

                assert_eq!(next_he.prev, he_h);
                assert_eq!(prev_he.next, he_h);
            }
        }
    }

    // Poke Face Test 7: Correct vertex outgoing references
    #[test]
    fn test_poke_7_vertex_outgoing_references() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        let vc = res.new_vertex();
        let out_vc = mesh.vertex(vc).unwrap().half_edge.unwrap();
        assert_eq!(mesh.half_edge(out_vc).unwrap().origin, vc);

        for vh in mesh.vertices() {
            if let Some(out_he) = mesh.vertex(vh).unwrap().half_edge {
                assert_eq!(mesh.half_edge(out_he).unwrap().origin, vh);
            }
        }
    }

    // Poke Face Test 8: Preservation of original boundary vertex attributes
    #[test]
    fn test_poke_8_preservation_of_boundary_vertex_attributes() {
        let mut mesh = make_single_quad();
        let verts: Vec<_> = mesh.vertices().collect();
        mesh.vertex_mut(verts[0]).unwrap().uv = Some(Vec2::new(0.0, 0.0));
        mesh.vertex_mut(verts[1]).unwrap().uv = Some(Vec2::new(1.0, 0.0));
        mesh.vertex_mut(verts[2]).unwrap().uv = Some(Vec2::new(1.0, 1.0));
        mesh.vertex_mut(verts[3]).unwrap().uv = Some(Vec2::new(0.0, 1.0));

        let f = mesh.faces().next().unwrap();
        mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        assert_eq!(mesh.vertex(verts[0]).unwrap().uv, Some(Vec2::new(0.0, 0.0)));
        assert_eq!(mesh.vertex(verts[1]).unwrap().uv, Some(Vec2::new(1.0, 0.0)));
        assert_eq!(mesh.vertex(verts[2]).unwrap().uv, Some(Vec2::new(1.0, 1.0)));
        assert_eq!(mesh.vertex(verts[3]).unwrap().uv, Some(Vec2::new(0.0, 1.0)));
    }

    // Poke Face Test 9: New vertex attribute initialization
    #[test]
    fn test_poke_9_new_vertex_attribute_initialization() {
        let mut mesh = make_single_quad();
        let verts: Vec<_> = mesh.vertices().collect();
        mesh.vertex_mut(verts[0]).unwrap().uv = Some(Vec2::new(0.0, 0.0));
        mesh.vertex_mut(verts[1]).unwrap().uv = Some(Vec2::new(2.0, 0.0));
        mesh.vertex_mut(verts[2]).unwrap().uv = Some(Vec2::new(2.0, 2.0));
        mesh.vertex_mut(verts[3]).unwrap().uv = Some(Vec2::new(0.0, 2.0));

        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();

        let vc = res.new_vertex();
        assert_eq!(mesh.vertex(vc).unwrap().position, Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(
            mesh.vertex(vc).unwrap().normal,
            Some(Vec3::new(0.0, 0.0, 1.0))
        );
        assert_eq!(mesh.vertex(vc).unwrap().uv, Some(Vec2::new(1.0, 1.0)));
    }

    // Poke Face Test 10: Material inheritance
    #[test]
    fn test_poke_10_material_inheritance() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.face_mut(f).unwrap().material_id = Some(888);

        let res = mesh.poke_face(f, Vec3::new(1.0, 1.0, 0.0)).unwrap();
        assert_eq!(
            mesh.face(res.original_face()).unwrap().material_id,
            Some(888)
        );
        for &nf in res.new_faces() {
            assert_eq!(mesh.face(nf).unwrap().material_id, Some(888));
        }
    }

    // Poke Face Test 11: Rejection of invalid handles
    #[test]
    fn test_poke_11_rejection_of_invalid_handles() {
        let mut mesh = make_single_quad();
        let res = mesh.poke_face(FaceHandle::NULL, Vec3::new(1.0, 1.0, 0.0));
        assert!(matches!(res, Err(TopologyError::InvalidFace(_))));
    }

    // Poke Face Test 12: Rejection of non-finite positions
    #[test]
    fn test_poke_12_rejection_of_non_finite_positions() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        assert!(matches!(
            mesh.poke_face(f, Vec3::new(f32::NAN, 1.0, 0.0)),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.poke_face(f, Vec3::new(1.0, f32::INFINITY, 0.0)),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Poke Face Test 13: Rejection of concave and non-planar polygons
    #[test]
    fn test_poke_13_rejection_of_concave_and_non_planar_polygons() {
        // Concave quad (reflex angle at vertex 2)
        let positions_concave = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(2.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 0.2, 0.0), // 2 (indented)
            Vec3::new(1.0, 2.0, 0.0), // 3
        ];
        let faces_concave = vec![vec![0, 1, 2, 3]];
        let mut mesh_concave = TopologyBuilder::build(&positions_concave, &faces_concave).unwrap();
        let f_concave = mesh_concave.faces().next().unwrap();

        let res_concave = mesh_concave.poke_face(f_concave, Vec3::new(1.0, 0.5, 0.0));
        assert!(matches!(
            res_concave,
            Err(TopologyError::InvalidOperation(_))
        ));

        // Non-coplanar position
        let mut mesh_quad = make_single_quad();
        let f_quad = mesh_quad.faces().next().unwrap();
        let res_nonplanar = mesh_quad.poke_face(f_quad, Vec3::new(1.0, 1.0, 1.0));
        assert!(matches!(
            res_nonplanar,
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Poke Face Test 14: Rejection of points outside or on boundary
    #[test]
    fn test_poke_14_rejection_of_points_outside_or_on_boundary() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        // Point outside
        assert!(matches!(
            mesh.poke_face(f, Vec3::new(5.0, 5.0, 0.0)),
            Err(TopologyError::InvalidOperation(_))
        ));

        // Point on boundary edge
        assert!(matches!(
            mesh.poke_face(f, Vec3::new(1.0, 0.0, 0.0)),
            Err(TopologyError::InvalidOperation(_))
        ));

        // Point on vertex
        assert!(matches!(
            mesh.poke_face(f, Vec3::new(0.0, 0.0, 0.0)),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Poke Face Test 15: Rejection of degenerate triangles
    #[test]
    fn test_poke_15_rejection_of_degenerate_triangles() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        // Point virtually on boundary edge
        let res = mesh.poke_face(f, Vec3::new(1.0, 1e-7, 0.0));
        assert!(matches!(res, Err(TopologyError::InvalidOperation(_))));
    }

    // Poke Face Test 16: Atomic rollback on failure
    #[test]
    fn test_poke_16_atomic_rollback_on_failure() {
        let mut mesh = make_single_quad();
        let backup = mesh.clone();
        let f = mesh.faces().next().unwrap();

        // 1. Invalid face handle
        assert!(
            mesh.poke_face(FaceHandle::NULL, Vec3::new(1.0, 1.0, 0.0))
                .is_err()
        );
        assert_eq!(mesh, backup);

        // 2. Point outside
        assert!(mesh.poke_face(f, Vec3::new(10.0, 10.0, 0.0)).is_err());
        assert_eq!(mesh, backup);

        // 3. Point on boundary
        assert!(mesh.poke_face(f, Vec3::new(0.0, 0.0, 0.0)).is_err());
        assert_eq!(mesh, backup);

        assert!(mesh.check_invariants().is_ok());
    }

    // Poke Face Test 17: Successful TopologyValidator and invariant checks
    #[test]
    fn test_poke_17_topology_validator_and_invariants() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();
        let res = mesh.poke_face(f, Vec3::ZERO).unwrap();
        assert!(mesh.is_valid_vertex(res.new_vertex()));

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid, "Report must be valid: {:?}", report.issues);
        assert_eq!(report.error_count(), 0);
        assert!(mesh.check_invariants().is_ok());
    }

    // =========================================================================
    // Phase 6.12: Basic Face Extrusion Unit Tests
    // =========================================================================

    fn make_single_triangle() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        TopologyBuilder::build(&positions, &faces).unwrap()
    }

    // Extrude Face Test 1: Extruding a single triangle into a triangular prism
    #[test]
    fn test_extrude_1_triangle() {
        let mut mesh = make_single_triangle();
        let f = mesh.faces().next().unwrap();

        let res = mesh.extrude_face(f, 1.5).expect("extrude triangle");
        assert_eq!(res.original_face(), f);
        assert!(mesh.is_valid_face(res.top_face()));
        assert_eq!(res.new_vertices().len(), 3);
        assert_eq!(res.new_edges().len(), 6);
        assert_eq!(res.new_faces().len(), 4); // 1 top + 3 side
        assert_eq!(res.new_half_edges().len(), 12);

        assert_eq!(mesh.num_vertices(), 6);
        assert_eq!(mesh.num_edges(), 9);
        assert_eq!(mesh.num_faces(), 5);
        assert_eq!(mesh.num_half_edges(), 18);

        // Triangular prism has Euler characteristic: 6 - 9 + 5 = 2
        assert_eq!(
            (mesh.num_vertices() as i32) - (mesh.num_edges() as i32) + (mesh.num_faces() as i32),
            2
        );
        assert!(mesh.check_invariants().is_ok());
    }

    // Extrude Face Test 2: Extruding a quad into a cube/cuboid
    #[test]
    fn test_extrude_2_quad() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        let res = mesh.extrude_face(f, 2.0).expect("extrude quad");
        assert_eq!(res.original_face(), f);
        assert!(mesh.is_valid_face(res.top_face()));
        assert_eq!(res.new_vertices().len(), 4);
        assert_eq!(res.new_edges().len(), 8);
        assert_eq!(res.new_faces().len(), 5); // 1 top + 4 side
        assert_eq!(res.new_half_edges().len(), 16);

        assert_eq!(mesh.num_vertices(), 8);
        assert_eq!(mesh.num_edges(), 12);
        assert_eq!(mesh.num_faces(), 6);
        assert_eq!(mesh.num_half_edges(), 24);

        // Cuboid has Euler characteristic: 8 - 12 + 6 = 2
        assert_eq!(
            (mesh.num_vertices() as i32) - (mesh.num_edges() as i32) + (mesh.num_faces() as i32),
            2
        );
        assert!(mesh.check_invariants().is_ok());
    }

    // Extrude Face Test 3: Correct generated vertex, edge, and face counts for N-gon
    #[test]
    fn test_extrude_3_correct_element_counts() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();

        let res = mesh.extrude_face(f, 1.0).expect("extrude pentagon");
        assert_eq!(res.new_vertices().len(), 5);
        assert_eq!(res.new_edges().len(), 10);
        assert_eq!(res.new_faces().len(), 6); // 1 top + 5 sides
        assert_eq!(res.new_half_edges().len(), 20);

        assert_eq!(mesh.num_vertices(), 10);
        assert_eq!(mesh.num_edges(), 15);
        assert_eq!(mesh.num_faces(), 7); // 1 bot + 1 top + 5 sides
        assert_eq!(mesh.num_half_edges(), 30);
        assert!(mesh.check_invariants().is_ok());
    }

    // Extrude Face Test 4: Correct translated vertex positions
    #[test]
    fn test_extrude_4_translated_vertex_positions() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let orig_pos: Vec<Vec3> = mesh
            .face_vertices(f)
            .iter()
            .map(|&v| mesh.vertex(v).unwrap().position)
            .collect();

        let dist = 3.5;
        let res = mesh.extrude_face(f, dist).unwrap();
        let norm = Vec3::new(0.0, 0.0, 1.0);

        for (i, &top_v) in res.new_vertices().iter().enumerate() {
            let actual = mesh.vertex(top_v).unwrap().position;
            let expected = orig_pos[i] + norm * dist;
            assert!((actual - expected).length() < 1e-5);
        }
    }

    // Extrude Face Test 5: Correct top and side face loops
    #[test]
    fn test_extrude_5_top_and_side_face_loops() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.extrude_face(f, 1.0).unwrap();

        // Top face loop must have 4 vertices
        let top_verts = mesh.face_vertices(res.top_face());
        assert_eq!(top_verts.len(), 4);
        assert_eq!(top_verts, res.new_vertices());

        // Side faces (indices 1..5 of new_faces) must each have 4 vertices
        for &sf in &res.new_faces()[1..] {
            let sf_verts = mesh.face_vertices(sf);
            assert_eq!(sf_verts.len(), 4);
        }

        // Bottom face must have 4 vertices
        let bot_verts = mesh.face_vertices(res.original_face());
        assert_eq!(bot_verts.len(), 4);
    }

    // Extrude Face Test 6: Reciprocal twin relationships
    #[test]
    fn test_extrude_6_reciprocal_twin_relationships() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.extrude_face(f, 1.0).unwrap();

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(twin.twin, h);
            assert_eq!(he.edge, twin.edge);
            assert_ne!(he.origin, twin.origin);
        }
    }

    // Extrude Face Test 7: Correct next and prev links
    #[test]
    fn test_extrude_7_correct_next_prev_links() {
        let mut mesh = make_single_triangle();
        let f = mesh.faces().next().unwrap();
        mesh.extrude_face(f, 1.0).unwrap();

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let next = mesh.half_edge(he.next).unwrap();
            let prev = mesh.half_edge(he.prev).unwrap();
            assert_eq!(next.prev, h);
            assert_eq!(prev.next, h);
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(next.origin, twin.origin);
        }
    }

    // Extrude Face Test 8: Correct face references
    #[test]
    fn test_extrude_8_correct_face_references() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.extrude_face(f, 1.0).unwrap();

        for fh in mesh.faces() {
            let hes = mesh.face_half_edges(fh);
            for h in hes {
                assert_eq!(mesh.half_edge(h).unwrap().face, Some(fh));
            }
        }

        // All new faces exist
        for &fh in res.new_faces() {
            assert!(mesh.is_valid_face(fh));
        }
    }

    // Extrude Face Test 9: Vertex attribute preservation
    #[test]
    fn test_extrude_9_vertex_attribute_preservation() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        // Assign vertex attributes
        let test_uv = Vec2::new(0.5, 0.75);
        let test_color = [1.0, 0.2, 0.3, 1.0];
        let test_normal = Vec3::new(0.0, 0.0, 1.0);

        let all_verts: Vec<_> = mesh.vertices().collect();
        for v in all_verts {
            let vert = mesh.vertex_mut(v).unwrap();
            vert.uv = Some(test_uv);
            vert.color = Some(test_color);
            vert.normal = Some(test_normal);
        }

        let res = mesh.extrude_face(f, 2.0).unwrap();

        // Check original vertices
        for &v in &mesh.face_vertices(res.original_face()) {
            let vert = mesh.vertex(v).unwrap();
            assert_eq!(vert.uv, Some(test_uv));
            assert_eq!(vert.color, Some(test_color));
            assert_eq!(vert.normal, Some(test_normal));
        }

        // Check top vertices
        for &v in res.new_vertices() {
            let vert = mesh.vertex(v).unwrap();
            assert_eq!(vert.uv, Some(test_uv));
            assert_eq!(vert.color, Some(test_color));
            assert_eq!(vert.normal, Some(test_normal));
        }
    }

    // Extrude Face Test 10: UV seam and corner normal preservation
    #[test]
    fn test_extrude_10_uv_seams_and_corner_normal_preservation() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let corner_uvs = vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ]];
        let corner_normals = vec![vec![
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]];

        let mut mesh = TopologyBuilder::build_with_attributes(
            &positions,
            &faces,
            None,
            None,
            Some(&corner_uvs),
            Some(&corner_normals),
        )
        .unwrap();

        let f = mesh.faces().next().unwrap();
        let res = mesh.extrude_face(f, 1.0).unwrap();

        // Check top face corners
        let top_hes = mesh.face_half_edges(res.top_face());
        for (i, &h) in top_hes.iter().enumerate() {
            let he = mesh.half_edge(h).unwrap();
            assert_eq!(he.uv, Some(corner_uvs[0][i]));
            assert_eq!(he.normal, Some(corner_normals[0][i]));
        }
    }

    // Extrude Face Test 11: Material inheritance
    #[test]
    fn test_extrude_11_material_inheritance() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.face_mut(f).unwrap().material_id = Some(99);

        let res = mesh.extrude_face(f, 1.0).unwrap();
        assert_eq!(
            mesh.face(res.original_face()).unwrap().material_id,
            Some(99)
        );
        assert_eq!(mesh.face(res.top_face()).unwrap().material_id, Some(99));
        for &sf in res.new_faces() {
            assert_eq!(mesh.face(sf).unwrap().material_id, Some(99));
        }
    }

    // Extrude Face Test 12: Rejection of invalid handles
    #[test]
    fn test_extrude_12_rejection_of_invalid_handles() {
        let mut mesh = make_single_quad();
        let invalid_f = FaceHandle::new(9999, 0);
        let res = mesh.extrude_face(invalid_f, 1.0);
        assert!(matches!(res, Err(TopologyError::InvalidFace(_))));

        let null_f = FaceHandle::NULL;
        let res2 = mesh.extrude_face(null_f, 1.0);
        assert!(matches!(res2, Err(TopologyError::InvalidFace(_))));
    }

    // Extrude Face Test 13: Rejection of non-finite and zero distances
    #[test]
    fn test_extrude_13_rejection_of_non_finite_and_zero_distances() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        assert!(matches!(
            mesh.extrude_face(f, f32::NAN),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.extrude_face(f, f32::INFINITY),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.extrude_face(f, 0.0),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.extrude_face(f, 1e-7),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Extrude Face Test 14: Rejection of degenerate geometry
    #[test]
    fn test_extrude_14_rejection_of_degenerate_geometry() {
        // Collinear triangle
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
        let f = mesh.faces().next().unwrap();

        let res = mesh.extrude_face(f, 1.0);
        assert!(matches!(res, Err(TopologyError::Degenerate(_))));
    }

    // Extrude Face Test 15: Rejection of shared adjacent face
    #[test]
    fn test_extrude_15_rejection_of_shared_adjacent_face() {
        let mut mesh = make_two_quads();
        let f = mesh.faces().next().unwrap();

        // Extruding Face 0 when it shares an edge with Face 1 must be rejected
        let res = mesh.extrude_face(f, 1.0);
        assert!(matches!(res, Err(TopologyError::NonManifold(_))));
    }

    // Extrude Face Test 16: Atomic rollback on failure
    #[test]
    fn test_extrude_16_atomic_rollback_on_failure() {
        let mut mesh = make_single_quad();
        let backup = mesh.clone();
        let f = mesh.faces().next().unwrap();

        // Attempt extrusion with invalid distance
        assert!(mesh.extrude_face(f, 0.0).is_err());
        assert_eq!(mesh, backup);

        // Attempt extrusion with invalid face
        assert!(mesh.extrude_face(FaceHandle::NULL, 1.0).is_err());
        assert_eq!(mesh, backup);

        assert!(mesh.check_invariants().is_ok());
    }

    // Extrude Face Test 17: Topology validator and invariants
    #[test]
    fn test_extrude_17_topology_validator_and_invariants() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.extrude_face(f, 2.0).unwrap();
        assert!(mesh.is_valid_face(res.top_face()));

        let val_report = TopologyValidator::validate(&mesh);
        assert!(
            val_report.is_valid,
            "ValidationReport must be valid: {:?}",
            val_report.issues
        );
        assert_eq!(val_report.error_count(), 0);

        let top_report = mesh.validate_topology();
        assert!(top_report.is_valid);
        assert!(top_report.is_manifold);
        assert!(top_report.is_closed);
        assert_eq!(top_report.boundary_loop_count, 0);
        assert_eq!(top_report.euler_characteristic, 2);
        assert!(mesh.check_invariants().is_ok());
    }

    // =========================================================================
    // Phase 6.13: Basic Inset Face Unit Tests
    // =========================================================================

    // Inset Face Test 1: Inset of a square
    #[test]
    fn test_inset_1_square() {
        let mut mesh = make_single_quad(); // 2x2 square: (0,0), (2,0), (2,2), (0,2)
        let f = mesh.faces().next().unwrap();

        let res = mesh.inset_face(f, 0.5).expect("inset square");
        assert_eq!(res.original_face(), f);
        assert!(mesh.is_valid_face(res.inner_face()));
        assert_eq!(res.new_vertices().len(), 4);
        assert_eq!(res.new_edges().len(), 8);
        assert_eq!(res.new_side_faces().len(), 3); // 3 new + 1 original = 4 side faces
        assert_eq!(res.side_faces().len(), 4);
        assert_eq!(res.new_half_edges().len(), 16);

        // Check total element counts
        assert_eq!(mesh.num_vertices(), 8); // 4 outer + 4 inner
        assert_eq!(mesh.num_edges(), 12); // 4 outer + 4 inner + 4 radial
        assert_eq!(mesh.num_faces(), 5); // 1 inner + 4 side
        assert_eq!(mesh.num_half_edges(), 24);

        assert!(mesh.check_invariants().is_ok());
    }

    // Inset Face Test 2: Inset of a regular pentagon
    #[test]
    fn test_inset_2_regular_pentagon() {
        let mut mesh = make_pentagon();
        let f = mesh.faces().next().unwrap();

        let res = mesh.inset_face(f, 0.2).expect("inset pentagon");
        assert_eq!(res.new_vertices().len(), 5);
        assert_eq!(res.new_edges().len(), 10);
        assert_eq!(res.new_side_faces().len(), 4);
        assert_eq!(res.side_faces().len(), 5);
        assert_eq!(res.new_half_edges().len(), 20);

        assert_eq!(mesh.num_vertices(), 10);
        assert_eq!(mesh.num_edges(), 15);
        assert_eq!(mesh.num_faces(), 6); // 1 inner + 5 side
        assert_eq!(mesh.num_half_edges(), 30);
        assert!(mesh.check_invariants().is_ok());
    }

    // Inset Face Test 3: Correct inner vertex positions (parallel edge offset)
    #[test]
    fn test_inset_3_correct_inner_vertex_positions() {
        let mut mesh = make_single_quad(); // [0,0], [2,0], [2,2], [0,2]
        let f = mesh.faces().next().unwrap();

        let offset = 0.4;
        let res = mesh.inset_face(f, offset).unwrap();

        // For a square, each corner moves inward by offset along x and y
        let expected = [
            Vec3::new(0.4, 0.4, 0.0),
            Vec3::new(1.6, 0.4, 0.0),
            Vec3::new(1.6, 1.6, 0.0),
            Vec3::new(0.4, 1.6, 0.0),
        ];

        for (i, &v) in res.new_vertices().iter().enumerate() {
            let actual = mesh.vertex(v).unwrap().position;
            assert!(
                (actual - expected[i]).length() < 1e-5,
                "Vertex {i}: actual {actual:?} != expected {:?}",
                expected[i]
            );
        }
    }

    // Inset Face Test 4: Correct element counts
    #[test]
    fn test_inset_4_correct_element_counts() {
        let mut mesh = make_single_triangle();
        let f = mesh.faces().next().unwrap();

        let res = mesh.inset_face(f, 0.3).unwrap();
        assert_eq!(res.new_vertices().len(), 3);
        assert_eq!(res.new_edges().len(), 6);
        assert_eq!(res.new_side_faces().len(), 2);
        assert_eq!(res.side_faces().len(), 3);
        assert_eq!(res.new_half_edges().len(), 12);

        assert_eq!(mesh.num_vertices(), 6);
        assert_eq!(mesh.num_edges(), 9);
        assert_eq!(mesh.num_faces(), 4);
        assert_eq!(mesh.num_half_edges(), 18);
        assert!(mesh.check_invariants().is_ok());
    }

    // Inset Face Test 5: Valid inner and side face loops
    #[test]
    fn test_inset_5_valid_inner_and_side_face_loops() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.inset_face(f, 0.5).unwrap();

        // Inner face has 4 vertices
        let inner_verts = mesh.face_vertices(res.inner_face());
        assert_eq!(inner_verts.len(), 4);
        assert_eq!(inner_verts, res.new_vertices());

        // Each side face has 4 vertices
        for sf in res.side_faces() {
            let sf_verts = mesh.face_vertices(sf);
            assert_eq!(sf_verts.len(), 4);
        }
    }

    // Inset Face Test 6: Reciprocal twin relationships
    #[test]
    fn test_inset_6_reciprocal_twin_relationships() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.inset_face(f, 0.3).unwrap();

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(twin.twin, h);
            assert_eq!(he.edge, twin.edge);
            assert_ne!(he.origin, twin.origin);
        }
    }

    // Inset Face Test 7: Correct next and prev links
    #[test]
    fn test_inset_7_correct_next_prev_links() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.inset_face(f, 0.3).unwrap();

        for h in mesh.half_edges() {
            let he = mesh.half_edge(h).unwrap();
            let next = mesh.half_edge(he.next).unwrap();
            let prev = mesh.half_edge(he.prev).unwrap();
            assert_eq!(next.prev, h);
            assert_eq!(prev.next, h);
            let twin = mesh.half_edge(he.twin).unwrap();
            assert_eq!(next.origin, twin.origin);
        }
    }

    // Inset Face Test 8: Correct face references
    #[test]
    fn test_inset_8_correct_face_references() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.inset_face(f, 0.5).unwrap();

        assert!(mesh.is_valid_face(res.inner_face()));
        for sf in res.side_faces() {
            assert!(mesh.is_valid_face(sf));
            let hes = mesh.face_half_edges(sf);
            for h in hes {
                assert_eq!(mesh.half_edge(h).unwrap().face, Some(sf));
            }
        }
        let in_hes = mesh.face_half_edges(res.inner_face());
        for h in in_hes {
            assert_eq!(mesh.half_edge(h).unwrap().face, Some(res.inner_face()));
        }
    }

    // Inset Face Test 9: UV seam and corner normal preservation
    #[test]
    fn test_inset_9_uv_seam_and_corner_normal_preservation() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let corner_uvs = vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ]];
        let corner_normals = vec![vec![
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]];

        let mut mesh = TopologyBuilder::build_with_attributes(
            &positions,
            &faces,
            None,
            None,
            Some(&corner_uvs),
            Some(&corner_normals),
        )
        .unwrap();

        let f = mesh.faces().next().unwrap();
        let res = mesh.inset_face(f, 0.5).unwrap();

        // Inner face corners must have corner attributes
        let inner_hes = mesh.face_half_edges(res.inner_face());
        for (i, &h) in inner_hes.iter().enumerate() {
            let he = mesh.half_edge(h).unwrap();
            assert_eq!(he.uv, Some(corner_uvs[0][i]));
            assert_eq!(he.normal, Some(corner_normals[0][i]));
        }
    }

    // Inset Face Test 10: Vertex attribute interpolation
    #[test]
    fn test_inset_10_vertex_attribute_interpolation() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        let test_uv = Vec2::new(0.25, 0.75);
        let test_color = [0.1, 0.2, 0.3, 1.0];
        let test_normal = Vec3::new(0.0, 0.0, 1.0);

        let all_verts: Vec<_> = mesh.vertices().collect();
        for v in all_verts {
            let vert = mesh.vertex_mut(v).unwrap();
            vert.uv = Some(test_uv);
            vert.color = Some(test_color);
            vert.normal = Some(test_normal);
        }

        let res = mesh.inset_face(f, 0.5).unwrap();

        for &v in res.new_vertices() {
            let vert = mesh.vertex(v).unwrap();
            assert_eq!(vert.uv, Some(test_uv));
            assert_eq!(vert.color, Some(test_color));
            assert_eq!(vert.normal, Some(test_normal));
        }
    }

    // Inset Face Test 11: Material inheritance
    #[test]
    fn test_inset_11_material_inheritance() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        mesh.face_mut(f).unwrap().material_id = Some(77);

        let res = mesh.inset_face(f, 0.4).unwrap();
        assert_eq!(mesh.face(res.inner_face()).unwrap().material_id, Some(77));
        for sf in res.side_faces() {
            assert_eq!(mesh.face(sf).unwrap().material_id, Some(77));
        }
    }

    // Inset Face Test 12: Rejection of invalid handles
    #[test]
    fn test_inset_12_rejection_of_invalid_handles() {
        let mut mesh = make_single_quad();
        let res = mesh.inset_face(FaceHandle::new(9999, 0), 0.5);
        assert!(matches!(res, Err(TopologyError::InvalidFace(_))));

        let res_null = mesh.inset_face(FaceHandle::NULL, 0.5);
        assert!(matches!(res_null, Err(TopologyError::InvalidFace(_))));
    }

    // Inset Face Test 13: Rejection of non-finite and zero offsets
    #[test]
    fn test_inset_13_rejection_of_non_finite_and_zero_offsets() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();

        assert!(matches!(
            mesh.inset_face(f, 0.0),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.inset_face(f, -0.5),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.inset_face(f, f32::NAN),
            Err(TopologyError::InvalidOperation(_))
        ));
        assert!(matches!(
            mesh.inset_face(f, f32::INFINITY),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Inset Face Test 14: Rejection of concave and non-planar polygons
    #[test]
    fn test_inset_14_rejection_of_concave_and_non_planar_polygons() {
        // Concave L-shape: (0,0), (2,0), (2,1), (1,1), (1,2), (0,2)
        let concave_pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let concave_faces = vec![vec![0, 1, 2, 3, 4, 5]];
        let mut mesh_concave = TopologyBuilder::build(&concave_pos, &concave_faces).unwrap();
        let f_concave = mesh_concave.faces().next().unwrap();
        assert!(matches!(
            mesh_concave.inset_face(f_concave, 0.2),
            Err(TopologyError::InvalidOperation(_))
        ));

        // Non-planar quad
        let non_planar_pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 1.0), // z = 1.0 out of plane
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let non_planar_faces = vec![vec![0, 1, 2, 3]];
        let mut mesh_np = TopologyBuilder::build(&non_planar_pos, &non_planar_faces).unwrap();
        let f_np = mesh_np.faces().next().unwrap();
        assert!(matches!(
            mesh_np.inset_face(f_np, 0.2),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Inset Face Test 15: Rejection of excessive offsets
    #[test]
    fn test_inset_15_rejection_of_excessive_offsets() {
        let mut mesh = make_single_quad(); // 2x2 square, half-width is 1.0
        let f = mesh.faces().next().unwrap();

        // Offset 1.0 exactly collapses the inner polygon
        assert!(matches!(
            mesh.inset_face(f, 1.0),
            Err(TopologyError::InvalidOperation(_))
        ));

        // Offset 2.0 inverts the inner polygon
        assert!(matches!(
            mesh.inset_face(f, 2.0),
            Err(TopologyError::InvalidOperation(_))
        ));
    }

    // Inset Face Test 16: Rejection of degenerate geometry
    #[test]
    fn test_inset_16_rejection_of_degenerate_geometry() {
        // Collinear vertices
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
        let f = mesh.faces().next().unwrap();

        let res = mesh.inset_face(f, 0.1);
        assert!(res.is_err());
    }

    // Inset Face Test 17: Atomic rollback on failure
    #[test]
    fn test_inset_17_atomic_rollback() {
        let mut mesh = make_single_quad();
        let backup = mesh.clone();
        let f = mesh.faces().next().unwrap();

        // 1. Excessive offset
        assert!(mesh.inset_face(f, 5.0).is_err());
        assert_eq!(mesh, backup);

        // 2. Invalid distance
        assert!(mesh.inset_face(f, -1.0).is_err());
        assert_eq!(mesh, backup);

        assert!(mesh.check_invariants().is_ok());
    }

    // Inset Face Test 18: Successful TopologyValidator and invariant checks
    #[test]
    fn test_inset_18_topology_validator_and_invariants() {
        let mut mesh = make_single_quad();
        let f = mesh.faces().next().unwrap();
        let res = mesh.inset_face(f, 0.5).unwrap();
        assert!(mesh.is_valid_face(res.inner_face()));

        let val_report = TopologyValidator::validate(&mesh);
        assert!(
            val_report.is_valid,
            "ValidationReport must be valid: {:?}",
            val_report.issues
        );
        assert_eq!(val_report.error_count(), 0);

        let top_report = mesh.validate_topology();
        assert!(top_report.is_valid);
        assert!(top_report.is_manifold);
        assert_eq!(top_report.boundary_loop_count, 1); // open sheet keeps outer boundary
        assert!(mesh.check_invariants().is_ok());
    }
}
