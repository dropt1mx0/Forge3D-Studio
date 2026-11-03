use std::collections::HashSet;

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

    /// Connects two non-adjacent vertices of face `f` with a new edge, splitting it into two faces.
    ///
    /// Transactionally rolls back if an invariant is violated.
    pub fn split_face(
        &mut self,
        face: FaceHandle,
        v0: VertexHandle,
        v1: VertexHandle,
    ) -> Result<(FaceHandle, EdgeHandle), TopologyError> {
        if !self.is_valid_face(face) {
            return Err(TopologyError::InvalidFace(face));
        }
        if !self.is_valid_vertex(v0) || !self.is_valid_vertex(v1) {
            return Err(TopologyError::InvalidOperation(
                "Invalid vertex handle for face split".into(),
            ));
        }
        if v0 == v1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot split face using the same vertex twice".into(),
            ));
        }

        let backup = self.clone();

        let f_hes = self.face_half_edges(face);
        let pos0 = f_hes
            .iter()
            .position(|&h| self.half_edge(h).unwrap().origin == v0)
            .ok_or_else(|| {
                TopologyError::InvalidOperation(format!("Vertex {v0} does not belong to {face}"))
            })?;
        let pos1 = f_hes
            .iter()
            .position(|&h| self.half_edge(h).unwrap().origin == v1)
            .ok_or_else(|| {
                TopologyError::InvalidOperation(format!("Vertex {v1} does not belong to {face}"))
            })?;

        let k = f_hes.len();
        if (pos0 + 1) % k == pos1 || (pos1 + 1) % k == pos0 {
            return Err(TopologyError::InvalidOperation(
                "Cannot split face between adjacent vertices (edge already exists)".into(),
            ));
        }

        let h_a = f_hes[pos0]; // starts at v0
        let h_b = f_hes[pos1]; // starts at v1

        let h_a_prev = self.half_edge(h_a).unwrap().prev;
        let h_b_prev = self.half_edge(h_b).unwrap().prev;

        // Allocate new edge and new face
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let e_new = self.add_edge(dummy_he);
        let f_new = self.add_face(dummy_he);

        // h_new0 runs v0 -> v1 (belongs to f_new)
        let h_new0 = self.add_half_edge(v0, e_new, Some(f_new));
        // h_new1 runs v1 -> v0 (belongs to original face)
        let h_new1 = self.add_half_edge(v1, e_new, Some(face));

        self.edge_mut(e_new).unwrap().half_edge = h_new0;

        self.half_edge_mut(h_new0).unwrap().twin = h_new1;
        self.half_edge_mut(h_new1).unwrap().twin = h_new0;

        // Rewire face f: h_a .. h_b_prev -> h_new1 -> h_a
        self.half_edge_mut(h_b_prev).unwrap().next = h_new1;
        self.half_edge_mut(h_new1).unwrap().prev = h_b_prev;
        self.half_edge_mut(h_new1).unwrap().next = h_a;
        self.half_edge_mut(h_a).unwrap().prev = h_new1;
        self.face_mut(face).unwrap().half_edge = h_a;

        // Rewire f_new: h_b .. h_a_prev -> h_new0 -> h_b
        self.half_edge_mut(h_a_prev).unwrap().next = h_new0;
        self.half_edge_mut(h_new0).unwrap().prev = h_a_prev;
        self.half_edge_mut(h_new0).unwrap().next = h_b;
        self.half_edge_mut(h_b).unwrap().prev = h_new0;
        self.face_mut(f_new).unwrap().half_edge = h_b;

        // Update face pointers for half-edges in f_new cycle
        let mut curr = h_b;
        while curr != h_new0 {
            self.half_edge_mut(curr).unwrap().face = Some(f_new);
            curr = self.half_edge(curr).unwrap().next;
        }

        // Recalculate face normals
        let norm_f = self.face_normal(face);
        let norm_f_new = self.face_normal(f_new);
        self.face_mut(face).unwrap().normal = Some(norm_f);
        self.face_mut(f_new).unwrap().normal = Some(norm_f_new);

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok((f_new, e_new))
    }

    /// Removes an internal edge between two faces, merging them into a single polygon face.
    ///
    /// Transactionally rolls back if an invariant is violated.
    pub fn merge_faces(&mut self, edge: EdgeHandle) -> Result<FaceHandle, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let f0 = self.half_edge(h0).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation("Cannot merge on boundary edge".into())
        })?;
        let f1 = self.half_edge(h1).unwrap().face.ok_or_else(|| {
            TopologyError::InvalidOperation("Cannot merge on boundary edge".into())
        })?;

        if f0 == f1 {
            return Err(TopologyError::InvalidOperation(
                "Cannot merge edge belonging to the same face on both sides".into(),
            ));
        }

        let v0 = self.half_edge(h0).unwrap().origin;
        let v1 = self.half_edge(h1).unwrap().origin;

        let h0_prev = self.half_edge(h0).unwrap().prev;
        let h0_next = self.half_edge(h0).unwrap().next;
        let h1_prev = self.half_edge(h1).unwrap().prev;
        let h1_next = self.half_edge(h1).unwrap().next;

        // Splice cycles together bypassing h0 and h1
        self.half_edge_mut(h0_prev).unwrap().next = h1_next;
        self.half_edge_mut(h1_next).unwrap().prev = h0_prev;

        self.half_edge_mut(h1_prev).unwrap().next = h0_next;
        self.half_edge_mut(h0_next).unwrap().prev = h1_prev;

        // Retarget all half-edges in f1 to f0
        let mut curr = h1_next;
        while curr != h0_prev {
            self.half_edge_mut(curr).unwrap().face = Some(f0);
            curr = self.half_edge(curr).unwrap().next;
        }

        self.face_mut(f0).unwrap().half_edge = h0_next;

        // Fix vertex outgoing pointers if they pointed to h0 or h1
        if self.vertex(v0).unwrap().half_edge == Some(h0) {
            self.vertex_mut(v0).unwrap().half_edge = Some(h1_next);
        }
        if self.vertex(v1).unwrap().half_edge == Some(h1) {
            self.vertex_mut(v1).unwrap().half_edge = Some(h0_next);
        }

        // Delete h0, h1, edge, and f1
        self.free_half_edge(h0)?;
        self.free_half_edge(h1)?;
        self.free_edge(edge)?;
        self.free_face(f1)?;

        let norm = self.face_normal(f0);
        self.face_mut(f0).unwrap().normal = Some(norm);

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(f0)
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
        assert_eq!(res.surviving_vertex().index(), 0);
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
}
