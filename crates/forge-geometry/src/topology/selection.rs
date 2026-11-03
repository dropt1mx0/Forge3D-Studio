//! # Topological Selection & Detection
//!
//! Algorithms for deterministic edge loop and edge ring detection on a `HalfEdgeMesh`.
//!
//! - **Edge Loop**: A sequence of edges connected end-to-end at shared vertices,
//!   stepping across 4-valence interior vertices or along regular boundary loops.
//! - **Edge Ring**: A sequence of parallel edges separated by quad faces,
//!   stepping across opposite edges in quadrilateral polygons.

use std::collections::HashSet;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::handles::{EdgeHandle, FaceHandle, VertexHandle};
use super::mesh::HalfEdgeMesh;

/// Reasons why edge loop or edge ring traversal terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TerminationReason {
    /// Traversal reached an open boundary or hole where no further regular continuation exists.
    BoundaryReached,
    /// Traversal successfully closed upon itself, forming a complete topological cycle.
    ClosedLoop,
    /// Traversal encountered a triangular face (3 vertices) where regular quad continuation is ill-defined.
    TriangleEncountered,
    /// Traversal encountered an N-gon face (> 4 vertices) where regular quad continuation is ill-defined.
    NgonEncountered,
    /// Traversal encountered non-manifold connectivity (e.g. edge with > 2 faces or inconsistent orientation).
    NonManifoldEncountered,
    /// Traversal encountered an ambiguous continuation (e.g. extraordinary vertex / pole with valence != 4).
    AmbiguousContinuation,
    /// Traversal failed or stopped due to an invalid handle or corrupted mesh topology.
    InvalidTopology,
}

/// The result of an edge loop or edge ring detection traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeSelectionPath {
    edges: Vec<EdgeHandle>,
    start_edge: EdgeHandle,
    is_closed: bool,
    termination: TerminationReason,
    forward_termination: TerminationReason,
    backward_termination: TerminationReason,
}

impl EdgeSelectionPath {
    /// Constructs a new `EdgeSelectionPath`.
    pub fn new(
        edges: Vec<EdgeHandle>,
        start_edge: EdgeHandle,
        is_closed: bool,
        termination: TerminationReason,
        forward_termination: TerminationReason,
        backward_termination: TerminationReason,
    ) -> Self {
        Self {
            edges,
            start_edge,
            is_closed,
            termination,
            forward_termination,
            backward_termination,
        }
    }

    /// Creates an empty path representing an invalid or failed query.
    pub fn invalid(start_edge: EdgeHandle) -> Self {
        Self {
            edges: Vec::new(),
            start_edge,
            is_closed: false,
            termination: TerminationReason::InvalidTopology,
            forward_termination: TerminationReason::InvalidTopology,
            backward_termination: TerminationReason::InvalidTopology,
        }
    }

    /// Ordered sequence of edges visited during traversal.
    #[inline]
    pub fn edges(&self) -> &[EdgeHandle] {
        &self.edges
    }

    /// The edge where traversal originated.
    #[inline]
    pub fn start_edge(&self) -> EdgeHandle {
        self.start_edge
    }

    /// Whether traversal successfully completed a closed loop/ring.
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Primary reason why traversal stopped.
    #[inline]
    pub fn termination(&self) -> TerminationReason {
        self.termination
    }

    /// Termination reason in the forward traversal direction.
    #[inline]
    pub fn forward_termination(&self) -> TerminationReason {
        self.forward_termination
    }

    /// Termination reason in the backward traversal direction.
    #[inline]
    pub fn backward_termination(&self) -> TerminationReason {
        self.backward_termination
    }

    /// Number of edges in the resulting path.
    #[inline]
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// True if the resulting path contains no edges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Checks if a specific `EdgeHandle` is present in the path.
    #[inline]
    pub fn contains(&self, edge: EdgeHandle) -> bool {
        self.edges.contains(&edge)
    }

    /// Consumes the path and returns the ordered vector of edges.
    #[inline]
    pub fn into_edges(self) -> Vec<EdgeHandle> {
        self.edges
    }
}

// ---------------------------------------------------------------------------
// Step Continuations
// ---------------------------------------------------------------------------

/// Returns the next edge continuing an edge loop arriving at `at_vertex` from `edge`,
/// along with the termination reason if traversal cannot continue.
pub fn next_loop_edge(
    mesh: &HalfEdgeMesh,
    edge: EdgeHandle,
    at_vertex: VertexHandle,
) -> (Option<EdgeHandle>, TerminationReason) {
    if !mesh.is_valid_edge(edge) || !mesh.is_valid_vertex(at_vertex) {
        return (None, TerminationReason::InvalidTopology);
    }

    let Some((v0, v1)) = mesh.edge_vertices(edge) else {
        return (None, TerminationReason::InvalidTopology);
    };
    if v0 != at_vertex && v1 != at_vertex {
        return (None, TerminationReason::InvalidTopology);
    }

    let Some((h0, h1)) = mesh.edge_half_edges(edge) else {
        return (None, TerminationReason::InvalidTopology);
    };
    let Some(he0) = mesh.half_edge(h0) else {
        return (None, TerminationReason::InvalidTopology);
    };
    let Some(he1) = mesh.half_edge(h1) else {
        return (None, TerminationReason::InvalidTopology);
    };

    // Determine half-edge entering at_vertex
    let h_in = if he1.origin == at_vertex { h0 } else { h1 };
    let h_twin = if h_in == h0 { h1 } else { h0 };

    // Inspect circulating outgoing half-edges around at_vertex
    let out_hes = mesh.outgoing_half_edges(at_vertex);
    let val = out_hes.len();
    if val == 0 {
        return (None, TerminationReason::InvalidTopology);
    }

    // Check for non-quad faces incident to at_vertex
    for &h in &out_hes {
        let Some(he) = mesh.half_edge(h) else {
            return (None, TerminationReason::InvalidTopology);
        };
        if let Some(f) = he.face {
            let f_sides = mesh.face_vertex_count(f);
            if f_sides == 3 {
                return (None, TerminationReason::TriangleEncountered);
            }
            if f_sides > 4 {
                return (None, TerminationReason::NgonEncountered);
            }
            if f_sides < 3 {
                return (None, TerminationReason::InvalidTopology);
            }
        }
    }

    let is_boundary_edge = he0.face.is_none() || he1.face.is_none();

    if is_boundary_edge {
        // Boundary edge arriving at boundary vertex: follow boundary loop
        let bnd_he_h = if he0.face.is_none() { h0 } else { h1 };
        let Some(bnd_he) = mesh.half_edge(bnd_he_h) else {
            return (None, TerminationReason::InvalidTopology);
        };

        if val == 2 {
            // Corner of an open mesh (e.g. 90 degree corner of a quad grid): loop terminates at corner
            return (None, TerminationReason::BoundaryReached);
        }
        if val == 3 {
            // Regular boundary vertex: follow the continuing boundary half-edge
            let next_bnd_h = if bnd_he.origin == at_vertex {
                bnd_he.prev
            } else {
                bnd_he.next
            };
            let Some(next_he) = mesh.half_edge(next_bnd_h) else {
                return (None, TerminationReason::InvalidTopology);
            };
            return (Some(next_he.edge), TerminationReason::BoundaryReached);
        }

        // Higher valence or irregular boundary configuration
        return (None, TerminationReason::AmbiguousContinuation);
    }

    // Interior edge: if the vertex lies on the mesh boundary, interior loop terminates
    if mesh.is_vertex_boundary(at_vertex) {
        return (None, TerminationReason::BoundaryReached);
    }

    // Regular quad interior vertex requires valence == 4
    if val != 4 {
        return (None, TerminationReason::AmbiguousContinuation);
    }

    // Find position of the edge we came from (h_twin originates at at_vertex)
    let Some(pos) = out_hes.iter().position(|&h| h == h_twin) else {
        return (None, TerminationReason::InvalidTopology);
    };

    // Opposite edge across valence-4 vertex is at (pos + 2) % 4
    let opp_h = out_hes[(pos + 2) % 4];
    let Some(opp_he) = mesh.half_edge(opp_h) else {
        return (None, TerminationReason::InvalidTopology);
    };

    (Some(opp_he.edge), TerminationReason::BoundaryReached)
}

/// Returns the next edge continuing an edge ring from `edge` stepping across `across_face`,
/// along with the termination reason if traversal cannot continue.
pub fn next_ring_edge(
    mesh: &HalfEdgeMesh,
    edge: EdgeHandle,
    across_face: FaceHandle,
) -> (Option<EdgeHandle>, TerminationReason) {
    if !mesh.is_valid_edge(edge) || !mesh.is_valid_face(across_face) {
        return (None, TerminationReason::InvalidTopology);
    }

    let f_edges = mesh.face_edges(across_face);
    if !f_edges.contains(&edge) {
        return (None, TerminationReason::InvalidTopology);
    }

    let f_count = f_edges.len();
    if f_count == 3 {
        return (None, TerminationReason::TriangleEncountered);
    }
    if f_count > 4 {
        return (None, TerminationReason::NgonEncountered);
    }
    if f_count < 3 {
        return (None, TerminationReason::InvalidTopology);
    }

    // Quad face: find opposite half-edge in 4-cycle
    let f_hes = mesh.face_half_edges(across_face);
    if f_hes.len() != 4 {
        return (None, TerminationReason::InvalidTopology);
    }

    let Some(pos) = f_hes
        .iter()
        .position(|&h| mesh.half_edge(h).map(|he| he.edge) == Some(edge))
    else {
        return (None, TerminationReason::InvalidTopology);
    };

    let opp_he_h = f_hes[(pos + 2) % 4];
    let Some(opp_he) = mesh.half_edge(opp_he_h) else {
        return (None, TerminationReason::InvalidTopology);
    };

    (Some(opp_he.edge), TerminationReason::BoundaryReached)
}

// ---------------------------------------------------------------------------
// Traversal Algorithms
// ---------------------------------------------------------------------------

/// Traverses an edge loop starting from `start_edge` in both directions.
pub fn edge_loop(mesh: &HalfEdgeMesh, start_edge: EdgeHandle) -> EdgeSelectionPath {
    if !mesh.is_valid_edge(start_edge) {
        return EdgeSelectionPath::invalid(start_edge);
    }

    let Some((v0, v1)) = mesh.edge_vertices(start_edge) else {
        return EdgeSelectionPath::invalid(start_edge);
    };

    let max_steps = mesh.edge_count().max(16);
    let mut visited = HashSet::new();
    visited.insert(start_edge);

    // Direction A: forward through v1
    let mut curr_edge = start_edge;
    let mut curr_vertex = v1;
    let mut forward_edges = Vec::new();
    let mut forward_termination = TerminationReason::BoundaryReached;
    let mut is_closed = false;

    for _ in 0..max_steps {
        let (next_opt, term) = next_loop_edge(mesh, curr_edge, curr_vertex);
        forward_termination = term;
        let Some(next_edge) = next_opt else {
            break;
        };

        if next_edge == start_edge {
            is_closed = true;
            forward_termination = TerminationReason::ClosedLoop;
            break;
        }

        if !visited.insert(next_edge) {
            is_closed = true;
            forward_termination = TerminationReason::ClosedLoop;
            break;
        }

        forward_edges.push(next_edge);

        let Some((ev0, ev1)) = mesh.edge_vertices(next_edge) else {
            forward_termination = TerminationReason::InvalidTopology;
            break;
        };
        curr_vertex = if ev0 == curr_vertex { ev1 } else { ev0 };
        curr_edge = next_edge;
    }

    if is_closed {
        let mut edges = vec![start_edge];
        edges.extend(forward_edges);
        return EdgeSelectionPath::new(
            edges,
            start_edge,
            true,
            TerminationReason::ClosedLoop,
            TerminationReason::ClosedLoop,
            TerminationReason::ClosedLoop,
        );
    }

    // Direction B: backward through v0
    let mut curr_edge = start_edge;
    let mut curr_vertex = v0;
    let mut backward_edges = Vec::new();
    let mut backward_termination = TerminationReason::BoundaryReached;

    for _ in 0..max_steps {
        let (next_opt, term) = next_loop_edge(mesh, curr_edge, curr_vertex);
        backward_termination = term;
        let Some(next_edge) = next_opt else {
            break;
        };

        if next_edge == start_edge {
            is_closed = true;
            backward_termination = TerminationReason::ClosedLoop;
            break;
        }

        if !visited.insert(next_edge) {
            is_closed = true;
            backward_termination = TerminationReason::ClosedLoop;
            break;
        }

        backward_edges.push(next_edge);

        let Some((ev0, ev1)) = mesh.edge_vertices(next_edge) else {
            backward_termination = TerminationReason::InvalidTopology;
            break;
        };
        curr_vertex = if ev0 == curr_vertex { ev1 } else { ev0 };
        curr_edge = next_edge;
    }

    let mut all_edges = Vec::new();
    all_edges.extend(backward_edges.into_iter().rev());
    all_edges.push(start_edge);
    all_edges.extend(forward_edges);

    let term = resolve_termination(is_closed, forward_termination, backward_termination);
    EdgeSelectionPath::new(
        all_edges,
        start_edge,
        is_closed,
        term,
        forward_termination,
        backward_termination,
    )
}

/// Traverses an edge ring starting from `start_edge` in both directions.
pub fn edge_ring(mesh: &HalfEdgeMesh, start_edge: EdgeHandle) -> EdgeSelectionPath {
    if !mesh.is_valid_edge(start_edge) {
        return EdgeSelectionPath::invalid(start_edge);
    }

    let Some((f0_opt, f1_opt)) = mesh.edge_faces(start_edge) else {
        return EdgeSelectionPath::invalid(start_edge);
    };

    let max_steps = mesh.edge_count().max(16);
    let mut visited = HashSet::new();
    visited.insert(start_edge);

    // Direction A: through f0
    let mut forward_edges = Vec::new();
    let mut forward_termination = TerminationReason::BoundaryReached;
    let mut is_closed = false;

    if let Some(mut curr_face) = f0_opt {
        let mut curr_edge = start_edge;
        for _ in 0..max_steps {
            let (next_opt, term) = next_ring_edge(mesh, curr_edge, curr_face);
            forward_termination = term;
            let Some(next_edge) = next_opt else {
                break;
            };

            if next_edge == start_edge {
                is_closed = true;
                forward_termination = TerminationReason::ClosedLoop;
                break;
            }

            if !visited.insert(next_edge) {
                is_closed = true;
                forward_termination = TerminationReason::ClosedLoop;
                break;
            }

            forward_edges.push(next_edge);

            let Some((nf0, nf1)) = mesh.edge_faces(next_edge) else {
                forward_termination = TerminationReason::InvalidTopology;
                break;
            };

            let other_face_opt = if nf0 == Some(curr_face) { nf1 } else { nf0 };
            let Some(next_face) = other_face_opt else {
                forward_termination = TerminationReason::BoundaryReached;
                break;
            };

            curr_face = next_face;
            curr_edge = next_edge;
        }
    } else {
        forward_termination = TerminationReason::BoundaryReached;
    }

    if is_closed {
        let mut edges = vec![start_edge];
        edges.extend(forward_edges);
        return EdgeSelectionPath::new(
            edges,
            start_edge,
            true,
            TerminationReason::ClosedLoop,
            TerminationReason::ClosedLoop,
            TerminationReason::ClosedLoop,
        );
    }

    // Direction B: through f1
    let mut backward_edges = Vec::new();
    let mut backward_termination = TerminationReason::BoundaryReached;

    if let Some(mut curr_face) = f1_opt {
        let mut curr_edge = start_edge;
        for _ in 0..max_steps {
            let (next_opt, term) = next_ring_edge(mesh, curr_edge, curr_face);
            backward_termination = term;
            let Some(next_edge) = next_opt else {
                break;
            };

            if next_edge == start_edge {
                is_closed = true;
                backward_termination = TerminationReason::ClosedLoop;
                break;
            }

            if !visited.insert(next_edge) {
                is_closed = true;
                backward_termination = TerminationReason::ClosedLoop;
                break;
            }

            backward_edges.push(next_edge);

            let Some((nf0, nf1)) = mesh.edge_faces(next_edge) else {
                backward_termination = TerminationReason::InvalidTopology;
                break;
            };

            let other_face_opt = if nf0 == Some(curr_face) { nf1 } else { nf0 };
            let Some(next_face) = other_face_opt else {
                backward_termination = TerminationReason::BoundaryReached;
                break;
            };

            curr_face = next_face;
            curr_edge = next_edge;
        }
    } else {
        backward_termination = TerminationReason::BoundaryReached;
    }

    let mut all_edges = Vec::new();
    all_edges.extend(backward_edges.into_iter().rev());
    all_edges.push(start_edge);
    all_edges.extend(forward_edges);

    let term = resolve_termination(is_closed, forward_termination, backward_termination);
    EdgeSelectionPath::new(
        all_edges,
        start_edge,
        is_closed,
        term,
        forward_termination,
        backward_termination,
    )
}

fn resolve_termination(
    is_closed: bool,
    fwd: TerminationReason,
    bwd: TerminationReason,
) -> TerminationReason {
    if is_closed {
        return TerminationReason::ClosedLoop;
    }
    for reason in [
        TerminationReason::InvalidTopology,
        TerminationReason::NonManifoldEncountered,
        TerminationReason::TriangleEncountered,
        TerminationReason::NgonEncountered,
        TerminationReason::AmbiguousContinuation,
    ] {
        if fwd == reason || bwd == reason {
            return reason;
        }
    }
    TerminationReason::BoundaryReached
}

// ---------------------------------------------------------------------------
// Unit Tests (Phase 6.5 Requirements 1-12)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::HalfEdgeHandle;
    use crate::topology::builder::TopologyBuilder;
    use forge_math::Vec3;

    // Helper: builds an M x N quad grid (M rows, N cols of quads)
    fn make_quad_grid(rows: usize, cols: usize) -> HalfEdgeMesh {
        let mut positions = Vec::new();
        for r in 0..=rows {
            for c in 0..=cols {
                positions.push(Vec3::new(c as f32, r as f32, 0.0));
            }
        }

        let stride = cols + 1;
        let mut faces = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let v00 = r * stride + c;
                let v01 = r * stride + c + 1;
                let v11 = (r + 1) * stride + c + 1;
                let v10 = (r + 1) * stride + c;
                faces.push(vec![v00, v01, v11, v10]);
            }
        }

        TopologyBuilder::build(&positions, &faces).expect("quad grid build")
    }

    // Helper: builds an open cylinder (tube) with `cols` segments around circumference and `rows` along height
    fn make_quad_cylinder(rows: usize, cols: usize) -> HalfEdgeMesh {
        let mut positions = Vec::new();
        let radius = 1.0;
        let angle_step = std::f32::consts::TAU / cols as f32;

        for r in 0..=rows {
            for c in 0..cols {
                let angle = c as f32 * angle_step;
                let x = radius * angle.cos();
                let z = radius * angle.sin();
                let y = r as f32;
                positions.push(Vec3::new(x, y, z));
            }
        }

        let mut faces = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let next_c = (c + 1) % cols;
                let v00 = r * cols + c;
                let v01 = r * cols + next_c;
                let v11 = (r + 1) * cols + next_c;
                let v10 = (r + 1) * cols + c;
                faces.push(vec![v00, v01, v11, v10]);
            }
        }

        TopologyBuilder::build(&positions, &faces).expect("quad cylinder build")
    }

    // Test 1: Edge Loop on regular Quad mesh
    #[test]
    fn test_1_edge_loop_regular_quad_mesh() {
        // 3x3 quad grid
        let mesh = make_quad_grid(3, 3);
        // Find an interior horizontal edge between (v4, v5) or (v5, v6)
        // Vertices at r=1: 4, 5, 6, 7
        let e_mid = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 5 && v1.index() == 6) || (v0.index() == 6 && v1.index() == 5)
            })
            .expect("middle horizontal edge");

        let path = edge_loop(&mesh, e_mid);
        assert_eq!(path.len(), 3, "loop along r=1 should contain 3 edges");
        assert_eq!(path.start_edge(), e_mid);
        assert!(!path.is_closed());
        assert_eq!(path.termination(), TerminationReason::BoundaryReached);
        assert_eq!(
            path.forward_termination(),
            TerminationReason::BoundaryReached
        );
        assert_eq!(
            path.backward_termination(),
            TerminationReason::BoundaryReached
        );

        // Verify edges are connected head-to-tail
        for window in path.edges().windows(2) {
            let (a0, a1) = mesh.edge_vertices(window[0]).unwrap();
            let (b0, b1) = mesh.edge_vertices(window[1]).unwrap();
            assert!(
                a0 == b0 || a0 == b1 || a1 == b0 || a1 == b1,
                "consecutive loop edges must share a vertex"
            );
        }
    }

    // Test 2: Edge Ring on regular Quad mesh
    #[test]
    fn test_2_edge_ring_regular_quad_mesh() {
        // 3x3 quad grid
        let mesh = make_quad_grid(3, 3);
        // Find interior vertical edge between r=1 and r=2, c=1 (v5, v9)
        let e_vert = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 5 && v1.index() == 9) || (v0.index() == 9 && v1.index() == 5)
            })
            .expect("vertical edge");

        let path = edge_ring(&mesh, e_vert);
        assert_eq!(path.len(), 4, "ring across col 1 should contain 4 edges");
        assert_eq!(path.start_edge(), e_vert);
        assert!(!path.is_closed());
        assert_eq!(path.termination(), TerminationReason::BoundaryReached);

        // Verify edges do NOT share vertices in a pure quad ring
        for window in path.edges().windows(2) {
            let (a0, a1) = mesh.edge_vertices(window[0]).unwrap();
            let (b0, b1) = mesh.edge_vertices(window[1]).unwrap();
            assert!(
                a0 != b0 && a0 != b1 && a1 != b0 && a1 != b1,
                "parallel ring edges across quads do not share endpoints"
            );
        }
    }

    // Test 3: Closed Edge Loop
    #[test]
    fn test_3_closed_edge_loop() {
        // 2 rows, 4 cols cylinder (tube): middle ring at r=1 is internal and closed
        let mesh = make_quad_cylinder(2, 4);
        // Middle ring vertices at r=1: 4, 5, 6, 7
        // Edge between (4, 5)
        let e_closed = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 4 && v1.index() == 5) || (v0.index() == 5 && v1.index() == 4)
            })
            .expect("middle ring edge");

        let path = edge_loop(&mesh, e_closed);
        assert!(path.is_closed(), "cylinder middle loop must be closed");
        assert_eq!(path.len(), 4, "4-segment cylinder loop has 4 edges");
        assert_eq!(path.termination(), TerminationReason::ClosedLoop);
        assert_eq!(path.forward_termination(), TerminationReason::ClosedLoop);
    }

    // Test 4: Closed Edge Ring
    #[test]
    fn test_4_closed_edge_ring() {
        // 1 row, 4 cols cylinder (tube): vertical edges form a closed ring around the cylinder
        let mesh = make_quad_cylinder(1, 4);
        // Vertical edge between top v0 and bot v4
        let e_vert = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 0 && v1.index() == 4) || (v0.index() == 4 && v1.index() == 0)
            })
            .expect("vertical cylinder edge");

        let path = edge_ring(&mesh, e_vert);
        assert!(path.is_closed(), "vertical ring around cylinder must close");
        assert_eq!(path.len(), 4, "4 vertical edges form the ring");
        assert_eq!(path.termination(), TerminationReason::ClosedLoop);
    }

    // Test 5: Traversal from boundary edge
    #[test]
    fn test_5_traversal_from_boundary_edge() {
        let mesh = make_quad_grid(2, 2);
        // Top boundary edge between v0 and v1
        let e_bnd = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 0 && v1.index() == 1) || (v0.index() == 1 && v1.index() == 0)
            })
            .expect("top boundary edge");

        assert!(mesh.is_edge_boundary(e_bnd));

        // Edge Ring from boundary edge: one side has no face, other side steps into mesh
        let ring_path = edge_ring(&mesh, e_bnd);
        assert_eq!(ring_path.len(), 3, "2x2 grid ring has 3 rungs");
        assert_eq!(ring_path.termination(), TerminationReason::BoundaryReached);
        assert_eq!(ring_path.edges()[0], e_bnd);

        // Edge Loop from boundary edge along top boundary: stops at corner (valence 2)
        let loop_path = edge_loop(&mesh, e_bnd);
        assert_eq!(loop_path.len(), 2, "top boundary has 2 edges");
        assert_eq!(loop_path.termination(), TerminationReason::BoundaryReached);
    }

    // Test 6: Traversal on open mesh
    #[test]
    fn test_6_traversal_on_open_mesh() {
        let mesh = make_quad_grid(3, 3);
        // Center vertical edge (5, 9)
        let e_center = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 5 && v1.index() == 9) || (v0.index() == 9 && v1.index() == 5)
            })
            .unwrap();

        let loop_path = edge_loop(&mesh, e_center);
        assert!(!loop_path.is_closed());
        assert_eq!(loop_path.len(), 3, "vertical column has 3 edges");
        assert_eq!(loop_path.termination(), TerminationReason::BoundaryReached);
        assert_eq!(
            loop_path.forward_termination(),
            TerminationReason::BoundaryReached
        );
        assert_eq!(
            loop_path.backward_termination(),
            TerminationReason::BoundaryReached
        );
    }

    // Test 7: Stopping at Triangle
    #[test]
    fn test_7_stopping_at_triangle() {
        // Mesh: Quad [0, 1, 4, 3] and Triangle [1, 2, 4]
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 4, 3], vec![1, 2, 4]];
        let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

        // Edge (0, 3) - left edge of quad
        let e_left = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 0 && v1.index() == 3) || (v0.index() == 3 && v1.index() == 0)
            })
            .unwrap();

        // Edge Ring: opposite edge in quad is (1, 4), then other face of (1, 4) is triangle [1, 2, 4]
        let ring_path = edge_ring(&mesh, e_left);
        assert_eq!(
            ring_path.termination(),
            TerminationReason::TriangleEncountered,
            "Ring entering a triangle must terminate with TriangleEncountered"
        );

        // Edge Loop from (0, 1) towards vertex 1: vertex 1 is incident to triangle [1, 2, 4]
        let e_top = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 0 && v1.index() == 1) || (v0.index() == 1 && v1.index() == 0)
            })
            .unwrap();
        let loop_path = edge_loop(&mesh, e_top);
        assert_eq!(
            loop_path.termination(),
            TerminationReason::TriangleEncountered
        );
    }

    // Test 8: Stopping at N-gon
    #[test]
    fn test_8_stopping_at_ngon() {
        // Mesh: Quad [0, 1, 5, 4] and Pentagon [1, 2, 3, 6, 5]
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.5, 0.5, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 5, 4], vec![1, 2, 3, 6, 5]];
        let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

        // Edge (0, 4) - left edge of quad
        let e_left = mesh
            .edges()
            .find(|&eh| {
                let (v0, v1) = mesh.edge_vertices(eh).unwrap();
                (v0.index() == 0 && v1.index() == 4) || (v0.index() == 4 && v1.index() == 0)
            })
            .unwrap();

        let ring_path = edge_ring(&mesh, e_left);
        assert_eq!(
            ring_path.termination(),
            TerminationReason::NgonEncountered,
            "Ring stepping into a pentagon must terminate with NgonEncountered"
        );
    }

    // Test 9: Prevention of visiting duplicate edges
    #[test]
    fn test_9_prevention_of_duplicate_edges() {
        let mesh = make_quad_cylinder(2, 4);
        for e in mesh.edges() {
            let loop_path = edge_loop(&mesh, e);
            let mut seen = HashSet::new();
            for &edge in loop_path.edges() {
                assert!(seen.insert(edge), "no duplicate edges allowed in loop path");
            }

            let ring_path = edge_ring(&mesh, e);
            let mut seen_ring = HashSet::new();
            for &edge in ring_path.edges() {
                assert!(
                    seen_ring.insert(edge),
                    "no duplicate edges allowed in ring path"
                );
            }
        }
    }

    // Test 10: Passing invalid EdgeHandle
    #[test]
    fn test_10_passing_invalid_edge_handle() {
        let mesh = make_quad_grid(2, 2);

        // Null handle
        let null_path = edge_loop(&mesh, EdgeHandle::NULL);
        assert!(null_path.is_empty());
        assert_eq!(null_path.termination(), TerminationReason::InvalidTopology);

        // Out of bounds handle
        let oob_path = edge_ring(&mesh, EdgeHandle::new(99999, 0));
        assert!(oob_path.is_empty());
        assert_eq!(oob_path.termination(), TerminationReason::InvalidTopology);

        // Wrong generation
        let valid_edge = mesh.edges().next().unwrap();
        let wrong_gen = EdgeHandle::new(valid_edge.raw_index(), valid_edge.generation() + 1);
        let gen_path = edge_loop(&mesh, wrong_gen);
        assert!(gen_path.is_empty());
        assert_eq!(gen_path.termination(), TerminationReason::InvalidTopology);
    }

    // Test 11: Corrupted topological connections
    #[test]
    fn test_11_corrupted_topological_connections() {
        let mut mesh = make_quad_grid(2, 2);
        let e = mesh.edges().next().unwrap();

        // Break twin link on one half-edge
        let (h0, _) = mesh.edge_half_edges(e).unwrap();
        mesh.half_edge_mut(h0).unwrap().twin = HalfEdgeHandle::NULL;

        // Both functions must safely handle broken links without panicking
        let loop_path = edge_loop(&mesh, e);
        assert_eq!(loop_path.termination(), TerminationReason::InvalidTopology);

        let ring_path = edge_ring(&mesh, e);
        assert_eq!(ring_path.termination(), TerminationReason::InvalidTopology);
    }

    // Test 12: Deterministic results
    #[test]
    fn test_12_deterministic_results() {
        let mesh = make_quad_grid(3, 3);
        for e in mesh.edges() {
            let loop_1 = edge_loop(&mesh, e);
            let loop_2 = edge_loop(&mesh, e);
            assert_eq!(
                loop_1, loop_2,
                "edge_loop must produce deterministic results"
            );

            let ring_1 = edge_ring(&mesh, e);
            let ring_2 = edge_ring(&mesh, e);
            assert_eq!(
                ring_1, ring_2,
                "edge_ring must produce deterministic results"
            );
        }
    }
}
