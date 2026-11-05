use std::collections::HashSet;

use super::error::TopologyError;
use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};
use super::mesh::HalfEdgeMesh;

impl HalfEdgeMesh {
    fn refresh_vertex_outgoing_half_edge(&mut self, vertex: VertexHandle) {
        let mut boundary = None;
        let mut any_outgoing = None;
        for (idx, half_edge) in self.half_edges.iter().enumerate() {
            if !half_edge.deleted && half_edge.origin == vertex {
                let handle = HalfEdgeHandle::new(idx as u32, half_edge.generation);
                if half_edge.is_boundary() {
                    boundary = Some(handle);
                    break;
                }
                if any_outgoing.is_none() {
                    any_outgoing = Some(handle);
                }
            }
        }
        if let Some(vertex_data) = self.vertex_mut(vertex) {
            vertex_data.half_edge = boundary.or(any_outgoing);
        }
    }

    /// Inserts a new vertex along edge `e` at linear interpolation parameter `t` in (0, 1).
    ///
    /// Subdivides the edge into two edges and updates the incident face/boundary cycles.
    /// Transactionally rolls back if an invariant is violated.
    pub fn split_edge(
        &mut self,
        edge: EdgeHandle,
        t: f32,
    ) -> Result<(VertexHandle, EdgeHandle), TopologyError> {
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
        let p_new = (1.0 - t) * p0 + t * p1;

        // Allocate new vertex
        let v_new = self.add_vertex(p_new);

        // Interpolate vertex normal and uv if available
        if let (Some(n0), Some(n1)) = (
            self.vertex(v0).unwrap().normal,
            self.vertex(v1).unwrap().normal,
        ) {
            let n_new = ((1.0 - t) * n0 + t * n1).normalize_or_zero();
            self.vertex_mut(v_new).unwrap().normal = Some(n_new);
        }
        if let (Some(uv0), Some(uv1)) = (self.vertex(v0).unwrap().uv, self.vertex(v1).unwrap().uv) {
            let uv_new = (1.0 - t) * uv0 + t * uv1;
            self.vertex_mut(v_new).unwrap().uv = Some(uv_new);
        }

        let f0 = self.half_edge(h0).unwrap().face;
        let f1 = self.half_edge(h1).unwrap().face;

        let h0_next = self.half_edge(h0).unwrap().next;
        let h1_next = self.half_edge(h1).unwrap().next;

        // Create new edge connecting v_new and v1
        let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
        let e_new = self.add_edge(dummy_he);

        // Create new half-edge h0b (v_new -> v1)
        let h0b = self.add_half_edge(v_new, e_new, f0);
        // Create new half-edge h1b (v_new -> v0)
        let h1b = self.add_half_edge(v_new, edge, f1);

        self.edge_mut(e_new).unwrap().half_edge = h0b;

        // Link twins
        // edge connects h0 and h1b
        // edge connects h0 (v0 -> v_new) and h1b (v_new -> v0)
        self.half_edge_mut(h0).unwrap().twin = h1b;
        self.half_edge_mut(h1b).unwrap().twin = h0;

        // e_new connects h0b and h1
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

        // Outgoing half-edges for vertices
        self.vertex_mut(v_new).unwrap().half_edge = Some(h0b);
        self.vertex_mut(v0).unwrap().half_edge = Some(h0);
        self.vertex_mut(v1).unwrap().half_edge = Some(h1);
        let v_new_out = if f1.is_none() { h1b } else { h0b };
        self.vertex_mut(v_new).unwrap().half_edge = Some(v_new_out);

        // Transactional invariant verification
        if let Err(e) = self.check_invariants() {
            *self = backup;
            return Err(e);
        }

        Ok((v_new, e_new))
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

    /// Dissolves an internal edge between two faces, merging them into a single polygon face.
    ///
    /// The first incident face `f0` is retained as the surviving face, and its `material_id`
    /// is kept as the deterministic material policy. The second incident face `f1`, the edge,
    /// and its two half-edges are deleted and their slots freed.
    ///
    /// Transactionally rolls back if an invariant is violated.
    pub fn merge_faces(&mut self, edge: EdgeHandle) -> Result<FaceHandle, TopologyError> {
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

    /// Collapses edge `edge` by merging its endpoints into a single vertex at interpolation parameter `t` in [0, 1].
    ///
    /// Preserves 2-manifold invariants using the topological Link Condition.
    /// Transactionally rolls back if an invariant is violated.
    pub fn collapse_edge(&mut self, edge: EdgeHandle) -> Result<VertexHandle, TopologyError> {
        if !self.is_valid_edge(edge) {
            return Err(TopologyError::InvalidEdge(edge));
        }
        let t = 0.5;

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
