use std::collections::HashMap;

use forge_math::{Vec2, Vec3};

use super::builder::TopologyBuilder;
use super::error::TopologyError;
use super::handles::VertexHandle;
use super::mesh::HalfEdgeMesh;
use crate::edit_mesh::{EditEdge, EditFace, EditVertex, EditableMesh};
use crate::mesh::Mesh;
use crate::vertex::Vertex;

impl HalfEdgeMesh {
    /// Builds a `HalfEdgeMesh` from an `EditableMesh`.
    pub fn from_editable_mesh(edit_mesh: &EditableMesh) -> Result<Self, TopologyError> {
        TopologyBuilder::from_editable_mesh(edit_mesh)
    }

    /// Converts this `HalfEdgeMesh` into an `EditableMesh`.
    pub fn to_editable_mesh(&self) -> EditableMesh {
        let mut v_map: HashMap<VertexHandle, usize> = HashMap::new();
        let mut edit_verts = Vec::with_capacity(self.num_vertices());

        for vh in self.vertices() {
            if let Some(v) = self.vertex(vh) {
                let idx = edit_verts.len();
                v_map.insert(vh, idx);
                edit_verts.push(EditVertex::new(
                    v.position,
                    v.normal.unwrap_or(Vec3::Y),
                    v.uv.unwrap_or(Vec2::ZERO),
                ));
            }
        }

        let mut edit_edges = Vec::with_capacity(self.num_edges());
        for eh in self.edges() {
            if let Some((v0, v1)) = self.edge_vertices(eh)
                && let (Some(&i0), Some(&i1)) = (v_map.get(&v0), v_map.get(&v1))
            {
                edit_edges.push(EditEdge::new(i0, i1));
            }
        }

        let mut edit_faces = Vec::with_capacity(self.num_faces());
        for fh in self.faces() {
            let f_verts = self.face_vertices(fh);
            let mut face_indices = Vec::with_capacity(f_verts.len());
            for vh in f_verts {
                if let Some(&idx) = v_map.get(&vh) {
                    face_indices.push(idx);
                }
            }
            let norm = self.face_normal(fh);
            edit_faces.push(EditFace::new(face_indices, norm));
        }

        EditableMesh {
            vertices: edit_verts,
            edges: edit_edges,
            faces: edit_faces,
        }
    }

    /// Builds a `HalfEdgeMesh` from a GPU `Mesh`, welding topological positions while
    /// preserving corner UVs and hard normals.
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, TopologyError> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return Err(TopologyError::EmptyMesh);
        }

        // Quantized spatial grid key to weld duplicate geometric vertices (e.g. UV seams)
        let quantize = |p: [f32; 3]| -> (i32, i32, i32) {
            const SCALE: f32 = 10000.0;
            (
                (p[0] * SCALE).round() as i32,
                (p[1] * SCALE).round() as i32,
                (p[2] * SCALE).round() as i32,
            )
        };

        let mut unique_positions = Vec::new();
        let mut pos_map: HashMap<(i32, i32, i32), usize> = HashMap::new();
        // Maps original mesh vertex index -> topological vertex index
        let mut orig_to_topo = Vec::with_capacity(mesh.vertices.len());

        for v in &mesh.vertices {
            let key = quantize(v.position);
            let topo_idx = *pos_map.entry(key).or_insert_with(|| {
                let idx = unique_positions.len();
                unique_positions.push(Vec3::from_array(v.position));
                idx
            });
            orig_to_topo.push(topo_idx);
        }

        let num_triangles = mesh.indices.len() / 3;
        let mut faces = Vec::with_capacity(num_triangles);
        let mut corner_uvs = Vec::with_capacity(num_triangles);
        let mut corner_normals = Vec::with_capacity(num_triangles);

        for chunk in mesh.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
                continue;
            }

            let t0 = orig_to_topo[i0];
            let t1 = orig_to_topo[i1];
            let t2 = orig_to_topo[i2];

            // Ignore collapsed triangles
            if t0 == t1 || t1 == t2 || t2 == t0 {
                continue;
            }

            faces.push(vec![t0, t1, t2]);
            corner_uvs.push(vec![
                Vec2::from_array(mesh.vertices[i0].uv),
                Vec2::from_array(mesh.vertices[i1].uv),
                Vec2::from_array(mesh.vertices[i2].uv),
            ]);
            corner_normals.push(vec![
                Vec3::from_array(mesh.vertices[i0].normal),
                Vec3::from_array(mesh.vertices[i1].normal),
                Vec3::from_array(mesh.vertices[i2].normal),
            ]);
        }

        TopologyBuilder::build_with_attributes(
            &unique_positions,
            &faces,
            None,
            None,
            Some(&corner_uvs),
            Some(&corner_normals),
        )
    }

    /// Converts this `HalfEdgeMesh` into a triangulated GPU `Mesh`, preserving UV seams
    /// and hard normal edges.
    pub fn to_mesh(&self) -> Mesh {
        let mut out_vertices = Vec::new();
        let mut out_indices = Vec::new();

        for fh in self.faces() {
            let f_hes = self.face_half_edges(fh);
            let k = f_hes.len();
            if k < 3 {
                continue;
            }

            // Extract vertices, UVs, and normals for each corner of face
            let mut corner_data = Vec::with_capacity(k);
            let f_norm = self.face_normal(fh);

            for &he_h in &f_hes {
                let he = self.half_edge(he_h).unwrap();
                let v = self.vertex(he.origin).unwrap();

                let pos = v.position.to_array();
                let uv = he.uv.or(v.uv).unwrap_or(Vec2::ZERO).to_array();
                let norm = he.normal.or(v.normal).unwrap_or(f_norm).to_array();
                let col = v.color.unwrap_or([1.0, 1.0, 1.0, 1.0]);

                corner_data.push(Vertex {
                    position: pos,
                    normal: norm,
                    uv,
                    color: col,
                });
            }

            // Triangulate face (convex fan or ear clipping)
            let base_idx = out_vertices.len() as u32;
            for v in corner_data {
                out_vertices.push(v);
            }

            for i in 1..(k - 1) {
                out_indices.push(base_idx);
                out_indices.push(base_idx + (i as u32));
                out_indices.push(base_idx + ((i + 1) as u32));
            }
        }

        Mesh::new(out_vertices, out_indices)
    }
}
