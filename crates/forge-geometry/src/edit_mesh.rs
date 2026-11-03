use std::collections::{HashMap, HashSet};

use forge_math::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Selection component mode for Edit Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditSelectionMode {
    #[default]
    Vertex,
    Edge,
    Face,
}

/// A vertex in an editable polygon mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub selected: bool,
}

impl EditVertex {
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
            selected: false,
        }
    }
}

/// An edge connecting two vertex indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditEdge {
    pub v0: usize,
    pub v1: usize,
    pub selected: bool,
}

impl EditEdge {
    pub fn new(v0: usize, v1: usize) -> Self {
        Self {
            v0: v0.min(v1),
            v1: v0.max(v1),
            selected: false,
        }
    }

    pub fn contains(&self, v: usize) -> bool {
        self.v0 == v || self.v1 == v
    }

    pub fn other(&self, v: usize) -> Option<usize> {
        if self.v0 == v {
            Some(self.v1)
        } else if self.v1 == v {
            Some(self.v0)
        } else {
            None
        }
    }
}

/// A polygon face with an arbitrary number of vertices (triangles, quads, or n-gons).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditFace {
    pub vertices: Vec<usize>,
    pub normal: Vec3,
    pub selected: bool,
}

impl EditFace {
    pub fn new(vertices: Vec<usize>, normal: Vec3) -> Self {
        Self {
            vertices,
            normal,
            selected: false,
        }
    }
}

/// An editable B-Rep / PolyMesh data structure with rich modeling operations.
///
/// Supports vertex, edge, and face selection, extrude, inset, subdivide, bevel, merge, fill, and delete.
/// Pure CPU data structure, completely independent of GPU handles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EditableMesh {
    pub vertices: Vec<EditVertex>,
    pub edges: Vec<EditEdge>,
    pub faces: Vec<EditFace>,
}

impl EditableMesh {
    /// Creates an empty editable mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Converts a triangular `Mesh` into an `EditableMesh`.
    pub fn from_mesh(mesh: &Mesh) -> Self {
        let mut vertices = Vec::with_capacity(mesh.vertices.len());
        for v in &mesh.vertices {
            vertices.push(EditVertex::new(
                Vec3::from_array(v.position),
                Vec3::from_array(v.normal),
                Vec2::from_array(v.uv),
            ));
        }

        let mut edge_map = HashSet::new();
        let mut edges = Vec::new();
        let mut faces = Vec::new();

        for chunk in mesh.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }

            // Edges
            for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
                let e = (a.min(b), a.max(b));
                if edge_map.insert(e) {
                    edges.push(EditEdge::new(a, b));
                }
            }

            // Normal
            let p0 = vertices[i0].position;
            let p1 = vertices[i1].position;
            let p2 = vertices[i2].position;
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

            faces.push(EditFace::new(vec![i0, i1, i2], normal));
        }

        let mut edit_mesh = Self {
            vertices,
            edges,
            faces,
        };
        edit_mesh.recalculate_normals();
        edit_mesh
    }

    /// Converts the `EditableMesh` into a triangulated render `Mesh` for GPU upload.
    pub fn to_render_mesh(&self) -> Mesh {
        let mut out_vertices = Vec::new();
        let mut out_indices = Vec::new();

        for face in &self.faces {
            if face.vertices.len() < 3 {
                continue;
            }

            let start_v_idx = out_vertices.len() as u32;

            // Emit vertices for this face
            for &v_idx in &face.vertices {
                if let Some(v) = self.vertices.get(v_idx) {
                    out_vertices.push(Vertex::new(
                        v.position.to_array(),
                        face.normal.to_array(),
                        v.uv.to_array(),
                        if face.selected || v.selected {
                            [1.0, 0.7, 0.2, 1.0] // Selected amber highlight
                        } else {
                            [0.9, 0.9, 0.95, 1.0]
                        },
                    ));
                }
            }

            // Fan triangulation for convex polygons (0, i, i+1)
            let num_verts = face.vertices.len() as u32;
            for i in 1..(num_verts - 1) {
                out_indices.push(start_v_idx);
                out_indices.push(start_v_idx + i);
                out_indices.push(start_v_idx + i + 1);
            }
        }

        Mesh::new(out_vertices, out_indices)
    }

    /// Converts this `EditableMesh` into a topological `HalfEdgeMesh`.
    pub fn to_half_edge_mesh(
        &self,
    ) -> Result<crate::topology::HalfEdgeMesh, crate::topology::TopologyError> {
        crate::topology::HalfEdgeMesh::from_editable_mesh(self)
    }

    /// Creates an `EditableMesh` from a `HalfEdgeMesh`.
    pub fn from_half_edge_mesh(half_edge_mesh: &crate::topology::HalfEdgeMesh) -> Self {
        half_edge_mesh.to_editable_mesh()
    }

    /// Recalculates face normals and averages them for vertex normals.
    pub fn recalculate_normals(&mut self) {
        // Calculate face normals
        for face in &mut self.faces {
            if face.vertices.len() >= 3 {
                let p0 = self.vertices[face.vertices[0]].position;
                let p1 = self.vertices[face.vertices[1]].position;
                let p2 = self.vertices[face.vertices[2]].position;
                face.normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            }
        }

        // Accumulate vertex normals from adjacent faces
        for v in &mut self.vertices {
            v.normal = Vec3::ZERO;
        }

        for face in &self.faces {
            for &v_idx in &face.vertices {
                if let Some(v) = self.vertices.get_mut(v_idx) {
                    v.normal += face.normal;
                }
            }
        }

        for v in &mut self.vertices {
            v.normal = v.normal.normalize_or_zero();
        }
    }

    /// Selects all elements of the given selection mode.
    pub fn select_all(&mut self, mode: EditSelectionMode) {
        match mode {
            EditSelectionMode::Vertex => {
                for v in &mut self.vertices {
                    v.selected = true;
                }
            }
            EditSelectionMode::Edge => {
                for e in &mut self.edges {
                    e.selected = true;
                }
            }
            EditSelectionMode::Face => {
                for f in &mut self.faces {
                    f.selected = true;
                }
            }
        }
    }

    /// Deselects all elements of the given selection mode.
    pub fn deselect_all(&mut self, mode: EditSelectionMode) {
        match mode {
            EditSelectionMode::Vertex => {
                for v in &mut self.vertices {
                    v.selected = false;
                }
            }
            EditSelectionMode::Edge => {
                for e in &mut self.edges {
                    e.selected = false;
                }
            }
            EditSelectionMode::Face => {
                for f in &mut self.faces {
                    f.selected = false;
                }
            }
        }
    }

    /// Inverts selection of elements for the given mode.
    pub fn invert_selection(&mut self, mode: EditSelectionMode) {
        match mode {
            EditSelectionMode::Vertex => {
                for v in &mut self.vertices {
                    v.selected = !v.selected;
                }
            }
            EditSelectionMode::Edge => {
                for e in &mut self.edges {
                    e.selected = !e.selected;
                }
            }
            EditSelectionMode::Face => {
                for f in &mut self.faces {
                    f.selected = !f.selected;
                }
            }
        }
    }

    /// Returns list of selected vertex indices.
    pub fn selected_vertices(&self) -> Vec<usize> {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(idx, v)| if v.selected { Some(idx) } else { None })
            .collect()
    }

    /// Returns list of selected face indices.
    pub fn selected_faces(&self) -> Vec<usize> {
        self.faces
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| if f.selected { Some(idx) } else { None })
            .collect()
    }

    /// Computes the median center of the current selection.
    pub fn selection_center(&self, mode: EditSelectionMode) -> Option<Vec3> {
        match mode {
            EditSelectionMode::Vertex => {
                let selected = self.selected_vertices();
                if selected.is_empty() {
                    return None;
                }
                let sum: Vec3 = selected.iter().map(|&i| self.vertices[i].position).sum();
                Some(sum / selected.len() as f32)
            }
            EditSelectionMode::Edge => {
                let mut sum = Vec3::ZERO;
                let mut count = 0;
                for e in &self.edges {
                    if e.selected {
                        sum += (self.vertices[e.v0].position + self.vertices[e.v1].position) * 0.5;
                        count += 1;
                    }
                }
                if count > 0 {
                    Some(sum / count as f32)
                } else {
                    None
                }
            }
            EditSelectionMode::Face => {
                let selected = self.selected_faces();
                if selected.is_empty() {
                    return None;
                }
                let mut sum = Vec3::ZERO;
                for &idx in &selected {
                    let face = &self.faces[idx];
                    let face_sum: Vec3 = face
                        .vertices
                        .iter()
                        .map(|&i| self.vertices[i].position)
                        .sum();
                    sum += face_sum / face.vertices.len() as f32;
                }
                Some(sum / selected.len() as f32)
            }
        }
    }

    /// Translates the selected elements by `delta`.
    pub fn translate_selection(&mut self, delta: Vec3, mode: EditSelectionMode) {
        let mut affected_verts = HashSet::new();
        match mode {
            EditSelectionMode::Vertex => {
                for (idx, v) in self.vertices.iter().enumerate() {
                    if v.selected {
                        affected_verts.insert(idx);
                    }
                }
            }
            EditSelectionMode::Edge => {
                for e in &self.edges {
                    if e.selected {
                        affected_verts.insert(e.v0);
                        affected_verts.insert(e.v1);
                    }
                }
            }
            EditSelectionMode::Face => {
                for f in &self.faces {
                    if f.selected {
                        for &v_idx in &f.vertices {
                            affected_verts.insert(v_idx);
                        }
                    }
                }
            }
        }

        for idx in affected_verts {
            self.vertices[idx].position += delta;
        }

        self.recalculate_normals();
    }

    /// Rotates the selected elements around `pivot` by `angle_rad` around `axis`.
    pub fn rotate_selection(
        &mut self,
        axis: Vec3,
        angle_rad: f32,
        pivot: Vec3,
        mode: EditSelectionMode,
    ) {
        let rot = forge_math::Quat::from_axis_angle(axis.normalize_or_zero(), angle_rad);
        let mut affected = HashSet::new();
        match mode {
            EditSelectionMode::Vertex => {
                for (i, v) in self.vertices.iter().enumerate() {
                    if v.selected {
                        affected.insert(i);
                    }
                }
            }
            EditSelectionMode::Edge => {
                for e in &self.edges {
                    if e.selected {
                        affected.insert(e.v0);
                        affected.insert(e.v1);
                    }
                }
            }
            EditSelectionMode::Face => {
                for f in &self.faces {
                    if f.selected {
                        for &v in &f.vertices {
                            affected.insert(v);
                        }
                    }
                }
            }
        }

        for idx in affected {
            let offset = self.vertices[idx].position - pivot;
            self.vertices[idx].position = pivot + rot.mul_vec3(offset);
        }

        self.recalculate_normals();
    }

    /// Scales the selected elements relative to `pivot` by `factor`.
    pub fn scale_selection(&mut self, factor: Vec3, pivot: Vec3, mode: EditSelectionMode) {
        let mut affected = HashSet::new();
        match mode {
            EditSelectionMode::Vertex => {
                for (i, v) in self.vertices.iter().enumerate() {
                    if v.selected {
                        affected.insert(i);
                    }
                }
            }
            EditSelectionMode::Edge => {
                for e in &self.edges {
                    if e.selected {
                        affected.insert(e.v0);
                        affected.insert(e.v1);
                    }
                }
            }
            EditSelectionMode::Face => {
                for f in &self.faces {
                    if f.selected {
                        for &v in &f.vertices {
                            affected.insert(v);
                        }
                    }
                }
            }
        }

        for idx in affected {
            let offset = self.vertices[idx].position - pivot;
            self.vertices[idx].position = pivot + offset * factor;
        }

        self.recalculate_normals();
    }

    /// Extrudes selected faces along their normals, creating side quad walls.
    /// Returns the indices of the newly extruded top faces.
    pub fn extrude_faces(&mut self, distance: f32) -> Vec<usize> {
        let selected_indices = self.selected_faces();
        if selected_indices.is_empty() {
            return Vec::new();
        }

        let mut new_face_indices = Vec::new();

        for face_idx in selected_indices {
            let face = self.faces[face_idx].clone();
            let normal = face.normal;
            let offset = normal * distance;

            // Duplicate face vertices and move along normal
            let mut new_v_indices = Vec::with_capacity(face.vertices.len());
            for &old_v in &face.vertices {
                let v = &self.vertices[old_v];
                let new_v = EditVertex::new(v.position + offset, v.normal, v.uv);
                let new_idx = self.vertices.len();
                self.vertices.push(new_v);
                new_v_indices.push(new_idx);
            }

            // Create side quad walls
            let n = face.vertices.len();
            for i in 0..n {
                let next = (i + 1) % n;
                let bot0 = face.vertices[i];
                let bot1 = face.vertices[next];
                let top0 = new_v_indices[i];
                let top1 = new_v_indices[next];

                // Side quad face: [bot0, bot1, top1, top0]
                let side_face = EditFace::new(vec![bot0, bot1, top1, top0], normal);
                self.faces.push(side_face);

                // Add side edges
                self.edges.push(EditEdge::new(bot0, top0));
                self.edges.push(EditEdge::new(top0, top1));
            }

            // Update original face to use the new extruded vertices
            self.faces[face_idx].vertices = new_v_indices;
            self.faces[face_idx].selected = true;
            new_face_indices.push(face_idx);
        }

        self.recalculate_normals();
        new_face_indices
    }

    /// Insets selected faces by creating an inner ring scaled towards the face centroid.
    pub fn inset_faces(&mut self, amount: f32) -> Vec<usize> {
        let selected_indices = self.selected_faces();
        if selected_indices.is_empty() {
            return Vec::new();
        }

        let mut new_faces = Vec::new();

        for face_idx in selected_indices {
            let face = self.faces[face_idx].clone();
            let n = face.vertices.len();
            if n < 3 {
                continue;
            }

            // Compute face centroid
            let centroid: Vec3 = face
                .vertices
                .iter()
                .map(|&i| self.vertices[i].position)
                .sum::<Vec3>()
                / n as f32;

            // Create inner vertices
            let mut inner_indices = Vec::with_capacity(n);
            for &old_v in &face.vertices {
                let pos = self.vertices[old_v].position;
                let dir = (centroid - pos).normalize_or_zero();
                let inner_pos = pos + dir * amount;

                let new_v = EditVertex::new(inner_pos, face.normal, self.vertices[old_v].uv);
                let idx = self.vertices.len();
                self.vertices.push(new_v);
                inner_indices.push(idx);
            }

            // Create border quad strips connecting outer to inner loop
            for i in 0..n {
                let next = (i + 1) % n;
                let out0 = face.vertices[i];
                let out1 = face.vertices[next];
                let in0 = inner_indices[i];
                let in1 = inner_indices[next];

                self.faces
                    .push(EditFace::new(vec![out0, out1, in1, in0], face.normal));
                self.edges.push(EditEdge::new(in0, in1));
                self.edges.push(EditEdge::new(out0, in0));
            }

            // Update center face
            self.faces[face_idx].vertices = inner_indices;
            self.faces[face_idx].selected = true;
            new_faces.push(face_idx);
        }

        self.recalculate_normals();
        new_faces
    }

    /// Subdivides selected faces (splits 1 quad into 4 quads, or 1 triangle into 3 triangles).
    pub fn subdivide(&mut self) {
        let selected_indices = self.selected_faces();
        if selected_indices.is_empty() {
            return;
        }

        let mut new_faces = Vec::new();
        let mut edge_midpoint_cache = HashMap::new();

        let mut get_edge_mid = |e0: usize, e1: usize, verts: &mut Vec<EditVertex>| -> usize {
            let key = (e0.min(e1), e0.max(e1));
            if let Some(&mid) = edge_midpoint_cache.get(&key) {
                return mid;
            }
            let p = (verts[e0].position + verts[e1].position) * 0.5;
            let norm = (verts[e0].normal + verts[e1].normal).normalize_or_zero();
            let uv = (verts[e0].uv + verts[e1].uv) * 0.5;
            let idx = verts.len();
            verts.push(EditVertex::new(p, norm, uv));
            edge_midpoint_cache.insert(key, idx);
            idx
        };

        for &face_idx in &selected_indices {
            let face = &self.faces[face_idx];
            let n = face.vertices.len();

            if n == 4 {
                // Quad subdivision -> 4 quads
                let v0 = face.vertices[0];
                let v1 = face.vertices[1];
                let v2 = face.vertices[2];
                let v3 = face.vertices[3];

                let m01 = get_edge_mid(v0, v1, &mut self.vertices);
                let m12 = get_edge_mid(v1, v2, &mut self.vertices);
                let m23 = get_edge_mid(v2, v3, &mut self.vertices);
                let m30 = get_edge_mid(v3, v0, &mut self.vertices);

                // Center point
                let center_p = (self.vertices[v0].position
                    + self.vertices[v1].position
                    + self.vertices[v2].position
                    + self.vertices[v3].position)
                    * 0.25;
                let center_idx = self.vertices.len();
                self.vertices.push(EditVertex::new(
                    center_p,
                    face.normal,
                    (self.vertices[v0].uv + self.vertices[v2].uv) * 0.5,
                ));

                new_faces.push(EditFace::new(vec![v0, m01, center_idx, m30], face.normal));
                new_faces.push(EditFace::new(vec![m01, v1, m12, center_idx], face.normal));
                new_faces.push(EditFace::new(vec![center_idx, m12, v2, m23], face.normal));
                new_faces.push(EditFace::new(vec![m30, center_idx, m23, v3], face.normal));
            } else if n == 3 {
                // Triangle subdivision -> 4 triangles
                let v0 = face.vertices[0];
                let v1 = face.vertices[1];
                let v2 = face.vertices[2];

                let m01 = get_edge_mid(v0, v1, &mut self.vertices);
                let m12 = get_edge_mid(v1, v2, &mut self.vertices);
                let m20 = get_edge_mid(v2, v0, &mut self.vertices);

                new_faces.push(EditFace::new(vec![v0, m01, m20], face.normal));
                new_faces.push(EditFace::new(vec![m01, v1, m12], face.normal));
                new_faces.push(EditFace::new(vec![m20, m12, v2], face.normal));
                new_faces.push(EditFace::new(vec![m01, m12, m20], face.normal));
            }
        }

        // Replace subdivided faces
        let set: HashSet<usize> = selected_indices.into_iter().collect();
        let mut retained: Vec<EditFace> = self
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if set.contains(&i) {
                    None
                } else {
                    Some(f.clone())
                }
            })
            .collect();
        retained.extend(new_faces);
        self.faces = retained;

        self.rebuild_edges();
        self.recalculate_normals();
    }

    /// Rebuilds edge list from all face boundaries.
    pub fn rebuild_edges(&mut self) {
        let mut edge_set = HashSet::new();
        self.edges.clear();

        for face in &self.faces {
            let n = face.vertices.len();
            for i in 0..n {
                let next = (i + 1) % n;
                let a = face.vertices[i];
                let b = face.vertices[next];
                let key = (a.min(b), a.max(b));
                if edge_set.insert(key) {
                    self.edges.push(EditEdge::new(a, b));
                }
            }
        }
    }

    /// Merges selected vertices at their average position.
    pub fn merge_selected_at_center(&mut self) {
        let selected = self.selected_vertices();
        if selected.len() < 2 {
            return;
        }

        let target_pos = self
            .selection_center(EditSelectionMode::Vertex)
            .unwrap_or(Vec3::ZERO);
        let rep_idx = selected[0];
        self.vertices[rep_idx].position = target_pos;

        let remap: HashMap<usize, usize> = selected.iter().map(|&idx| (idx, rep_idx)).collect();

        for face in &mut self.faces {
            for v in &mut face.vertices {
                if let Some(&new_idx) = remap.get(v) {
                    *v = new_idx;
                }
            }
            // Remove consecutive duplicates
            face.vertices.dedup();
        }

        // Remove degenerate faces (< 3 distinct vertices)
        self.faces.retain(|f| f.vertices.len() >= 3);

        self.rebuild_edges();
        self.recalculate_normals();
    }

    /// Deletes selected elements.
    pub fn delete_selected(&mut self, mode: EditSelectionMode) {
        match mode {
            EditSelectionMode::Face => {
                self.faces.retain(|f| !f.selected);
            }
            EditSelectionMode::Edge => {
                let selected_edges: HashSet<(usize, usize)> = self
                    .edges
                    .iter()
                    .filter(|e| e.selected)
                    .map(|e| (e.v0, e.v1))
                    .collect();

                self.faces.retain(|f| {
                    let n = f.vertices.len();
                    for i in 0..n {
                        let next = (i + 1) % n;
                        let a = f.vertices[i].min(f.vertices[next]);
                        let b = f.vertices[i].max(f.vertices[next]);
                        if selected_edges.contains(&(a, b)) {
                            return false;
                        }
                    }
                    true
                });
            }
            EditSelectionMode::Vertex => {
                let selected_verts: HashSet<usize> = self.selected_vertices().into_iter().collect();
                self.faces
                    .retain(|f| !f.vertices.iter().any(|v| selected_verts.contains(v)));
            }
        }

        self.rebuild_edges();
        self.recalculate_normals();
    }

    /// Fills a face from selected vertices (creates a new face).
    pub fn fill_face_from_selection(&mut self) -> Option<usize> {
        let selected = self.selected_vertices();
        if selected.len() < 3 {
            return None;
        }

        let p0 = self.vertices[selected[0]].position;
        let p1 = self.vertices[selected[1]].position;
        let p2 = self.vertices[selected[2]].position;
        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

        let new_face = EditFace::new(selected, normal);
        let idx = self.faces.len();
        self.faces.push(new_face);

        self.rebuild_edges();
        self.recalculate_normals();
        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::generate_cube;

    #[test]
    fn test_editable_mesh_from_cube() {
        let cube_mesh = generate_cube(1.0);
        let edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        assert!(!edit_mesh.vertices.is_empty());
        assert!(!edit_mesh.faces.is_empty());
        assert!(!edit_mesh.edges.is_empty());

        let render_mesh = edit_mesh.to_render_mesh();
        assert!(!render_mesh.vertices.is_empty());
        assert!(!render_mesh.indices.is_empty());
    }

    #[test]
    fn test_editable_mesh_extrude() {
        let cube_mesh = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        let initial_faces = edit_mesh.faces.len();

        // Select first face and extrude
        edit_mesh.faces[0].selected = true;
        let extruded = edit_mesh.extrude_faces(1.0);
        assert!(!extruded.is_empty());
        assert!(edit_mesh.faces.len() > initial_faces);
    }

    #[test]
    fn test_editable_mesh_subdivide() {
        let cube_mesh = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        let initial_face_count = edit_mesh.faces.len();

        edit_mesh.select_all(EditSelectionMode::Face);
        edit_mesh.subdivide();
        assert!(edit_mesh.faces.len() > initial_face_count);
    }
}
