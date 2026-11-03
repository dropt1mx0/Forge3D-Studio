use std::collections::HashMap;

use forge_math::{Vec2, Vec3};

use super::error::TopologyError;
use super::handles::HalfEdgeHandle;
use super::mesh::HalfEdgeMesh;

/// Builder for constructing a `HalfEdgeMesh` with manifold invariant enforcement.
pub struct TopologyBuilder;

impl TopologyBuilder {
    /// Builds a `HalfEdgeMesh` from an `EditableMesh`.
    pub fn from_editable_mesh(
        edit_mesh: &crate::edit_mesh::EditableMesh,
    ) -> Result<HalfEdgeMesh, TopologyError> {
        let positions: Vec<Vec3> = edit_mesh.vertices.iter().map(|v| v.position).collect();
        let normals: Vec<Vec3> = edit_mesh.vertices.iter().map(|v| v.normal).collect();
        let uvs: Vec<Vec2> = edit_mesh.vertices.iter().map(|v| v.uv).collect();
        let faces: Vec<Vec<usize>> = edit_mesh.faces.iter().map(|f| f.vertices.clone()).collect();

        Self::build_with_attributes(&positions, &faces, Some(&normals), Some(&uvs), None, None)
    }

    /// Builds a `HalfEdgeMesh` from positions and polygon faces (triangles, quads, or n-gons).
    pub fn build(positions: &[Vec3], faces: &[Vec<usize>]) -> Result<HalfEdgeMesh, TopologyError> {
        Self::build_with_attributes(positions, faces, None, None, None, None)
    }

    /// Builds a `HalfEdgeMesh` from positions, polygon faces, and optional vertex/corner attributes.
    pub fn build_with_attributes(
        positions: &[Vec3],
        faces: &[Vec<usize>],
        vertex_normals: Option<&[Vec3]>,
        vertex_uvs: Option<&[Vec2]>,
        corner_uvs: Option<&[Vec<Vec2>]>,
        corner_normals: Option<&[Vec<Vec3>]>,
    ) -> Result<HalfEdgeMesh, TopologyError> {
        if positions.is_empty() {
            return Err(TopologyError::EmptyMesh);
        }

        let mut mesh = HalfEdgeMesh::with_capacity(positions.len(), faces.len());

        // 1. Create all vertices
        let mut vertex_handles = Vec::with_capacity(positions.len());
        for (i, &pos) in positions.iter().enumerate() {
            let vh = mesh.add_vertex(pos);
            if let Some(normals) = vertex_normals
                && let Some(&n) = normals.get(i)
            {
                mesh.vertex_mut(vh).unwrap().normal = Some(n);
            }
            if let Some(uvs) = vertex_uvs
                && let Some(&uv) = uvs.get(i)
            {
                mesh.vertex_mut(vh).unwrap().uv = Some(uv);
            }
            vertex_handles.push(vh);
        }

        // Map directed edge (u, v) -> HalfEdgeHandle
        let mut edge_map: HashMap<(u32, u32), HalfEdgeHandle> = HashMap::new();
        // Map undirected edge {min, max} -> count of incident faces
        let mut undirected_edge_counts: HashMap<(u32, u32), usize> = HashMap::new();

        // 2. Create faces and face half-edges
        for (f_idx, face_verts) in faces.iter().enumerate() {
            let k = face_verts.len();
            if k < 3 {
                return Err(TopologyError::Degenerate(format!(
                    "Face {f_idx} has fewer than 3 vertices ({k})"
                )));
            }

            // Check vertex bounds and duplicate consecutive vertices
            for i in 0..k {
                let v0 = face_verts[i];
                let v1 = face_verts[(i + 1) % k];

                if v0 >= positions.len() || v1 >= positions.len() {
                    return Err(TopologyError::BuildError(format!(
                        "Face {f_idx} references out-of-bounds vertex index ({v0} or {v1})"
                    )));
                }

                if v0 == v1 {
                    return Err(TopologyError::Degenerate(format!(
                        "Face {f_idx} contains consecutive duplicate vertex {v0}"
                    )));
                }

                let key = (v0 as u32, v1 as u32);
                if edge_map.contains_key(&key) {
                    return Err(TopologyError::NonManifold(format!(
                        "Directed edge ({v0}, {v1}) is shared by multiple faces with the same orientation"
                    )));
                }

                let undirected = ((v0.min(v1)) as u32, (v0.max(v1)) as u32);
                let count = undirected_edge_counts.entry(undirected).or_insert(0);
                *count += 1;
                if *count > 2 {
                    return Err(TopologyError::NonManifold(format!(
                        "Undirected edge ({}, {}) is shared by more than 2 faces (non-manifold edge)",
                        undirected.0, undirected.1
                    )));
                }
            }

            // Temporary placeholder edge handle for half-edge creation
            let dummy_edge = super::handles::EdgeHandle::new(u32::MAX, 0);

            // Allocate face
            let dummy_he = HalfEdgeHandle::new(u32::MAX, 0);
            let face_h = mesh.add_face(dummy_he);

            let mut face_half_edges = Vec::with_capacity(k);
            for i in 0..k {
                let v_from = vertex_handles[face_verts[i]];
                let he_h = mesh.add_half_edge(v_from, dummy_edge, Some(face_h));

                // Assign corner attributes if available
                if let Some(c_uvs) = corner_uvs
                    && let Some(face_c_uvs) = c_uvs.get(f_idx)
                    && let Some(&uv) = face_c_uvs.get(i)
                {
                    mesh.half_edge_mut(he_h).unwrap().uv = Some(uv);
                }
                if let Some(c_norms) = corner_normals
                    && let Some(face_c_norms) = c_norms.get(f_idx)
                    && let Some(&n) = face_c_norms.get(i)
                {
                    mesh.half_edge_mut(he_h).unwrap().normal = Some(n);
                }

                face_half_edges.push(he_h);
                edge_map.insert((face_verts[i] as u32, face_verts[(i + 1) % k] as u32), he_h);
            }

            // Link next and prev around face
            for i in 0..k {
                let curr = face_half_edges[i];
                let next = face_half_edges[(i + 1) % k];
                let prev = face_half_edges[(i + k - 1) % k];

                let he = mesh.half_edge_mut(curr).unwrap();
                he.next = next;
                he.prev = prev;
            }

            // Set face representative half-edge
            mesh.face_mut(face_h).unwrap().half_edge = face_half_edges[0];

            // Compute face normal using Newell's polygon method
            let mut norm = Vec3::ZERO;
            for i in 0..k {
                let p_curr = positions[face_verts[i]];
                let p_next = positions[face_verts[(i + 1) % k]];
                norm.x += (p_curr.y - p_next.y) * (p_curr.z + p_next.z);
                norm.y += (p_curr.z - p_next.z) * (p_curr.x + p_next.x);
                norm.z += (p_curr.x - p_next.x) * (p_curr.y + p_next.y);
            }
            mesh.face_mut(face_h).unwrap().normal = Some(norm.normalize_or_zero());
        }

        // 3. Connect twins and create boundary half-edges and edges
        let mut boundary_half_edges = Vec::new();

        let all_face_directed_edges: Vec<((u32, u32), HalfEdgeHandle)> =
            edge_map.iter().map(|(&k, &v)| (k, v)).collect();

        for ((u, v), he_h) in all_face_directed_edges {
            // Check if already processed (has valid edge assigned)
            if mesh.half_edge(he_h).unwrap().edge.raw_index() != u32::MAX {
                continue;
            }

            let opp_key = (v, u);
            if let Some(&opp_he_h) = edge_map.get(&opp_key) {
                // Interior manifold edge
                let edge_h = mesh.add_edge(he_h);

                mesh.half_edge_mut(he_h).unwrap().twin = opp_he_h;
                mesh.half_edge_mut(he_h).unwrap().edge = edge_h;

                mesh.half_edge_mut(opp_he_h).unwrap().twin = he_h;
                mesh.half_edge_mut(opp_he_h).unwrap().edge = edge_h;
            } else {
                // Boundary edge: create virtual boundary twin with face = None
                let origin_v = vertex_handles[v as usize];
                let edge_h = mesh.add_edge(he_h);

                let bnd_he_h = mesh.add_half_edge(origin_v, edge_h, None);

                mesh.half_edge_mut(he_h).unwrap().twin = bnd_he_h;
                mesh.half_edge_mut(he_h).unwrap().edge = edge_h;

                mesh.half_edge_mut(bnd_he_h).unwrap().twin = he_h;
                mesh.half_edge_mut(bnd_he_h).unwrap().edge = edge_h;

                boundary_half_edges.push(bnd_he_h);
            }
        }

        // 4. Link boundary half-edges into closed directed boundary cycles
        // For each boundary half-edge bnd_he (origin v -> targets u), find the next boundary half-edge
        // starting at u by rotating around u.
        for &bnd_he_h in &boundary_half_edges {
            let twin_h = mesh.half_edge(bnd_he_h).unwrap().twin;
            // twin_h starts at u and ends at v, and belongs to a face
            let mut curr_h = twin_h;
            let mut found_next_bnd = None;

            for _ in 0..mesh.half_edges.len() {
                let prev_h = mesh.half_edge(curr_h).unwrap().prev;
                let candidate_bnd = mesh.half_edge(prev_h).unwrap().twin;
                if mesh.half_edge(candidate_bnd).unwrap().face.is_none() {
                    found_next_bnd = Some(candidate_bnd);
                    break;
                }
                curr_h = candidate_bnd;
            }

            if let Some(next_bnd_h) = found_next_bnd {
                mesh.half_edge_mut(bnd_he_h).unwrap().next = next_bnd_h;
                mesh.half_edge_mut(next_bnd_h).unwrap().prev = bnd_he_h;
            } else {
                // Fallback for non-manifold boundary topology: search any boundary half-edge starting at u
                let u_origin = mesh.half_edge(twin_h).unwrap().origin;
                for &other_bnd in &boundary_half_edges {
                    if mesh.half_edge(other_bnd).unwrap().origin == u_origin {
                        mesh.half_edge_mut(bnd_he_h).unwrap().next = other_bnd;
                        mesh.half_edge_mut(other_bnd).unwrap().prev = bnd_he_h;
                        break;
                    }
                }
            }
        }

        // 5. Assign outgoing half-edge to each vertex
        // Prefer a boundary half-edge if the vertex lies on a boundary
        for &vh in &vertex_handles {
            let mut assigned = None;
            for (idx, he) in mesh.half_edges.iter().enumerate() {
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
            mesh.vertex_mut(vh).unwrap().half_edge = assigned;
        }

        // Verify fundamental topological invariants before releasing the constructed mesh
        mesh.verify_basic_connectivity()?;

        Ok(mesh)
    }
}
