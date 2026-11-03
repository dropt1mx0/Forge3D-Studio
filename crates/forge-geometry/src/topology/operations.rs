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
    pub fn flip_edge(&mut self, edge: EdgeHandle) -> Result<(), TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let f0 =
            self.half_edge(h0).unwrap().face.ok_or_else(|| {
                TopologyError::InvalidOperation("Cannot flip boundary edge".into())
            })?;
        let f1 =
            self.half_edge(h1).unwrap().face.ok_or_else(|| {
                TopologyError::InvalidOperation("Cannot flip boundary edge".into())
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
                "Edge flip is only supported between two triangular faces".into(),
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

        // Geometric convexity check: quad (v0, v3, v1, v2) must be strictly convex
        let p0 = self.vertex(v0).unwrap().position;
        let p1 = self.vertex(v1).unwrap().position;
        let p2 = self.vertex(v2).unwrap().position;
        let p3 = self.vertex(v3).unwrap().position;

        let n_old0 = (p1 - p0).cross(p2 - p0);
        let n_old1 = (p0 - p1).cross(p3 - p1);

        // New triangles with flipped diagonal (v2, v3):
        // Triangle f0 (v2, v3, v1):
        let n_new0 = (p3 - p2).cross(p1 - p2);
        // Triangle f1 (v3, v2, v0):
        let n_new1 = (p2 - p3).cross(p0 - p3);

        if n_new0.dot(n_old0) <= 0.0 || n_new1.dot(n_old1) <= 0.0 || n_new0.dot(n_new1) <= 0.0 {
            return Err(TopologyError::InvalidOperation(
                "Cannot flip edge: resulting triangles would invert or fold".into(),
            ));
        }

        // Rewire origins: edge now connects v2 and v3
        self.half_edge_mut(h0).unwrap().origin = v2;
        self.half_edge_mut(h1).unwrap().origin = v3;

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

        // Outgoing half-edges for vertices
        self.vertex_mut(v0).unwrap().half_edge = Some(h1_next);
        self.vertex_mut(v1).unwrap().half_edge = Some(h0_next);
        self.vertex_mut(v2).unwrap().half_edge = Some(h0);
        self.vertex_mut(v3).unwrap().half_edge = Some(h1);

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

        Ok(())
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

    /// Collapses edge `e` by merging its endpoints into a single vertex.
    ///
    /// Preserves 2-manifold invariants using the topological Link Condition.
    /// Transactionally rolls back if an invariant is violated.
    pub fn collapse_edge(&mut self, edge: EdgeHandle) -> Result<VertexHandle, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }

        let backup = self.clone();

        let (h0, h1) = self
            .edge_half_edges(edge)
            .ok_or(TopologyError::InvalidEdge(edge))?;

        let v0 = self.half_edge(h0).unwrap().origin;
        let v1 = self.half_edge(h1).unwrap().origin;

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
        if let Some(f0) = self.half_edge(h0).unwrap().face {
            let f0_hes = self.face_half_edges(f0);
            if f0_hes.len() == 3 {
                let h0_prev = self.half_edge(h0).unwrap().prev;
                expected_common.insert(self.half_edge(h0_prev).unwrap().origin);
            } else {
                return Err(TopologyError::InvalidOperation(
                    "Edge collapse currently requires triangular incident faces".into(),
                ));
            }
        }
        if let Some(f1) = self.half_edge(h1).unwrap().face {
            let f1_hes = self.face_half_edges(f1);
            if f1_hes.len() == 3 {
                let h1_prev = self.half_edge(h1).unwrap().prev;
                expected_common.insert(self.half_edge(h1_prev).unwrap().origin);
            } else {
                return Err(TopologyError::InvalidOperation(
                    "Edge collapse currently requires triangular incident faces".into(),
                ));
            }
        }

        if common_neighbors != expected_common {
            return Err(TopologyError::InvalidOperation(
                "Link condition violated: edge collapse would create non-manifold geometry".into(),
            ));
        }

        // Prevent collapsing tetrahedral / minimal meshes into degenerate surfaces
        if self.num_faces() <= 2 {
            return Err(TopologyError::InvalidOperation(
                "Cannot collapse edge: mesh has too few faces".into(),
            ));
        }

        // Midpoint position for merged vertex v0
        let p0 = self.vertex(v0).unwrap().position;
        let p1 = self.vertex(v1).unwrap().position;
        let p_mid = 0.5 * (p0 + p1);
        self.vertex_mut(v0).unwrap().position = p_mid;

        // Retarget all outgoing half-edges from v1 to originate from v0
        let v1_out = self.outgoing_half_edges(v1);
        for out_he in v1_out {
            if out_he != h1 {
                self.half_edge_mut(out_he).unwrap().origin = v0;
            }
        }

        // For incident triangular faces, collapse the triangle
        let collapse_triangle =
            |mesh: &mut Self, he_in_tri: HalfEdgeHandle| -> Result<(), TopologyError> {
                let Some(f) = mesh.half_edge(he_in_tri).unwrap().face else {
                    return Ok(());
                };
                let f_hes = mesh.face_half_edges(f);
                if f_hes.len() != 3 {
                    return Ok(());
                }

                let h_next = mesh.half_edge(he_in_tri).unwrap().next;
                let h_prev = mesh.half_edge(he_in_tri).unwrap().prev;

                let twin_next = mesh.half_edge(h_next).unwrap().twin;
                let twin_prev = mesh.half_edge(h_prev).unwrap().twin;

                let edge_to_remove = mesh.half_edge(h_next).unwrap().edge;
                let edge_to_keep = mesh.half_edge(h_prev).unwrap().edge;

                let v_opp = mesh.half_edge(h_prev).unwrap().origin;

                // Merge twin_next and twin_prev
                mesh.half_edge_mut(twin_next).unwrap().twin = twin_prev;
                mesh.half_edge_mut(twin_prev).unwrap().twin = twin_next;

                mesh.half_edge_mut(twin_next).unwrap().edge = edge_to_keep;
                mesh.half_edge_mut(twin_prev).unwrap().edge = edge_to_keep;
                mesh.edge_mut(edge_to_keep).unwrap().half_edge = twin_prev;

                // If v_opp pointed to h_prev, repoint it to twin_next
                if mesh.vertex(v_opp).unwrap().half_edge == Some(h_prev) {
                    mesh.vertex_mut(v_opp).unwrap().half_edge = Some(twin_next);
                }

                mesh.free_half_edge(h_next)?;
                mesh.free_half_edge(h_prev)?;
                mesh.free_edge(edge_to_remove)?;
                mesh.free_face(f)?;

                Ok(())
            };

        collapse_triangle(self, h0)?;
        collapse_triangle(self, h1)?;

        // Remove h0, h1, edge, and v1
        self.free_half_edge(h0)?;
        self.free_half_edge(h1)?;
        self.free_edge(edge)?;
        self.free_vertex(v1)?;

        // Update v0 outgoing half-edge
        let mut new_v0_out = None;
        for (idx, he) in self.half_edges.iter().enumerate() {
            if !he.deleted && he.origin == v0 {
                let he_h = HalfEdgeHandle::new(idx as u32, he.generation);
                if he.is_boundary() {
                    new_v0_out = Some(he_h);
                    break;
                } else if new_v0_out.is_none() {
                    new_v0_out = Some(he_h);
                }
            }
        }
        self.vertex_mut(v0).unwrap().half_edge = new_v0_out;

        // Recalculate normals of incident faces
        for f in self.adjacent_faces(v0) {
            let n = self.face_normal(f);
            if let Some(face_ref) = self.face_mut(f) {
                face_ref.normal = Some(n);
            }
        }

        // Invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok(v0)
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
}
