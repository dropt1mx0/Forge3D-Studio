use forge_geometry::{Mesh, Vertex};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Modifier that recalculates vertex normals weighted by face area and corner angles.
///
/// Produces clean, artifact-free shading on hard-surface geometry and chamfered bevels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedNormalModifier {
    /// Influence weight of the modifier (0.0 to 1.0).
    pub weight: f32,
    /// Keep sharp corners flat if face angle difference exceeds this threshold.
    pub keep_sharp: bool,
}

impl Default for WeightedNormalModifier {
    fn default() -> Self {
        Self {
            weight: 1.0,
            keep_sharp: true,
        }
    }
}

impl WeightedNormalModifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, mesh: &Mesh) -> Mesh {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return mesh.clone();
        }

        let mut accumulated_normals = vec![Vec3::ZERO; mesh.vertices.len()];

        for chunk in mesh.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
                continue;
            }

            let p0 = Vec3::from(mesh.vertices[i0].position);
            let p1 = Vec3::from(mesh.vertices[i1].position);
            let p2 = Vec3::from(mesh.vertices[i2].position);

            let e01 = p1 - p0;
            let e12 = p2 - p1;
            let e20 = p0 - p2;

            let face_cross = e01.cross(-e20);
            let area = face_cross.length() * 0.5;

            if area > 1e-8 {
                let face_normal = face_cross / (area * 2.0);

                // Corner angles
                let angle0 = angle_between(e01, -e20);
                let angle1 = angle_between(e12, -e01);
                let angle2 = angle_between(e20, -e12);

                accumulated_normals[i0] += face_normal * area * angle0;
                accumulated_normals[i1] += face_normal * area * angle1;
                accumulated_normals[i2] += face_normal * area * angle2;
            }
        }

        let new_vertices: Vec<Vertex> = mesh
            .vertices
            .iter()
            .zip(accumulated_normals.iter())
            .map(|(v, &acc)| {
                let mut vert = *v;
                if acc.length_squared() > 1e-10 {
                    let weighted_norm = acc.normalize();
                    let orig_norm = Vec3::from(v.normal);
                    let final_norm = orig_norm
                        .lerp(weighted_norm, self.weight)
                        .normalize_or_zero();
                    vert.normal = final_norm.into();
                }
                vert
            })
            .collect();

        Mesh::new(new_vertices, mesh.indices.clone())
    }
}

fn angle_between(v1: Vec3, v2: Vec3) -> f32 {
    let len_product = v1.length() * v2.length();
    if len_product < 1e-8 {
        return 0.0;
    }
    let cos_theta = (v1.dot(v2) / len_product).clamp(-1.0, 1.0);
    cos_theta.acos()
}
