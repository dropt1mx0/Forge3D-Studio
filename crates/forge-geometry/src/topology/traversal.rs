use std::collections::{HashSet, VecDeque};

use forge_math::Vec3;

use super::error::TopologyError;
use super::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle, VertexHandle};
use super::mesh::HalfEdgeMesh;
use super::selection::{self, EdgeSelectionPath, TerminationReason};

impl HalfEdgeMesh {
    // --- Vertex Circulators & Queries ---

    /// Returns all directed half-edges originating from vertex `v`.
    pub fn outgoing_half_edges(&self, v: VertexHandle) -> Vec<HalfEdgeHandle> {
        let mut result = Vec::new();
        let Some(vert) = self.vertex(v) else {
            return result;
        };
        let Some(first_he) = vert.half_edge else {
            return result;
        };

        let mut curr = first_he;
        let mut visited = HashSet::new();

        while visited.insert(curr) {
            result.push(curr);
            let Some(he) = self.half_edge(curr) else {
                break;
            };
            let Some(prev_he) = self.half_edge(he.prev) else {
                break;
            };
            let next_out = prev_he.twin;
            if next_out == first_he || !self.is_valid_half_edge(next_out) {
                break;
            }
            curr = next_out;
        }

        result
    }

    /// Returns all directed half-edges ending at vertex `v`.
    pub fn incoming_half_edges(&self, v: VertexHandle) -> Vec<HalfEdgeHandle> {
        self.outgoing_half_edges(v)
            .into_iter()
            .filter_map(|out_he| self.half_edge(out_he).map(|he| he.twin))
            .collect()
    }

    /// Returns all 1-ring neighbor vertices connected to `v` by an edge without duplicates.
    pub fn adjacent_vertices(&self, v: VertexHandle) -> Vec<VertexHandle> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for out_he in self.outgoing_half_edges(v) {
            if let Some(he) = self.half_edge(out_he)
                && let Some(twin) = self.half_edge(he.twin)
                && seen.insert(twin.origin)
            {
                result.push(twin.origin);
            }
        }
        result
    }

    /// Returns all polygon faces incident to vertex `v` without duplicates.
    pub fn adjacent_faces(&self, v: VertexHandle) -> Vec<FaceHandle> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for out_he in self.outgoing_half_edges(v) {
            if let Some(he) = self.half_edge(out_he)
                && let Some(fh) = he.face
                && seen.insert(fh)
            {
                result.push(fh);
            }
        }
        result
    }

    /// Returns true if vertex `v` lies on an open boundary or hole.
    pub fn is_vertex_boundary(&self, v: VertexHandle) -> bool {
        self.outgoing_half_edges(v).into_iter().any(|h| {
            if let Some(he) = self.half_edge(h) {
                he.is_boundary()
            } else {
                false
            }
        })
    }

    /// Returns the topological valence (number of incident edges) of vertex `v`.
    pub fn vertex_valence(&self, v: VertexHandle) -> usize {
        self.outgoing_half_edges(v).len()
    }

    /// Returns all undirected edges incident to vertex `v`.
    pub fn incident_edges(&self, v: VertexHandle) -> Vec<EdgeHandle> {
        let mut edges = Vec::new();
        let mut seen = HashSet::new();
        for he_h in self.outgoing_half_edges(v) {
            if let Some(he) = self.half_edge(he_h)
                && seen.insert(he.edge)
            {
                edges.push(he.edge);
            }
        }
        edges
    }

    /// Returns all edges adjacent/incident to vertex `v` (alias for `incident_edges`).
    #[inline]
    pub fn vertex_adjacent_edges(&self, v: VertexHandle) -> Vec<EdgeHandle> {
        self.incident_edges(v)
    }

    // --- Edge Queries ---

    /// Returns the two twin half-edges forming edge `e`.
    pub fn edge_half_edges(&self, e: EdgeHandle) -> Option<(HalfEdgeHandle, HalfEdgeHandle)> {
        let edge = self.edge(e)?;
        let h0 = edge.half_edge;
        let he0 = self.half_edge(h0)?;
        Some((h0, he0.twin))
    }

    /// Returns the two endpoint vertices of edge `e`.
    pub fn edge_vertices(&self, e: EdgeHandle) -> Option<(VertexHandle, VertexHandle)> {
        let (h0, h1) = self.edge_half_edges(e)?;
        let v0 = self.half_edge(h0)?.origin;
        let v1 = self.half_edge(h1)?.origin;
        Some((v0, v1))
    }

    /// Returns the incident faces of edge `e` (up to 2; `None` represents open boundary).
    pub fn edge_faces(&self, e: EdgeHandle) -> Option<(Option<FaceHandle>, Option<FaceHandle>)> {
        let (h0, h1) = self.edge_half_edges(e)?;
        let f0 = self.half_edge(h0)?.face;
        let f1 = self.half_edge(h1)?.face;
        Some((f0, f1))
    }

    /// Returns all incident non-boundary faces of edge `e` without duplicates.
    pub fn edge_adjacent_faces(&self, e: EdgeHandle) -> Vec<FaceHandle> {
        let mut result = Vec::new();
        if let Some((f0, f1)) = self.edge_faces(e) {
            if let Some(face0) = f0 {
                result.push(face0);
            }
            if let Some(face1) = f1
                && Some(face1) != f0
            {
                result.push(face1);
            }
        }
        result
    }

    /// Returns true if edge `e` lies on an open mesh boundary.
    pub fn is_edge_boundary(&self, e: EdgeHandle) -> bool {
        if let Some((f0, f1)) = self.edge_faces(e) {
            f0.is_none() || f1.is_none()
        } else {
            false
        }
    }

    /// Returns all edges adjacent to edge `e` sharing at least one vertex.
    pub fn adjacent_edges(&self, e: EdgeHandle) -> Vec<EdgeHandle> {
        let mut result = Vec::new();
        let Some((v0, v1)) = self.edge_vertices(e) else {
            return result;
        };

        let mut seen = HashSet::new();
        seen.insert(e);

        for v in [v0, v1] {
            for he_h in self.outgoing_half_edges(v) {
                if let Some(he) = self.half_edge(he_h)
                    && seen.insert(he.edge)
                {
                    result.push(he.edge);
                }
            }
        }

        result
    }

    // --- Face Queries ---

    /// Returns all half-edges bounding face `f` in counter-clockwise order.
    pub fn face_half_edges(&self, f: FaceHandle) -> Vec<HalfEdgeHandle> {
        let mut result = Vec::new();
        let Some(face) = self.face(f) else {
            return result;
        };

        let first = face.half_edge;
        let mut curr = first;
        let mut visited = HashSet::new();

        while visited.insert(curr) {
            result.push(curr);
            let Some(he) = self.half_edge(curr) else {
                break;
            };
            if he.next == first || !self.is_valid_half_edge(he.next) {
                break;
            }
            curr = he.next;
        }

        result
    }

    /// Returns all vertices of face `f` in counter-clockwise order.
    pub fn face_vertices(&self, f: FaceHandle) -> Vec<VertexHandle> {
        self.face_half_edges(f)
            .into_iter()
            .filter_map(|he_h| self.half_edge(he_h).map(|he| he.origin))
            .collect()
    }

    /// Returns all edges bounding face `f`.
    pub fn face_edges(&self, f: FaceHandle) -> Vec<EdgeHandle> {
        self.face_half_edges(f)
            .into_iter()
            .filter_map(|he_h| self.half_edge(he_h).map(|he| he.edge))
            .collect()
    }

    /// Returns the number of vertices bounding face `f`.
    pub fn face_vertex_count(&self, f: FaceHandle) -> usize {
        self.face_half_edges(f).len()
    }

    /// Returns the number of bounding sides/edges of face `f` (alias for `face_vertex_count`).
    #[inline]
    pub fn face_side_count(&self, f: FaceHandle) -> usize {
        self.face_vertex_count(f)
    }

    /// Returns all adjacent faces sharing an edge with face `f` without duplicates.
    pub fn face_adjacent_faces(&self, f: FaceHandle) -> Vec<FaceHandle> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for he_h in self.face_half_edges(f) {
            if let Some(he) = self.half_edge(he_h)
                && let Some(twin) = self.half_edge(he.twin)
                && let Some(neighbor_face) = twin.face
                && neighbor_face != f
                && seen.insert(neighbor_face)
            {
                result.push(neighbor_face);
            }
        }
        result
    }

    /// Traverses face `f` returning its ordered half-edges with full cycle safety and error reporting.
    pub fn face_loop_safe(&self, f: FaceHandle) -> Result<Vec<HalfEdgeHandle>, TopologyError> {
        if !self.is_valid_face(f) {
            return Err(TopologyError::InvalidFace(f));
        }

        let face = self.face(f).unwrap();
        let first = face.half_edge;
        if !self.is_valid_half_edge(first) {
            return Err(TopologyError::MissingConnection(format!(
                "Face {f} references invalid start half-edge {first}"
            )));
        }

        let mut result = Vec::new();
        let mut curr = first;
        let mut visited = HashSet::new();
        let max_steps = self.half_edge_count().max(16);
        let mut closed = false;

        while result.len() < max_steps {
            if !visited.insert(curr) {
                if curr == first {
                    closed = true;
                }
                break;
            }

            result.push(curr);

            let Some(he) = self.half_edge(curr) else {
                return Err(TopologyError::BrokenCycle(format!(
                    "Face {f} cycle references invalid half-edge {curr}"
                )));
            };

            let next_h = he.next;
            if !self.is_valid_half_edge(next_h) {
                return Err(TopologyError::BrokenCycle(format!(
                    "Face {f} cycle half-edge {curr} next handle {next_h} is invalid"
                )));
            }

            if next_h == first {
                closed = true;
                break;
            }

            curr = next_h;
        }

        if !closed {
            if result.len() >= max_steps {
                return Err(TopologyError::TraversalLimitExceeded(format!(
                    "Face {f} cycle exceeded step limit {max_steps}"
                )));
            }
            return Err(TopologyError::BrokenCycle(format!(
                "Face {f} cycle is unclosed"
            )));
        }

        Ok(result)
    }

    /// Computes the unit normal of face `f` using Newell's polygon method.
    pub fn face_normal(&self, f: FaceHandle) -> Vec3 {
        let verts = self.face_vertices(f);
        if verts.len() < 3 {
            return Vec3::ZERO;
        }

        let mut normal = Vec3::ZERO;
        let n = verts.len();
        for i in 0..n {
            let v_curr = self
                .vertex(verts[i])
                .map(|v| v.position)
                .unwrap_or(Vec3::ZERO);
            let v_next = self
                .vertex(verts[(i + 1) % n])
                .map(|v| v.position)
                .unwrap_or(Vec3::ZERO);

            normal.x += (v_curr.y - v_next.y) * (v_curr.z + v_next.z);
            normal.y += (v_curr.z - v_next.z) * (v_curr.x + v_next.x);
            normal.z += (v_curr.x - v_next.x) * (v_curr.y + v_next.y);
        }

        normal.normalize_or_zero()
    }

    /// Computes the 3D surface area of face `f`.
    pub fn face_area(&self, f: FaceHandle) -> f32 {
        let verts = self.face_vertices(f);
        if verts.len() < 3 {
            return 0.0;
        }

        let mut normal = Vec3::ZERO;
        let n = verts.len();
        for i in 0..n {
            let v_curr = self
                .vertex(verts[i])
                .map(|v| v.position)
                .unwrap_or(Vec3::ZERO);
            let v_next = self
                .vertex(verts[(i + 1) % n])
                .map(|v| v.position)
                .unwrap_or(Vec3::ZERO);

            normal.x += (v_curr.y - v_next.y) * (v_curr.z + v_next.z);
            normal.y += (v_curr.z - v_next.z) * (v_curr.x + v_next.x);
            normal.z += (v_curr.x - v_next.x) * (v_curr.y + v_next.y);
        }

        0.5 * normal.length()
    }

    /// Computes the arithmetic centroid of face `f`.
    pub fn face_center(&self, f: FaceHandle) -> Vec3 {
        let verts = self.face_vertices(f);
        if verts.is_empty() {
            return Vec3::ZERO;
        }

        let mut sum = Vec3::ZERO;
        for &vh in &verts {
            if let Some(v) = self.vertex(vh) {
                sum += v.position;
            }
        }

        sum / (verts.len() as f32)
    }

    // --- Half-Edge Navigation ---

    /// Returns the next half-edge in the face or boundary loop, or None if invalid.
    #[inline]
    pub fn half_edge_next(&self, h: HalfEdgeHandle) -> Option<HalfEdgeHandle> {
        let he = self.half_edge(h)?;
        if self.is_valid_half_edge(he.next) {
            Some(he.next)
        } else {
            None
        }
    }

    /// Returns the previous half-edge in the face or boundary loop, or None if invalid.
    #[inline]
    pub fn half_edge_prev(&self, h: HalfEdgeHandle) -> Option<HalfEdgeHandle> {
        let he = self.half_edge(h)?;
        if self.is_valid_half_edge(he.prev) {
            Some(he.prev)
        } else {
            None
        }
    }

    /// Returns the twin opposite half-edge, or None if invalid.
    #[inline]
    pub fn half_edge_twin(&self, h: HalfEdgeHandle) -> Option<HalfEdgeHandle> {
        let he = self.half_edge(h)?;
        if self.is_valid_half_edge(he.twin) {
            Some(he.twin)
        } else {
            None
        }
    }

    /// Returns the incident face of the twin half-edge (adjacent across this half-edge), or None if boundary.
    #[inline]
    pub fn half_edge_opposite_face(&self, h: HalfEdgeHandle) -> Option<FaceHandle> {
        let twin_h = self.half_edge_twin(h)?;
        self.half_edge(twin_h)?.face
    }

    /// Returns the origin (start) vertex of half-edge `h`.
    #[inline]
    pub fn half_edge_origin(&self, h: HalfEdgeHandle) -> Option<VertexHandle> {
        let he = self.half_edge(h)?;
        if self.is_valid_vertex(he.origin) {
            Some(he.origin)
        } else {
            None
        }
    }

    /// Returns the target (end) vertex of half-edge `h` using twin or next relations.
    #[inline]
    pub fn half_edge_target(&self, h: HalfEdgeHandle) -> Option<VertexHandle> {
        let he = self.half_edge(h)?;
        if let Some(twin) = self.half_edge(he.twin)
            && self.is_valid_vertex(twin.origin)
        {
            return Some(twin.origin);
        }
        if let Some(next) = self.half_edge(he.next)
            && self.is_valid_vertex(next.origin)
        {
            return Some(next.origin);
        }
        None
    }

    /// Returns the start and end vertex pair (origin, target) of half-edge `h`.
    #[inline]
    pub fn half_edge_endpoints(&self, h: HalfEdgeHandle) -> Option<(VertexHandle, VertexHandle)> {
        let origin = self.half_edge_origin(h)?;
        let target = self.half_edge_target(h)?;
        Some((origin, target))
    }

    // --- Advanced DCC Loops & Shortest Paths ---

    /// Traverses a quad edge loop starting from edge `e`.
    ///
    /// Returns an ordered list of edges forming an edge loop passing through `start_edge`.
    pub fn edge_loop(&self, start_edge: EdgeHandle) -> Vec<EdgeHandle> {
        self.edge_loop_path(start_edge).into_edges()
    }

    /// Returns the comprehensive topological edge loop path starting from `start_edge`.
    pub fn edge_loop_path(&self, start_edge: EdgeHandle) -> EdgeSelectionPath {
        selection::edge_loop(self, start_edge)
    }

    /// Returns an ordered list of edges forming an edge ring passing through `start_edge`.
    pub fn edge_ring(&self, start_edge: EdgeHandle) -> Vec<EdgeHandle> {
        self.edge_ring_path(start_edge).into_edges()
    }

    /// Returns the comprehensive topological edge ring path starting from `start_edge`.
    pub fn edge_ring_path(&self, start_edge: EdgeHandle) -> EdgeSelectionPath {
        selection::edge_ring(self, start_edge)
    }

    /// Returns the next edge continuing an edge loop arriving at `at_vertex` from `edge`.
    pub fn next_loop_edge(
        &self,
        edge: EdgeHandle,
        at_vertex: VertexHandle,
    ) -> (Option<EdgeHandle>, TerminationReason) {
        selection::next_loop_edge(self, edge, at_vertex)
    }

    /// Returns the next edge continuing an edge ring from `edge` stepping across `across_face`.
    pub fn next_ring_edge(
        &self,
        edge: EdgeHandle,
        across_face: FaceHandle,
    ) -> (Option<EdgeHandle>, TerminationReason) {
        selection::next_ring_edge(self, edge, across_face)
    }

    /// Discovers and returns all closed boundary loops (cycles of virtual boundary half-edges).
    pub fn boundary_loops(&self) -> Vec<Vec<HalfEdgeHandle>> {
        let mut loops = Vec::new();
        let mut visited = HashSet::new();

        for h in self.half_edges() {
            if let Some(he) = self.half_edge(h)
                && he.is_boundary()
                && !visited.contains(&h)
            {
                let mut current_loop = Vec::new();
                let mut curr = h;

                while visited.insert(curr) {
                    current_loop.push(curr);
                    let Some(he_curr) = self.half_edge(curr) else {
                        break;
                    };
                    if he_curr.next == h || !self.is_valid_half_edge(he_curr.next) {
                        break;
                    }
                    curr = he_curr.next;
                }

                if !current_loop.is_empty() {
                    loops.push(current_loop);
                }
            }
        }

        loops
    }

    /// Returns all boundary half-edges in the mesh (half-edges with `face == None`).
    pub fn boundary_half_edges(&self) -> Vec<HalfEdgeHandle> {
        self.half_edges()
            .filter(|&h| {
                if let Some(he) = self.half_edge(h) {
                    he.is_boundary()
                } else {
                    false
                }
            })
            .collect()
    }

    /// Returns the number of distinct closed boundary loops in the mesh.
    #[inline]
    pub fn boundary_loop_count(&self) -> usize {
        self.boundary_loops().len()
    }

    /// Traverses a single boundary loop starting from `start_he`.
    ///
    /// Returns the ordered cycle of boundary half-edges, or an error if invalid/unclosed.
    pub fn boundary_loop(
        &self,
        start_he: HalfEdgeHandle,
    ) -> Result<Vec<HalfEdgeHandle>, TopologyError> {
        if !self.is_valid_half_edge(start_he) {
            return Err(TopologyError::InvalidHalfEdge(start_he));
        }

        let he_init = self.half_edge(start_he).unwrap();
        if !he_init.is_boundary() {
            return Err(TopologyError::InvalidBoundaryLoop(format!(
                "Half-edge {start_he} is not a boundary half-edge (has face {:?})",
                he_init.face
            )));
        }

        let mut loop_hes = Vec::new();
        let mut visited = HashSet::new();
        let mut curr = start_he;
        let max_steps = self.half_edge_count().max(16);
        let mut closed = false;

        while loop_hes.len() < max_steps {
            if !visited.insert(curr) {
                if curr == start_he {
                    closed = true;
                }
                break;
            }

            loop_hes.push(curr);

            let Some(he) = self.half_edge(curr) else {
                return Err(TopologyError::BrokenCycle(format!(
                    "Boundary cycle references invalid half-edge {curr}"
                )));
            };

            if !he.is_boundary() {
                return Err(TopologyError::InvalidBoundaryLoop(format!(
                    "Half-edge {curr} in boundary cycle unexpectedly has face {:?}",
                    he.face
                )));
            }

            let next_h = he.next;
            if !self.is_valid_half_edge(next_h) {
                return Err(TopologyError::BrokenCycle(format!(
                    "Boundary cycle half-edge {curr} next handle {next_h} is invalid"
                )));
            }

            if next_h == start_he {
                closed = true;
                break;
            }

            curr = next_h;
        }

        if !closed {
            if loop_hes.len() >= max_steps {
                return Err(TopologyError::TraversalLimitExceeded(format!(
                    "Boundary cycle starting at {start_he} exceeded step limit {max_steps}"
                )));
            }
            return Err(TopologyError::BrokenCycle(format!(
                "Boundary cycle starting at {start_he} is unclosed"
            )));
        }

        Ok(loop_hes)
    }

    /// Returns the number of half-edges in the boundary loop starting from `start_he`.
    #[inline]
    pub fn boundary_loop_len(&self, start_he: HalfEdgeHandle) -> Result<usize, TopologyError> {
        self.boundary_loop(start_he).map(|l| l.len())
    }

    /// Finds the shortest topological edge path between `start` and `goal` vertices using BFS.
    pub fn shortest_edge_path(
        &self,
        start: VertexHandle,
        goal: VertexHandle,
    ) -> Option<Vec<EdgeHandle>> {
        if !self.is_valid_vertex(start) || !self.is_valid_vertex(goal) {
            return None;
        }
        if start == goal {
            return Some(Vec::new());
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        // Maps child_vertex -> (parent_vertex, connecting_edge)
        let mut came_from: std::collections::HashMap<VertexHandle, (VertexHandle, EdgeHandle)> =
            std::collections::HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        let mut found = false;

        while let Some(curr) = queue.pop_front() {
            if curr == goal {
                found = true;
                break;
            }

            for out_he in self.outgoing_half_edges(curr) {
                let Some(he) = self.half_edge(out_he) else {
                    continue;
                };
                let Some(twin) = self.half_edge(he.twin) else {
                    continue;
                };
                let neighbor = twin.origin;

                if visited.insert(neighbor) {
                    came_from.insert(neighbor, (curr, he.edge));
                    queue.push_back(neighbor);
                }
            }
        }

        if !found {
            return None;
        }

        // Reconstruct path backwards
        let mut path = Vec::new();
        let mut curr = goal;
        while curr != start {
            if let Some(&(parent, edge)) = came_from.get(&curr) {
                path.push(edge);
                curr = parent;
            } else {
                break;
            }
        }

        path.reverse();
        Some(path)
    }

    /// Finds the shortest vertex path between `start` and `goal` vertices using BFS.
    pub fn shortest_vertex_path(
        &self,
        start: VertexHandle,
        goal: VertexHandle,
    ) -> Option<Vec<VertexHandle>> {
        if !self.is_valid_vertex(start) || !self.is_valid_vertex(goal) {
            return None;
        }
        if start == goal {
            return Some(vec![start]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut came_from: std::collections::HashMap<VertexHandle, VertexHandle> =
            std::collections::HashMap::new();

        queue.push_back(start);
        visited.insert(start);
        let mut found = false;

        while let Some(curr) = queue.pop_front() {
            if curr == goal {
                found = true;
                break;
            }
            for neighbor in self.adjacent_vertices(curr) {
                if visited.insert(neighbor) {
                    came_from.insert(neighbor, curr);
                    queue.push_back(neighbor);
                }
            }
        }

        if !found {
            return None;
        }

        let mut path = Vec::new();
        let mut curr = goal;
        path.push(curr);
        while curr != start {
            if let Some(&parent) = came_from.get(&curr) {
                path.push(parent);
                curr = parent;
            } else {
                break;
            }
        }
        path.reverse();
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::TopologyBuilder;

    #[test]
    fn test_traversal_triangle_adjacent_vertices() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let v0 = mesh.vertices().next().unwrap();
        let adj = mesh.adjacent_vertices(v0);
        assert_eq!(adj.len(), 2);
        assert!(adj.contains(&VertexHandle::new(1, 0)));
        assert!(adj.contains(&VertexHandle::new(2, 0)));
    }

    #[test]
    fn test_traversal_quad_adjacent_vertices() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let v0 = mesh.vertices().next().unwrap();
        let adj = mesh.adjacent_vertices(v0);
        assert_eq!(adj.len(), 2);
        assert!(adj.contains(&VertexHandle::new(1, 0)));
        assert!(adj.contains(&VertexHandle::new(3, 0)));
    }

    #[test]
    fn test_traversal_adjacent_faces() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let f0 = FaceHandle::new(0, 0);
        let f1 = FaceHandle::new(1, 0);

        let adj_to_f0 = mesh.face_adjacent_faces(f0);
        assert_eq!(adj_to_f0, vec![f1]);

        let v0 = VertexHandle::new(0, 0);
        let adj_to_v0 = mesh.adjacent_faces(v0);
        assert_eq!(adj_to_v0.len(), 2);
        assert!(adj_to_v0.contains(&f0));
        assert!(adj_to_v0.contains(&f1));

        // Shared interior edge between v0 and v2
        let shared_edge = mesh
            .edges()
            .find(|&e| !mesh.is_edge_boundary(e))
            .expect("Shared edge exists");
        let edge_faces = mesh.edge_adjacent_faces(shared_edge);
        assert_eq!(edge_faces.len(), 2);
        assert!(edge_faces.contains(&f0));
        assert!(edge_faces.contains(&f1));
    }

    #[test]
    fn test_traversal_face_edges() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let f = FaceHandle::new(0, 0);
        let edges = mesh.face_edges(f);
        assert_eq!(edges.len(), 4);
        assert_eq!(mesh.face_side_count(f), 4);
    }

    #[test]
    fn test_traversal_next_prev_twin() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let h0 = HalfEdgeHandle::new(0, 0);
        let h_next = mesh.half_edge_next(h0).expect("next exists");
        let h_prev = mesh.half_edge_prev(h_next).expect("prev exists");
        assert_eq!(h_prev, h0);

        let h_twin = mesh.half_edge_twin(h0).expect("twin exists");
        assert_eq!(mesh.half_edge_twin(h_twin), Some(h0));
    }

    #[test]
    fn test_traversal_half_edge_endpoints() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let h0 = HalfEdgeHandle::new(0, 0);
        let origin = mesh.half_edge_origin(h0).unwrap();
        let target = mesh.half_edge_target(h0).unwrap();
        assert_eq!(origin, VertexHandle::new(0, 0));
        assert_eq!(target, VertexHandle::new(1, 0));
        assert_eq!(mesh.half_edge_endpoints(h0), Some((origin, target)));
    }

    #[test]
    fn test_traversal_face_loop() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let f = FaceHandle::new(0, 0);
        let hes = mesh.face_half_edges(f);
        assert_eq!(hes.len(), 3);
        assert_eq!(mesh.face_side_count(f), 3);

        let hes_safe = mesh.face_loop_safe(f).expect("face loop is valid");
        assert_eq!(hes_safe, hes);

        let verts = mesh.face_vertices(f);
        assert_eq!(
            verts,
            vec![
                VertexHandle::new(0, 0),
                VertexHandle::new(1, 0),
                VertexHandle::new(2, 0)
            ]
        );
    }

    #[test]
    fn test_traversal_single_boundary_loop() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let bnd_hes = mesh.boundary_half_edges();
        assert_eq!(bnd_hes.len(), 3);
        assert_eq!(mesh.boundary_loop_count(), 1);

        let start_bnd = bnd_hes[0];
        let bnd_loop = mesh.boundary_loop(start_bnd).expect("valid boundary loop");
        assert_eq!(bnd_loop.len(), 3);
        assert_eq!(mesh.boundary_loop_len(start_bnd).unwrap(), 3);
    }

    #[test]
    fn test_traversal_multiple_boundary_loops() {
        let pos = vec![
            // Triangle 1
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            // Triangle 2
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(11.0, 0.0, 0.0),
            Vec3::new(10.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![3, 4, 5]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let loops = mesh.boundary_loops();
        assert_eq!(loops.len(), 2);
        assert_eq!(mesh.boundary_loop_count(), 2);
        assert_eq!(loops[0].len(), 3);
        assert_eq!(loops[1].len(), 3);
    }

    #[test]
    fn test_traversal_no_duplicates_in_adjacencies() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let v0 = VertexHandle::new(0, 0);
        let adj_verts = mesh.adjacent_vertices(v0);
        let mut set = HashSet::new();
        for v in &adj_verts {
            assert!(set.insert(*v), "Duplicate vertex found in adjacencies");
        }

        let adj_faces = mesh.adjacent_faces(v0);
        let mut face_set = HashSet::new();
        for f in &adj_faces {
            assert!(
                face_set.insert(*f),
                "Duplicate face found in vertex adjacent faces"
            );
        }
    }

    #[test]
    fn test_traversal_invalid_handle() {
        let mesh = HalfEdgeMesh::new();

        let bogus_v = VertexHandle::new(999, 0);
        assert!(mesh.outgoing_half_edges(bogus_v).is_empty());
        assert!(mesh.adjacent_vertices(bogus_v).is_empty());
        assert!(mesh.adjacent_faces(bogus_v).is_empty());
        assert!(mesh.vertex_adjacent_edges(bogus_v).is_empty());

        let bogus_e = EdgeHandle::new(999, 0);
        assert!(mesh.edge_half_edges(bogus_e).is_none());
        assert!(mesh.edge_vertices(bogus_e).is_none());
        assert!(mesh.edge_faces(bogus_e).is_none());
        assert!(mesh.edge_adjacent_faces(bogus_e).is_empty());

        let bogus_f = FaceHandle::new(999, 0);
        assert!(mesh.face_half_edges(bogus_f).is_empty());
        assert!(mesh.face_vertices(bogus_f).is_empty());
        assert!(mesh.face_adjacent_faces(bogus_f).is_empty());
        assert!(mesh.face_loop_safe(bogus_f).is_err());

        let bogus_h = HalfEdgeHandle::new(999, 0);
        assert!(mesh.half_edge_next(bogus_h).is_none());
        assert!(mesh.half_edge_prev(bogus_h).is_none());
        assert!(mesh.half_edge_twin(bogus_h).is_none());
        assert!(mesh.half_edge_origin(bogus_h).is_none());
        assert!(mesh.half_edge_target(bogus_h).is_none());
        assert!(mesh.boundary_loop(bogus_h).is_err());
    }

    #[test]
    fn test_traversal_broken_cycle_safety() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let bnd_he = mesh.boundary_half_edges()[0];
        // Break the next pointer of the boundary half-edge
        mesh.half_edges[bnd_he.index()].next = HalfEdgeHandle::new(999, 0);

        let res = mesh.boundary_loop(bnd_he);
        assert!(matches!(res, Err(TopologyError::BrokenCycle(_))));

        // Break face cycle
        let f0 = FaceHandle::new(0, 0);
        let f0_he = mesh.face_half_edges(f0)[0];
        mesh.half_edges[f0_he.index()].next = HalfEdgeHandle::new(999, 0);
        let f_res = mesh.face_loop_safe(f0);
        assert!(matches!(f_res, Err(TopologyError::BrokenCycle(_))));
    }

    #[test]
    fn test_traversal_deterministic_order() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let f = FaceHandle::new(0, 0);
        let verts1 = mesh.face_vertices(f);
        let verts2 = mesh.face_vertices(f);
        let verts3 = mesh.face_vertices(f);
        assert_eq!(verts1, verts2);
        assert_eq!(verts2, verts3);

        let v0 = VertexHandle::new(0, 0);
        let adj1 = mesh.adjacent_vertices(v0);
        let adj2 = mesh.adjacent_vertices(v0);
        assert_eq!(adj1, adj2);
    }
}
