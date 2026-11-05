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

    /// Welds co-located vertices within `tolerance` distance into shared vertices.
    pub fn weld_colocated_vertices(&mut self, tolerance: f32) {
        let factor = if tolerance > 0.0 {
            (1.0 / tolerance).max(1.0)
        } else {
            100_000.0
        };
        let mut map: HashMap<(i64, i64, i64), usize> = HashMap::new();
        let mut remap: Vec<usize> = Vec::with_capacity(self.vertices.len());
        let mut new_verts: Vec<EditVertex> = Vec::new();

        for v in &self.vertices {
            let key = (
                (v.position.x * factor).round() as i64,
                (v.position.y * factor).round() as i64,
                (v.position.z * factor).round() as i64,
            );
            match map.get(&key) {
                Some(&existing_idx) => {
                    remap.push(existing_idx);
                }
                None => {
                    let new_idx = new_verts.len();
                    new_verts.push(v.clone());
                    map.insert(key, new_idx);
                    remap.push(new_idx);
                }
            }
        }

        for face in &mut self.faces {
            for v in &mut face.vertices {
                *v = remap[*v];
            }
            face.vertices.dedup();
        }

        self.vertices = new_verts;
        self.rebuild_edges();
        self.recalculate_normals();
    }

    /// Merges adjacent coplanar triangle pairs sharing an edge into quad faces (Tris to Quads).
    pub fn tris_to_quads(&mut self) {
        let mut i = 0;
        while i < self.faces.len() {
            if self.faces[i].vertices.len() != 3 {
                i += 1;
                continue;
            }

            let mut merged = false;
            let mut j = i + 1;
            while j < self.faces.len() {
                if self.faces[j].vertices.len() != 3 {
                    j += 1;
                    continue;
                }

                // Check coplanar normals
                if self.faces[i].normal.dot(self.faces[j].normal) > 0.99 {
                    let f0 = &self.faces[i];
                    let f1 = &self.faces[j];

                    if let Some((e0, e1)) = find_shared_edge(f0, f1) {
                        let quad_opt = merge_two_triangles(f0, f1, e0, e1);
                        if let Some(quad_verts) = quad_opt {
                            self.faces[i].vertices = quad_verts;
                            self.faces.remove(j);
                            merged = true;
                            break;
                        }
                    }
                }
                j += 1;
            }

            if !merged {
                i += 1;
            }
        }

        self.rebuild_edges();
        self.recalculate_normals();
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

    /// Performs a Loop Cut along a ring of quad faces starting from `start_edge_idx`.
    ///
    /// Subdivides each quad in the continuous ring at interpolation `factor` (0.0..1.0).
    pub fn loop_cut(&mut self, start_edge_idx: usize, factor: f32) -> Result<Vec<usize>, String> {
        if start_edge_idx >= self.edges.len() {
            return Err("Invalid edge index for loop cut".to_string());
        }

        let t = factor.clamp(0.01, 0.99);
        let start_edge = self.edges[start_edge_idx];

        // Find initial quad face containing this edge
        let mut first_face_idx = None;
        for (f_idx, face) in self.faces.iter().enumerate() {
            if face.vertices.len() == 4 && face_contains_edge(face, start_edge.v0, start_edge.v1) {
                first_face_idx = Some(f_idx);
                break;
            }
        }

        let first_face = match first_face_idx {
            Some(idx) => idx,
            None => return Err("Selected edge is not part of a quad face".to_string()),
        };

        // Traverse the quad loop
        let mut loop_steps = Vec::new(); // (face_idx, v_top0, v_top1, v_bot0, v_bot1)
        let mut visited_faces = HashSet::new();

        let mut current_face_idx = first_face;
        let mut current_edge = (start_edge.v0, start_edge.v1);

        loop {
            if visited_faces.contains(&current_face_idx) {
                break;
            }
            visited_faces.insert(current_face_idx);

            let face = &self.faces[current_face_idx];
            if face.vertices.len() != 4 {
                break;
            }

            // In quad [a, b, c, d], find position of current_edge
            let (e0, e1) = current_edge;
            let edge_pos = face_edge_index(face, e0, e1).ok_or("Edge not found in quad")?;

            // Opposite edge in quad is index (edge_pos + 2) % 4
            let opp_pos = (edge_pos + 2) % 4;
            let opp_v0 = face.vertices[opp_pos];
            let opp_v1 = face.vertices[(opp_pos + 1) % 4];

            let cut_v0 = face.vertices[edge_pos];
            let cut_v1 = face.vertices[(edge_pos + 1) % 4];

            loop_steps.push((current_face_idx, cut_v0, cut_v1, opp_v1, opp_v0));

            // Find next neighboring quad sharing (opp_v0, opp_v1)
            let mut next_face_idx = None;
            for (f_idx, f) in self.faces.iter().enumerate() {
                if f_idx != current_face_idx
                    && f.vertices.len() == 4
                    && face_contains_edge(f, opp_v0, opp_v1)
                {
                    next_face_idx = Some(f_idx);
                    break;
                }
            }

            match next_face_idx {
                Some(next_idx) => {
                    current_face_idx = next_idx;
                    current_edge = (opp_v0, opp_v1);
                }
                None => break, // Reached boundary
            }
        }

        if loop_steps.is_empty() {
            return Err("No quad loop could be traversed".to_string());
        }

        // Split each quad in loop_steps
        let mut new_faces = Vec::new();
        let mut split_faces_set = HashSet::new();

        // Cache midpoints to share across adjacent quads
        let mut edge_vertex_cache: HashMap<(usize, usize), usize> = HashMap::new();

        let mut get_or_create_cut_vert =
            |v_a: usize, v_b: usize, param: f32, verts: &mut Vec<EditVertex>| -> usize {
                let key = (v_a.min(v_b), v_a.max(v_b));
                if let Some(&cached) = edge_vertex_cache.get(&key) {
                    return cached;
                }
                let pa = verts[v_a].position;
                let pb = verts[v_b].position;
                let pos = pa.lerp(pb, param);
                let norm = (verts[v_a].normal + verts[v_b].normal).normalize_or_zero();
                let uv = verts[v_a].uv.lerp(verts[v_b].uv, param);
                let idx = verts.len();
                verts.push(EditVertex::new(pos, norm, uv));
                edge_vertex_cache.insert(key, idx);
                idx
            };

        for (f_idx, v_top0, v_top1, v_bot0, v_bot1) in loop_steps {
            split_faces_set.insert(f_idx);
            let normal = self.faces[f_idx].normal;

            let m_top = get_or_create_cut_vert(v_top0, v_top1, t, &mut self.vertices);
            let m_bot = get_or_create_cut_vert(v_bot0, v_bot1, 1.0 - t, &mut self.vertices);

            // Quad 1: [v_top0, m_top, m_bot, v_bot1]
            new_faces.push(EditFace::new(vec![v_top0, m_top, m_bot, v_bot1], normal));
            // Quad 2: [m_top, v_top1, v_bot0, m_bot]
            new_faces.push(EditFace::new(vec![m_top, v_top1, v_bot0, m_bot], normal));
        }

        // Retain unmodified faces and add new faces
        let mut retained: Vec<EditFace> = self
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if split_faces_set.contains(&i) {
                    None
                } else {
                    Some(f.clone())
                }
            })
            .collect();

        let new_face_start_idx = retained.len();
        retained.extend(new_faces);
        self.faces = retained;

        self.rebuild_edges();
        self.recalculate_normals();

        let created_indices = (new_face_start_idx..self.faces.len()).collect();
        Ok(created_indices)
    }

    /// Chamfers/bevels selected edges with configurable width.
    pub fn bevel_selected_edges(&mut self, width: f32) -> Result<Vec<usize>, String> {
        let selected_edges: Vec<EditEdge> =
            self.edges.iter().filter(|e| e.selected).copied().collect();
        if selected_edges.is_empty() {
            return Err("No edges selected for bevel".to_string());
        }

        let w = width.max(1e-4);
        let mut new_faces = Vec::new();
        let mut modified_face_indices = HashSet::new();

        for edge in selected_edges {
            // Find faces sharing this edge
            let mut face_occurrences = Vec::new();
            for (f_idx, face) in self.faces.iter().enumerate() {
                if face_contains_edge(face, edge.v0, edge.v1) {
                    face_occurrences.push(f_idx);
                }
            }

            if face_occurrences.len() != 2 {
                continue; // Only 2-manifold edges can be beveled
            }

            let f0_idx = face_occurrences[0];
            let f1_idx = face_occurrences[1];
            modified_face_indices.insert(f0_idx);
            modified_face_indices.insert(f1_idx);

            // Compute offset vertices along each face normal and tangent
            let v0_pos = self.vertices[edge.v0].position;
            let v1_pos = self.vertices[edge.v1].position;
            let edge_dir = (v1_pos - v0_pos).normalize_or_zero();

            let norm0 = self.faces[f0_idx].normal;
            let norm1 = self.faces[f1_idx].normal;
            let inward0 = norm0.cross(edge_dir).normalize_or_zero();
            let inward1 = norm1.cross(-edge_dir).normalize_or_zero();

            let v0_a = self.vertices.len();
            self.vertices.push(EditVertex::new(
                v0_pos + inward0 * w,
                norm0,
                self.vertices[edge.v0].uv,
            ));

            let v1_a = self.vertices.len();
            self.vertices.push(EditVertex::new(
                v1_pos + inward0 * w,
                norm0,
                self.vertices[edge.v1].uv,
            ));

            let v0_b = self.vertices.len();
            self.vertices.push(EditVertex::new(
                v0_pos + inward1 * w,
                norm1,
                self.vertices[edge.v0].uv,
            ));

            let v1_b = self.vertices.len();
            self.vertices.push(EditVertex::new(
                v1_pos + inward1 * w,
                norm1,
                self.vertices[edge.v1].uv,
            ));

            // Create connecting bevel quad: [v0_a, v1_a, v1_b, v0_b]
            let bevel_normal = ((norm0 + norm1) * 0.5).normalize_or_zero();
            let mut bevel_quad = EditFace::new(vec![v0_a, v1_a, v1_b, v0_b], bevel_normal);
            bevel_quad.selected = true;
            new_faces.push(bevel_quad);

            // Replace edge vertices in original faces
            replace_edge_in_face(&mut self.faces[f0_idx], edge.v0, edge.v1, v0_a, v1_a);
            replace_edge_in_face(&mut self.faces[f1_idx], edge.v0, edge.v1, v0_b, v1_b);
        }

        let start_idx = self.faces.len();
        self.faces.extend(new_faces);
        self.rebuild_edges();
        self.recalculate_normals();

        Ok((start_idx..self.faces.len()).collect())
    }

    /// Dissolves selected edges, merging adjacent faces into larger polygons.
    pub fn dissolve_selected_edges(&mut self) -> Result<usize, String> {
        let selected: Vec<EditEdge> = self.edges.iter().filter(|e| e.selected).copied().collect();
        if selected.is_empty() {
            return Err("No edges selected to dissolve".to_string());
        }

        let mut dissolved_count = 0;

        for edge in selected {
            // Find faces sharing this edge
            let mut face_indices = Vec::new();
            for (f_idx, face) in self.faces.iter().enumerate() {
                if face_contains_edge(face, edge.v0, edge.v1) {
                    face_indices.push(f_idx);
                }
            }

            if face_indices.len() == 2 {
                let f0_idx = face_indices[0];
                let f1_idx = face_indices[1];

                let f0 = &self.faces[f0_idx];
                let f1 = &self.faces[f1_idx];

                // Merge vertex loops
                if let Some(merged_verts) = merge_face_loops(f0, f1, edge.v0, edge.v1) {
                    let normal = ((f0.normal + f1.normal) * 0.5).normalize_or_zero();
                    let merged_face = EditFace::new(merged_verts, normal);

                    let min_idx = f0_idx.min(f1_idx);
                    let max_idx = f0_idx.max(f1_idx);

                    self.faces.remove(max_idx);
                    self.faces.remove(min_idx);
                    self.faces.push(merged_face);
                    dissolved_count += 1;
                }
            }
        }

        if dissolved_count > 0 {
            self.rebuild_edges();
            self.recalculate_normals();
        }

        Ok(dissolved_count)
    }

    /// Bridges two selected boundary edge loops by creating a strip of quad faces.
    pub fn bridge_edge_loops(&mut self) -> Result<usize, String> {
        let boundary_edges: Vec<EditEdge> = self
            .edges
            .iter()
            .filter(|e| e.selected && is_boundary_edge(&self.faces, e.v0, e.v1))
            .copied()
            .collect();

        if boundary_edges.len() < 2 {
            return Err("Need at least two selected boundary edges to bridge".to_string());
        }

        // Partition into two disconnected components
        let mut loop_a = Vec::new();
        let mut loop_b = Vec::new();

        for e in boundary_edges {
            if loop_a.is_empty()
                || loop_a
                    .iter()
                    .any(|other: &EditEdge| other.contains(e.v0) || other.contains(e.v1))
            {
                loop_a.push(e);
            } else {
                loop_b.push(e);
            }
        }

        if loop_a.is_empty() || loop_b.is_empty() {
            return Err("Could not identify two distinct edge loops".to_string());
        }

        let mut created_faces = 0;
        let count = loop_a.len().min(loop_b.len());

        for i in 0..count {
            let ea = loop_a[i];
            let eb = loop_b[i];

            let p0 = self.vertices[ea.v0].position;
            let p1 = self.vertices[ea.v1].position;
            let p2 = self.vertices[eb.v1].position;
            let norm = (p1 - p0).cross(p2 - p0).normalize_or_zero();

            let quad = EditFace::new(vec![ea.v0, ea.v1, eb.v1, eb.v0], norm);
            self.faces.push(quad);
            created_faces += 1;
        }

        self.rebuild_edges();
        self.recalculate_normals();
        Ok(created_faces)
    }

    /// Extracts selected faces into a new independent `EditableMesh`, removing them from `self`.
    pub fn separate_selected(&mut self) -> Option<EditableMesh> {
        let selected_indices: Vec<usize> = self.selected_faces();
        if selected_indices.is_empty() {
            return None;
        }

        let selected_set: HashSet<usize> = selected_indices.iter().copied().collect();

        let mut new_verts = Vec::new();
        let mut vert_remap: HashMap<usize, usize> = HashMap::new();
        let mut new_faces = Vec::new();

        for &f_idx in &selected_indices {
            let face = &self.faces[f_idx];
            let mut face_verts = Vec::with_capacity(face.vertices.len());

            for &v_idx in &face.vertices {
                let mapped = match vert_remap.get(&v_idx) {
                    Some(&idx) => idx,
                    None => {
                        let new_idx = new_verts.len();
                        new_verts.push(self.vertices[v_idx].clone());
                        vert_remap.insert(v_idx, new_idx);
                        new_idx
                    }
                };
                face_verts.push(mapped);
            }

            new_faces.push(EditFace::new(face_verts, face.normal));
        }

        let mut separated = EditableMesh {
            vertices: new_verts,
            edges: Vec::new(),
            faces: new_faces,
        };
        separated.rebuild_edges();
        separated.recalculate_normals();

        // Remove separated faces from self
        let mut retained_faces = Vec::new();
        for (i, face) in self.faces.drain(..).enumerate() {
            if !selected_set.contains(&i) {
                retained_faces.push(face);
            }
        }
        self.faces = retained_faces;
        self.rebuild_edges();
        self.recalculate_normals();

        Some(separated)
    }

    /// Translates vertices with proportional falloff within a radius.
    pub fn proportional_translate(
        &mut self,
        delta: Vec3,
        radius: f32,
        mode: EditSelectionMode,
        falloff: ProportionalFalloff,
    ) {
        let center = match self.selection_center(mode) {
            Some(c) => c,
            None => return,
        };

        let r = radius.max(1e-4);

        for v in &mut self.vertices {
            let dist = v.position.distance(center);
            if dist < r {
                let t = dist / r;
                let weight = falloff.calculate(t);
                v.position += delta * weight;
            }
        }

        self.recalculate_normals();
    }
}

/// Falloff curve type for proportional editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProportionalFalloff {
    #[default]
    Smooth,
    Linear,
    Sphere,
    Sharp,
    Constant,
}

impl ProportionalFalloff {
    pub fn calculate(&self, t: f32) -> f32 {
        let clamped = t.clamp(0.0, 1.0);
        match self {
            ProportionalFalloff::Smooth => {
                let inv = 1.0 - clamped;
                3.0 * inv * inv - 2.0 * inv * inv * inv
            }
            ProportionalFalloff::Linear => 1.0 - clamped,
            ProportionalFalloff::Sphere => (1.0 - clamped * clamped).sqrt(),
            ProportionalFalloff::Sharp => (1.0 - clamped) * (1.0 - clamped),
            ProportionalFalloff::Constant => 1.0,
        }
    }
}

fn face_contains_edge(face: &EditFace, v0: usize, v1: usize) -> bool {
    let n = face.vertices.len();
    for i in 0..n {
        let a = face.vertices[i];
        let b = face.vertices[(i + 1) % n];
        if (a == v0 && b == v1) || (a == v1 && b == v0) {
            return true;
        }
    }
    false
}

fn face_edge_index(face: &EditFace, v0: usize, v1: usize) -> Option<usize> {
    let n = face.vertices.len();
    for i in 0..n {
        let a = face.vertices[i];
        let b = face.vertices[(i + 1) % n];
        if (a == v0 && b == v1) || (a == v1 && b == v0) {
            return Some(i);
        }
    }
    None
}

fn is_boundary_edge(faces: &[EditFace], v0: usize, v1: usize) -> bool {
    let mut count = 0;
    for f in faces {
        if face_contains_edge(f, v0, v1) {
            count += 1;
        }
    }
    count == 1
}

fn replace_edge_in_face(
    face: &mut EditFace,
    old_v0: usize,
    old_v1: usize,
    new_v0: usize,
    new_v1: usize,
) {
    let n = face.vertices.len();
    for i in 0..n {
        let a = face.vertices[i];
        let b = face.vertices[(i + 1) % n];
        if a == old_v0 && b == old_v1 {
            face.vertices[i] = new_v0;
            face.vertices[(i + 1) % n] = new_v1;
            return;
        } else if a == old_v1 && b == old_v0 {
            face.vertices[i] = new_v1;
            face.vertices[(i + 1) % n] = new_v0;
            return;
        }
    }
}

fn merge_face_loops(f0: &EditFace, f1: &EditFace, v0: usize, v1: usize) -> Option<Vec<usize>> {
    let mut merged = Vec::new();
    let n0 = f0.vertices.len();
    let n1 = f1.vertices.len();

    // Collect all vertices of f0 except edge (v0, v1)
    let idx0 = f0.vertices.iter().position(|&v| v == v0)?;
    for i in 0..n0 {
        let v = f0.vertices[(idx0 + i) % n0];
        if v != v0 && v != v1 {
            merged.push(v);
        }
    }

    // Collect vertices of f1
    let idx1 = f1.vertices.iter().position(|&v| v == v1)?;
    for i in 0..n1 {
        let v = f1.vertices[(idx1 + i) % n1];
        if v != v0 && v != v1 && !merged.contains(&v) {
            merged.push(v);
        }
    }

    if merged.len() >= 3 {
        Some(merged)
    } else {
        None
    }
}

fn find_shared_edge(f0: &EditFace, f1: &EditFace) -> Option<(usize, usize)> {
    for i in 0..f0.vertices.len() {
        let a = f0.vertices[i];
        let b = f0.vertices[(i + 1) % f0.vertices.len()];
        if face_contains_edge(f1, a, b) {
            return Some((a, b));
        }
    }
    None
}

fn merge_two_triangles(f0: &EditFace, f1: &EditFace, e0: usize, e1: usize) -> Option<Vec<usize>> {
    let apex0 = *f0.vertices.iter().find(|&&v| v != e0 && v != e1)?;
    let apex1 = *f1.vertices.iter().find(|&&v| v != e0 && v != e1)?;
    Some(vec![apex0, e1, apex1, e0])
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

    #[test]
    fn test_loop_cut_cube() {
        let cube_mesh = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        edit_mesh.weld_colocated_vertices(1e-4);
        edit_mesh.tris_to_quads();
        assert_eq!(edit_mesh.faces.len(), 6); // 6 quad faces

        let initial_faces = edit_mesh.faces.len();

        // Perform loop cut on edge 0
        let res = edit_mesh.loop_cut(0, 0.5);
        assert!(res.is_ok(), "Loop cut should succeed: {:?}", res);
        assert!(edit_mesh.faces.len() > initial_faces);
    }

    #[test]
    fn test_bevel_selected_edges() {
        let cube_mesh = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        edit_mesh.weld_colocated_vertices(1e-4);
        edit_mesh.tris_to_quads();

        let initial_faces = edit_mesh.faces.len();

        // Select first edge
        edit_mesh.edges[0].selected = true;
        let res = edit_mesh.bevel_selected_edges(0.1);
        assert!(res.is_ok(), "Bevel edges should succeed: {:?}", res);
        assert!(edit_mesh.faces.len() > initial_faces);
    }

    #[test]
    fn test_separate_selected_faces() {
        let cube_mesh = generate_cube(1.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);
        let initial_faces = edit_mesh.faces.len();

        // Select 2 faces
        edit_mesh.faces[0].selected = true;
        edit_mesh.faces[1].selected = true;

        let separated = edit_mesh
            .separate_selected()
            .expect("Separation should succeed");
        assert_eq!(separated.faces.len(), 2);
        assert_eq!(edit_mesh.faces.len(), initial_faces - 2);
    }

    #[test]
    fn test_proportional_translate() {
        let cube_mesh = generate_cube(2.0);
        let mut edit_mesh = EditableMesh::from_mesh(&cube_mesh);

        // Select vertex 0
        edit_mesh.vertices[0].selected = true;
        let p0_initial = edit_mesh.vertices[0].position;
        let p1_initial = edit_mesh.vertices[1].position;

        edit_mesh.proportional_translate(
            Vec3::new(0.0, 1.0, 0.0),
            5.0,
            EditSelectionMode::Vertex,
            ProportionalFalloff::Smooth,
        );

        // Both selected and nearby vertices should have moved along Y
        assert!(edit_mesh.vertices[0].position.y > p0_initial.y);
        assert!(edit_mesh.vertices[1].position.y > p1_initial.y);
    }
}
