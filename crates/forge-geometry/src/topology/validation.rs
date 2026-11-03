use std::collections::{HashSet, VecDeque};

use super::error::TopologyError;
use super::handles::{FaceHandle, VertexHandle};
use super::mesh::HalfEdgeMesh;

/// Severity level of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueSeverity {
    Error,
    Warning,
}

/// Category of topological validation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    InvalidHandle,
    BrokenTwin,
    BrokenNext,
    BrokenPrev,
    BrokenFaceLoop,
    InvalidEdge,
    InvalidVertex,
    NonManifold,
    Boundary,
    IsolatedVertex,
    OrientationMismatch,
}

/// A specific diagnostic issue identified during topology validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Error,
            category,
            message: message.into(),
        }
    }

    pub fn warning(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            category,
            message: message.into(),
        }
    }
}

/// Comprehensive report detailing validation outcome, checked counts, and identified issues.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub elements_checked: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            elements_checked: 0,
            error_count: 0,
            warning_count: 0,
            issues: Vec::new(),
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    #[inline]
    pub fn elements_checked(&self) -> usize {
        self.elements_checked
    }

    #[inline]
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    #[inline]
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    #[inline]
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    pub fn add_issue(&mut self, issue: ValidationIssue) {
        match issue.severity {
            IssueSeverity::Error => {
                self.error_count += 1;
                self.is_valid = false;
            }
            IssueSeverity::Warning => {
                self.warning_count += 1;
            }
        }
        self.issues.push(issue);
    }
}

/// Independent validator for verifying `HalfEdgeMesh` structural integrity and topological invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TopologyValidator;

impl TopologyValidator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates if the mesh satisfies all critical topological invariants.
    pub fn is_valid(mesh: &HalfEdgeMesh) -> bool {
        Self::validate(mesh).is_valid
    }

    /// Performs comprehensive audit of elements and connectivity, generating a `ValidationReport`.
    pub fn validate(mesh: &HalfEdgeMesh) -> ValidationReport {
        let mut report = ValidationReport::new();

        let num_v = mesh.vertex_count();
        let num_he = mesh.half_edge_count();
        let num_e = mesh.edge_count();
        let num_f = mesh.face_count();
        report.elements_checked = num_v + num_he + num_e + num_f;

        // 1. Audit Vertices
        for vh in mesh.vertices() {
            let Some(v) = mesh.vertex(vh) else {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidHandle,
                    format!("Active vertex handle {vh} missing from array"),
                ));
                continue;
            };

            // Check outgoing half-edge
            if let Some(he_h) = v.half_edge {
                if !mesh.is_valid_half_edge(he_h) {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::InvalidHandle,
                        format!("Vertex {vh} points to invalid outgoing half-edge {he_h}"),
                    ));
                } else if let Some(he) = mesh.half_edge(he_h)
                    && he.origin != vh
                {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::InvalidVertex,
                        format!(
                            "Vertex {vh} outgoing half-edge {he_h} origin is {} instead of {vh}",
                            he.origin
                        ),
                    ));
                }
            } else {
                // Isolated vertex is a valid topological situation (warning, not error)
                report.add_issue(ValidationIssue::warning(
                    IssueCategory::IsolatedVertex,
                    format!("Vertex {vh} is isolated (no incident half-edges)"),
                ));
            }
        }

        // 2. Audit Half-Edges
        for he_h in mesh.half_edges() {
            let Some(he) = mesh.half_edge(he_h) else {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidHandle,
                    format!("Active half-edge handle {he_h} missing from array"),
                ));
                continue;
            };

            // Origin validity
            if !mesh.is_valid_vertex(he.origin) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidVertex,
                    format!("Half-edge {he_h} has invalid origin handle {}", he.origin),
                ));
            }

            // Edge validity
            if !mesh.is_valid_edge(he.edge) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidEdge,
                    format!("Half-edge {he_h} has invalid edge handle {}", he.edge),
                ));
            }

            // Twin involution: twin.twin == self
            if !mesh.is_valid_half_edge(he.twin) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenTwin,
                    format!("Half-edge {he_h} has invalid twin handle {}", he.twin),
                ));
            } else if let Some(twin) = mesh.half_edge(he.twin) {
                if twin.twin != he_h {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::BrokenTwin,
                        format!(
                            "Half-edge {he_h} twin involution mismatch: twin({}).twin == {} != {he_h}",
                            he.twin, twin.twin
                        ),
                    ));
                }
                if twin.edge != he.edge {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::InvalidEdge,
                        format!(
                            "Half-edge {he_h} and twin {} belong to different edges ({} vs {})",
                            he.twin, he.edge, twin.edge
                        ),
                    ));
                }
                // Check relative orientation: if both are incident on faces, their origins cannot be identical
                if he.face.is_some() && twin.face.is_some() && he.origin == twin.origin {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::OrientationMismatch,
                        format!(
                            "Half-edge {he_h} and twin {} share identical origin {}, orientation mismatch",
                            he.twin, he.origin
                        ),
                    ));
                }
            }

            // Next / Prev cycles
            if !mesh.is_valid_half_edge(he.next) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenNext,
                    format!("Half-edge {he_h} has invalid next handle {}", he.next),
                ));
            } else if let Some(next) = mesh.half_edge(he.next)
                && next.prev != he_h
            {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenNext,
                    format!(
                        "Half-edge {he_h} cycle broken: next({}).prev == {} != {he_h}",
                        he.next, next.prev
                    ),
                ));
            }

            if !mesh.is_valid_half_edge(he.prev) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenPrev,
                    format!("Half-edge {he_h} has invalid prev handle {}", he.prev),
                ));
            } else if let Some(prev) = mesh.half_edge(he.prev)
                && prev.next != he_h
            {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenPrev,
                    format!(
                        "Half-edge {he_h} cycle broken: prev({}).next == {} != {he_h}",
                        he.prev, prev.next
                    ),
                ));
            }

            // Face validity
            if let Some(fh) = he.face {
                if !mesh.is_valid_face(fh) {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::BrokenFaceLoop,
                        format!("Half-edge {he_h} points to invalid face handle {fh}"),
                    ));
                }
            } else {
                // Boundary half-edge is a normal structural feature, recorded as Boundary warning
                report.add_issue(ValidationIssue::warning(
                    IssueCategory::Boundary,
                    format!("Half-edge {he_h} is on a boundary loop (face is None)"),
                ));
            }
        }

        // 3. Audit Edges
        for eh in mesh.edges() {
            let Some(edge) = mesh.edge(eh) else {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidHandle,
                    format!("Active edge handle {eh} missing from array"),
                ));
                continue;
            };

            if !mesh.is_valid_half_edge(edge.half_edge) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidEdge,
                    format!("Edge {eh} points to invalid half-edge {}", edge.half_edge),
                ));
            } else if let Some(he) = mesh.half_edge(edge.half_edge)
                && he.edge != eh
            {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidEdge,
                    format!(
                        "Edge {eh} references half-edge {} whose edge field is {} != {eh}",
                        edge.half_edge, he.edge
                    ),
                ));
            }
        }

        // 4. Audit Faces & Face Loops
        for fh in mesh.faces() {
            let Some(face) = mesh.face(fh) else {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::InvalidHandle,
                    format!("Active face handle {fh} missing from array"),
                ));
                continue;
            };

            if !mesh.is_valid_half_edge(face.half_edge) {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenFaceLoop,
                    format!("Face {fh} points to invalid half-edge {}", face.half_edge),
                ));
                continue;
            }

            let start_h = face.half_edge;
            let mut curr_h = start_h;
            let mut count = 0;
            let max_steps = mesh.half_edge_count().max(16);
            let mut visited = HashSet::new();
            let mut closed = false;

            while count < max_steps {
                if !visited.insert(curr_h) {
                    if curr_h != start_h {
                        report.add_issue(ValidationIssue::error(
                            IssueCategory::BrokenFaceLoop,
                            format!("Face {fh} cycle re-entered unexpected half-edge {curr_h}"),
                        ));
                    }
                    closed = curr_h == start_h;
                    break;
                }

                let Some(he) = mesh.half_edge(curr_h) else {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::BrokenFaceLoop,
                        format!("Face {fh} cycle references invalid half-edge {curr_h}"),
                    ));
                    break;
                };

                if he.face != Some(fh) {
                    report.add_issue(ValidationIssue::error(
                        IssueCategory::BrokenFaceLoop,
                        format!(
                            "Face {fh} cycle half-edge {curr_h} has face field {:?} != Some({fh})",
                            he.face
                        ),
                    ));
                }

                curr_h = he.next;
                count += 1;
                if curr_h == start_h {
                    closed = true;
                    break;
                }
            }

            if !closed {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenFaceLoop,
                    format!("Face {fh} cycle is not closed after {count} steps"),
                ));
            } else if count < 3 {
                report.add_issue(ValidationIssue::error(
                    IssueCategory::BrokenFaceLoop,
                    format!("Face {fh} cycle has fewer than 3 half-edges ({count})"),
                ));
            }
        }

        report
    }
}

/// Diagnostic audit report detailing mesh topological and geometric invariants.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TopologyReport {
    /// True if all structural, topological, and geometric invariants hold.
    pub is_valid: bool,
    /// True if the mesh is 2-manifold (every edge has 1 or 2 faces, vertex 1-rings form single open or closed disk).
    pub is_manifold: bool,
    /// True if the mesh is closed / watertight (contains 0 boundary edges).
    pub is_closed: bool,
    /// Euler characteristic $\chi = V - E + F$.
    pub euler_characteristic: i32,
    /// Number of connected geometric components.
    pub num_components: usize,
    /// Count of distinct boundary hole/perimeter loops.
    pub boundary_loop_count: usize,
    /// Vertices not connected to any edges.
    pub isolated_vertices: Vec<VertexHandle>,
    /// Vertices with multiple disconnected fans/umbrellas (non-manifold vertices).
    pub non_manifold_vertices: Vec<VertexHandle>,
    /// Faces with area near zero or non-finite coordinates.
    pub degenerate_faces: Vec<FaceHandle>,
    /// Structural pointer integrity failures (dangling handles, twin mismatch, cycle breakage).
    pub structural_errors: Vec<String>,
    /// Topological failures (non-manifold edges/fans, non-orientable faces).
    pub topological_errors: Vec<String>,
    /// Geometric failures (NaN/inf positions, zero-area faces).
    pub geometric_errors: Vec<String>,
    /// Non-critical warnings (e.g., isolated vertices, high-valence vertices).
    pub warnings: Vec<String>,
}

impl HalfEdgeMesh {
    /// Performs an exhaustive audit of all half-edge mesh invariants.
    pub fn validate_topology(&self) -> TopologyReport {
        let mut report = TopologyReport::default();

        let v_count = self.num_vertices();
        let e_count = self.num_edges();
        let f_count = self.num_faces();

        report.euler_characteristic = (v_count as i32) - (e_count as i32) + (f_count as i32);

        // 1. Audit Vertices
        for vh in self.vertices() {
            let Some(v) = self.vertex(vh) else {
                report
                    .structural_errors
                    .push(format!("Active vertex handle {vh} missing from array"));
                continue;
            };

            // Check finite positions
            if !v.position.is_finite() {
                report.geometric_errors.push(format!(
                    "Vertex {vh} has non-finite position: {}",
                    v.position
                ));
            }

            // Check outgoing half-edge
            if let Some(he_h) = v.half_edge {
                if !self.is_valid_half_edge(he_h) {
                    report.structural_errors.push(format!(
                        "Vertex {vh} points to invalid outgoing half-edge {he_h}"
                    ));
                } else if let Some(he) = self.half_edge(he_h)
                    && he.origin != vh
                {
                    report.structural_errors.push(format!(
                        "Vertex {vh} outgoing half-edge {he_h} origin is {} instead of {vh}",
                        he.origin
                    ));
                }
            } else {
                report.isolated_vertices.push(vh);
                report
                    .warnings
                    .push(format!("Vertex {vh} is isolated (no incident edges)"));
            }

            // Check non-manifold vertex fan (multiple disconnected umbrellas)
            let out_hes = self.outgoing_half_edges(vh);
            let mut all_outgoing_from_array = 0;
            for h in self.half_edges() {
                if let Some(he) = self.half_edge(h)
                    && he.origin == vh
                {
                    all_outgoing_from_array += 1;
                }
            }
            if out_hes.len() != all_outgoing_from_array {
                report.non_manifold_vertices.push(vh);
                report.topological_errors.push(format!(
                    "Vertex {vh} has multiple disconnected umbrella fans ({all_outgoing_from_array} outgoing edges, single circulation visited {})",
                    out_hes.len()
                ));
            }
        }

        // 2. Audit Half-Edges
        for he_h in self.half_edges() {
            let Some(he) = self.half_edge(he_h) else {
                report
                    .structural_errors
                    .push(format!("Active half-edge handle {he_h} missing"));
                continue;
            };

            // Origin validity
            if !self.is_valid_vertex(he.origin) {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} has invalid origin handle {}",
                    he.origin
                ));
            }

            // Edge validity
            if !self.is_valid_edge(he.edge) {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} has invalid edge handle {}",
                    he.edge
                ));
            }

            // Twin involution: twin.twin == self
            if !self.is_valid_half_edge(he.twin) {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} has invalid twin handle {}",
                    he.twin
                ));
            } else if let Some(twin) = self.half_edge(he.twin) {
                if twin.twin != he_h {
                    report.structural_errors.push(format!(
                        "Half-edge {he_h} twin involution violated: twin({}).twin == {} != {he_h}",
                        he.twin, twin.twin
                    ));
                }
                if twin.edge != he.edge {
                    report.structural_errors.push(format!(
                        "Half-edge {he_h} and twin {} belong to different edges ({} vs {})",
                        he.twin, he.edge, twin.edge
                    ));
                }
            }

            // Next / Prev cycles: next.prev == self and prev.next == self
            if !self.is_valid_half_edge(he.next) {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} has invalid next handle {}",
                    he.next
                ));
            } else if let Some(next) = self.half_edge(he.next)
                && next.prev != he_h
            {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} cycle broken: next({}).prev == {} != {he_h}",
                    he.next, next.prev
                ));
            }

            if !self.is_valid_half_edge(he.prev) {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} has invalid prev handle {}",
                    he.prev
                ));
            } else if let Some(prev) = self.half_edge(he.prev)
                && prev.next != he_h
            {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} cycle broken: prev({}).next == {} != {he_h}",
                    he.prev, prev.next
                ));
            }

            // Geometric consistency: next.origin == twin.origin
            if let Some(next) = self.half_edge(he.next)
                && let Some(twin) = self.half_edge(he.twin)
                && next.origin != twin.origin
            {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} next origin {} != twin origin {}",
                    next.origin, twin.origin
                ));
            }

            // Face validity
            if let Some(fh) = he.face
                && !self.is_valid_face(fh)
            {
                report.structural_errors.push(format!(
                    "Half-edge {he_h} points to invalid face handle {fh}"
                ));
            }
        }

        // 3. Audit Edges
        for eh in self.edges() {
            let Some(edge) = self.edge(eh) else {
                report
                    .structural_errors
                    .push(format!("Active edge handle {eh} missing"));
                continue;
            };

            if !self.is_valid_half_edge(edge.half_edge) {
                report.structural_errors.push(format!(
                    "Edge {eh} points to invalid half-edge {}",
                    edge.half_edge
                ));
            }
        }

        // 4. Audit Faces
        for fh in self.faces() {
            let Some(face) = self.face(fh) else {
                report
                    .structural_errors
                    .push(format!("Active face handle {fh} missing"));
                continue;
            };

            if !self.is_valid_half_edge(face.half_edge) {
                report.structural_errors.push(format!(
                    "Face {fh} points to invalid half-edge {}",
                    face.half_edge
                ));
                continue;
            }

            let f_hes = self.face_half_edges(fh);
            if f_hes.len() < 3 {
                report.topological_errors.push(format!(
                    "Face {fh} cycle has fewer than 3 half-edges ({})",
                    f_hes.len()
                ));
            }

            for &h in &f_hes {
                if let Some(he) = self.half_edge(h)
                    && he.face != Some(fh)
                {
                    report.structural_errors.push(format!(
                        "Face {fh} cycle member {h} has face field {:?} != Some({fh})",
                        he.face
                    ));
                }
            }

            let area = self.face_area(fh);
            if area <= 1e-8 {
                report.degenerate_faces.push(fh);
                report
                    .geometric_errors
                    .push(format!("Face {fh} is degenerate (area = {area})"));
            }
        }

        // 5. Boundary Loops & Watertight Status
        let bnd_loops = self.boundary_loops();
        report.boundary_loop_count = bnd_loops.len();
        report.is_closed = report.boundary_loop_count == 0;

        // 6. Connected Components
        report.num_components = self.count_connected_components();

        // 7. Overall Manifold & Validity verdict
        report.is_manifold =
            report.non_manifold_vertices.is_empty() && report.topological_errors.is_empty();

        report.is_valid = report.structural_errors.is_empty()
            && report.topological_errors.is_empty()
            && report.geometric_errors.is_empty();

        report
    }

    /// Verifies all invariants, returning `Ok(())` or a descriptive `TopologyError`.
    pub fn check_invariants(&self) -> Result<(), TopologyError> {
        let report = self.validate_topology();
        if !report.structural_errors.is_empty() {
            return Err(TopologyError::InvariantViolation(
                report.structural_errors.join("; "),
            ));
        }
        if !report.topological_errors.is_empty() {
            return Err(TopologyError::NonManifold(
                report.topological_errors.join("; "),
            ));
        }
        if !report.geometric_errors.is_empty() {
            return Err(TopologyError::Degenerate(
                report.geometric_errors.join("; "),
            ));
        }
        Ok(())
    }

    fn count_connected_components(&self) -> usize {
        let mut visited = HashSet::new();
        let mut components = 0;

        for vh in self.vertices() {
            if visited.insert(vh) {
                components += 1;
                let mut queue = VecDeque::new();
                queue.push_back(vh);

                while let Some(curr) = queue.pop_front() {
                    for neighbor in self.adjacent_vertices(curr) {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::TopologyBuilder;
    use crate::topology::handles::{EdgeHandle, FaceHandle, HalfEdgeHandle};
    use forge_math::Vec3;

    #[test]
    fn test_valid_single_triangle() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
        assert!(report.elements_checked() > 0);
        assert!(TopologyValidator::is_valid(&mesh));
    }

    #[test]
    fn test_valid_quad() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
        assert_eq!(mesh.face_count(), 1);
    }

    #[test]
    fn test_two_adjacent_triangles() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
        assert_eq!(mesh.face_count(), 2);
    }

    #[test]
    fn test_valid_mesh_with_boundaries() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let report = TopologyValidator::validate(&mesh);
        assert!(report.is_valid());
        // Boundary edges are recorded as warnings, not errors
        assert_eq!(report.error_count(), 0);
        assert!(report.warning_count() > 0);
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::Boundary)
        );
    }

    #[test]
    fn test_isolated_vertex() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(10.0, 10.0, 10.0), // isolated
        ];
        let faces = vec![vec![0, 1, 2]];
        let mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        let report = TopologyValidator::validate(&mesh);
        // An isolated vertex issues a warning but does not invalidate connectivity
        assert!(report.is_valid());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::IsolatedVertex)
        );
    }

    #[test]
    fn test_corrupted_twin() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Corrupt twin handle
        mesh.half_edges[0].twin = HalfEdgeHandle::new(999, 0);

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(report.error_count() > 0);
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::BrokenTwin)
        );
    }

    #[test]
    fn test_corrupted_next() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Corrupt next handle
        mesh.half_edges[0].next = HalfEdgeHandle::new(999, 0);

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(report.error_count() > 0);
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::BrokenNext)
        );
    }

    #[test]
    fn test_corrupted_prev() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Corrupt prev handle
        mesh.half_edges[0].prev = HalfEdgeHandle::new(999, 0);

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(report.error_count() > 0);
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::BrokenPrev)
        );
    }

    #[test]
    fn test_corrupted_face_reference() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Corrupt face handle on half-edge
        mesh.half_edges[0].face = Some(FaceHandle::new(999, 0));

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(report.error_count() > 0);
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::BrokenFaceLoop)
        );
    }

    #[test]
    fn test_unclosed_face_cycle() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Break face cycle by modifying next pointer to non-cycle half-edge
        let f0_he = mesh.faces[0].half_edge;
        mesh.half_edge_mut(f0_he).unwrap().next = mesh.half_edges[4].twin;

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::BrokenFaceLoop)
        );
    }

    #[test]
    fn test_corrupted_vertex_outgoing_half_edge() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2]];
        let mut mesh = TopologyBuilder::build(&pos, &faces).unwrap();

        // Point vertex 0 to out-of-bounds half-edge
        mesh.vertices[0].half_edge = Some(HalfEdgeHandle::new(999, 0));

        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.category == IssueCategory::InvalidHandle)
        );
    }

    #[test]
    fn test_no_panic_on_corrupted_mesh() {
        let mut mesh = HalfEdgeMesh::new();
        // Construct mesh with bogus handles
        let v0 = mesh.add_vertex(Vec3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertex_mut(v0).unwrap().half_edge = Some(HalfEdgeHandle::new(999, 999));
        mesh.vertex_mut(v1).unwrap().half_edge = Some(HalfEdgeHandle::new(888, 888));
        let dummy_e = EdgeHandle::new(777, 777);
        let h0 = mesh.add_half_edge(v0, dummy_e, None);
        mesh.half_edge_mut(h0).unwrap().twin = HalfEdgeHandle::new(666, 666);
        mesh.half_edge_mut(h0).unwrap().next = HalfEdgeHandle::new(555, 555);
        mesh.half_edge_mut(h0).unwrap().prev = HalfEdgeHandle::new(444, 444);

        // Must run safely to completion without panics
        let report = TopologyValidator::validate(&mesh);
        assert!(!report.is_valid());
        assert!(report.error_count() > 0);
    }
}
