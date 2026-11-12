use forge_math::Vec3;
use serde::{Deserialize, Serialize};

use crate::edit_mesh::EditableMesh;

/// Supported organic sculpting brush types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SculptBrush {
    #[default]
    Draw,
    Clay,
    Smooth,
    Flatten,
    Inflate,
    Deflate,
    Grab,
    Pinch,
    Crease,
    SnakeHook,
    ElasticDeform,
    Thumb,
}

impl SculptBrush {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Draw => "Draw",
            Self::Clay => "Clay",
            Self::Smooth => "Smooth",
            Self::Flatten => "Flatten",
            Self::Inflate => "Inflate",
            Self::Deflate => "Deflate",
            Self::Grab => "Grab",
            Self::Pinch => "Pinch",
            Self::Crease => "Crease",
            Self::SnakeHook => "Snake Hook",
            Self::ElasticDeform => "Elastic Deform",
            Self::Thumb => "Thumb",
        }
    }
}

/// Parameters configuring the active sculpt brush stroke.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SculptSettings {
    pub brush: SculptBrush,
    pub radius: f32,
    pub strength: f32,
    pub symmetry_x: bool,
    pub symmetry_y: bool,
    pub symmetry_z: bool,
    pub voxel_size: f32,
}

impl Default for SculptSettings {
    fn default() -> Self {
        Self {
            brush: SculptBrush::Draw,
            radius: 0.5,
            strength: 0.35,
            symmetry_x: false,
            symmetry_y: false,
            symmetry_z: false,
            voxel_size: 0.1,
        }
    }
}

/// Evaluates smooth spherical cubic falloff: (1 - (d/r)^2)^3 for d < r, else 0.
pub fn sculpt_falloff(distance: f32, radius: f32) -> f32 {
    let r = radius.max(1e-4);
    if distance >= r {
        0.0
    } else {
        let t = distance / r;
        let factor = 1.0 - t * t;
        factor * factor * factor
    }
}

/// Applies a sculpt brush deformation stroke to an editable mesh at `hit_point`.
/// Returns the number of affected vertices.
pub fn apply_sculpt_stroke(
    mesh: &mut EditableMesh,
    hit_point: Vec3,
    hit_normal: Vec3,
    stroke_delta: Vec3,
    settings: &SculptSettings,
) -> usize {
    let mut centers = vec![(hit_point, hit_normal, stroke_delta)];

    if settings.symmetry_x {
        centers.push((
            Vec3::new(-hit_point.x, hit_point.y, hit_point.z),
            Vec3::new(-hit_normal.x, hit_normal.y, hit_normal.z),
            Vec3::new(-stroke_delta.x, stroke_delta.y, stroke_delta.z),
        ));
    }
    if settings.symmetry_y {
        centers.push((
            Vec3::new(hit_point.x, -hit_point.y, hit_point.z),
            Vec3::new(hit_normal.x, -hit_normal.y, hit_normal.z),
            Vec3::new(stroke_delta.x, -stroke_delta.y, stroke_delta.z),
        ));
    }
    if settings.symmetry_z {
        centers.push((
            Vec3::new(hit_point.x, hit_point.y, -hit_point.z),
            Vec3::new(hit_normal.x, hit_normal.y, -hit_normal.z),
            Vec3::new(stroke_delta.x, stroke_delta.y, -stroke_delta.z),
        ));
    }

    let mut affected_count = 0;
    let radius = settings.radius.max(1e-4);
    let strength = settings.strength;

    for (center, normal, delta) in centers {
        let norm = normal.normalize_or_zero();

        // Compute centroid for smoothing if needed
        let mut avg_pos = Vec3::ZERO;
        let mut avg_count = 0;
        if settings.brush == SculptBrush::Smooth {
            for v in &mesh.vertices {
                let dist = v.position.distance(center);
                if dist < radius {
                    avg_pos += v.position;
                    avg_count += 1;
                }
            }
            if avg_count > 0 {
                avg_pos /= avg_count as f32;
            }
        }

        for v in &mut mesh.vertices {
            let dist = v.position.distance(center);
            if dist < radius {
                let w = sculpt_falloff(dist, radius);
                if w > 1e-5 {
                    match settings.brush {
                        SculptBrush::Draw => {
                            v.position += norm * (strength * w * radius * 0.5);
                        }
                        SculptBrush::Clay => {
                            let target_cap = center + norm * (strength * radius * 0.5);
                            let to_cap = target_cap - v.position;
                            v.position += to_cap * (strength * w).clamp(0.0, 1.0);
                        }
                        SculptBrush::Inflate => {
                            let v_norm = v.normal.normalize_or_zero();
                            v.position += v_norm * (strength * w * radius * 0.5);
                        }
                        SculptBrush::Deflate => {
                            let v_norm = v.normal.normalize_or_zero();
                            v.position -= v_norm * (strength * w * radius * 0.5);
                        }
                        SculptBrush::Smooth => {
                            if avg_count > 0 {
                                let diff = avg_pos - v.position;
                                v.position += diff * (strength * w);
                            }
                        }
                        SculptBrush::Flatten => {
                            let plane_dist = (v.position - center).dot(norm);
                            v.position -= norm * (plane_dist * strength * w);
                        }
                        SculptBrush::Grab => {
                            v.position += delta * (strength * w);
                        }
                        SculptBrush::Pinch => {
                            let to_center = center - v.position;
                            v.position += to_center * (strength * w * 0.5);
                        }
                        SculptBrush::Crease => {
                            let to_center = center - v.position;
                            v.position += to_center * (strength * w * 0.35);
                            v.position -= norm * (strength * w * radius * 0.35);
                        }
                        SculptBrush::SnakeHook => {
                            let hook_factor = (w * w * 1.5).min(2.5);
                            v.position += delta * (strength * hook_factor);
                        }
                        SculptBrush::ElasticDeform => {
                            let t = (dist / radius).clamp(0.0, 1.0);
                            let elastic_w = (1.0 - t * t).clamp(0.0, 1.0);
                            v.position += delta * (strength * elastic_w);
                        }
                        SculptBrush::Thumb => {
                            v.position += delta * (strength * w * 0.75);
                        }
                    }
                    affected_count += 1;
                }
            }
        }
    }

    if affected_count > 0 {
        mesh.recalculate_normals();
    }

    affected_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::generate_plane;

    #[test]
    fn test_sculpt_draw_brush() {
        let plane_mesh = generate_plane(2.0, 8);
        let mut edit_mesh = EditableMesh::from_mesh(&plane_mesh);

        let hit_pt = Vec3::new(0.0, 0.0, 0.0);
        let hit_norm = Vec3::Y;
        let settings = SculptSettings {
            brush: SculptBrush::Draw,
            radius: 0.8,
            strength: 0.5,
            ..Default::default()
        };

        let affected = apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, Vec3::ZERO, &settings);
        assert!(
            affected > 0,
            "Sculpt draw should affect vertices within radius"
        );

        // The center vertex at (0, 0, 0) should have been displaced along +Y
        let center_v = edit_mesh
            .vertices
            .iter()
            .find(|v| v.position.x.abs() < 1e-4 && v.position.z.abs() < 1e-4)
            .expect("Center vertex should exist");
        assert!(
            center_v.position.y > 0.0,
            "Center vertex should move upwards"
        );
    }

    #[test]
    fn test_sculpt_inflate_brush() {
        let plane_mesh = generate_plane(2.0, 8);
        let mut edit_mesh = EditableMesh::from_mesh(&plane_mesh);

        let hit_pt = Vec3::ZERO;
        let hit_norm = Vec3::Y;
        let settings = SculptSettings {
            brush: SculptBrush::Inflate,
            radius: 0.8,
            strength: 0.5,
            ..Default::default()
        };

        let affected = apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, Vec3::ZERO, &settings);
        assert!(affected > 0);

        let center_v = edit_mesh
            .vertices
            .iter()
            .find(|v| v.position.x.abs() < 1e-4 && v.position.z.abs() < 1e-4)
            .unwrap();
        assert!(center_v.position.y > 0.0);
    }

    #[test]
    fn test_sculpt_pinch_brush() {
        let plane_mesh = generate_plane(2.0, 8);
        let mut edit_mesh = EditableMesh::from_mesh(&plane_mesh);

        let hit_pt = Vec3::ZERO;
        let hit_norm = Vec3::Y;
        let settings = SculptSettings {
            brush: SculptBrush::Pinch,
            radius: 0.8,
            strength: 0.5,
            ..Default::default()
        };

        // Find a vertex near (0.25, 0.0, 0.0) before sculpt
        let v_init = edit_mesh
            .vertices
            .iter()
            .find(|v| v.position.x > 0.1 && v.position.x < 0.4 && v.position.z.abs() < 1e-4)
            .unwrap()
            .position;

        apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, Vec3::ZERO, &settings);

        let v_post = edit_mesh
            .vertices
            .iter()
            .find(|v| v.position.x > 0.0 && v.position.x < 0.4 && v.position.z.abs() < 1e-4)
            .unwrap()
            .position;

        // Pinch pulls towards center (0, 0, 0), so x must decrease
        assert!(
            v_post.x < v_init.x,
            "Pinch should pull vertex toward center"
        );
    }

    #[test]
    fn test_sculpt_symmetry_x() {
        let plane_mesh = generate_plane(4.0, 16);
        let mut edit_mesh = EditableMesh::from_mesh(&plane_mesh);

        let hit_pt = Vec3::new(1.0, 0.0, 0.0);
        let hit_norm = Vec3::Y;
        let settings = SculptSettings {
            brush: SculptBrush::Draw,
            radius: 0.6,
            strength: 0.5,
            symmetry_x: true,
            ..Default::default()
        };

        apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, Vec3::ZERO, &settings);

        // Positive X center (+1.0) and Negative X center (-1.0) should both have elevated Y
        let pos_v = edit_mesh
            .vertices
            .iter()
            .find(|v| (v.position.x - 1.0).abs() < 0.1 && v.position.z.abs() < 0.1)
            .unwrap();
        let neg_v = edit_mesh
            .vertices
            .iter()
            .find(|v| (v.position.x - (-1.0)).abs() < 0.1 && v.position.z.abs() < 0.1)
            .unwrap();

        assert!(pos_v.position.y > 0.0, "Positive side must be displaced");
        assert!(
            neg_v.position.y > 0.0,
            "Negative side must be mirrored via symmetry_x"
        );
    }

    #[test]
    fn test_sculpt_snake_hook_and_elastic_deform() {
        let plane_mesh = generate_plane(2.0, 8);
        let mut edit_mesh = EditableMesh::from_mesh(&plane_mesh);

        let hit_pt = Vec3::new(0.0, 0.0, 0.0);
        let hit_norm = Vec3::Y;
        let pull_delta = Vec3::new(0.5, 0.8, 0.0);

        let settings = SculptSettings {
            brush: SculptBrush::SnakeHook,
            radius: 0.6,
            strength: 0.8,
            ..Default::default()
        };

        let affected = apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, pull_delta, &settings);
        assert!(affected > 0);

        let displaced_v = edit_mesh
            .vertices
            .iter()
            .find(|v| v.position.y > 0.1)
            .unwrap();
        assert!(displaced_v.position.y > 0.1, "Snake Hook must pull vertices along delta");

        // Test ElasticDeform
        let elastic_settings = SculptSettings {
            brush: SculptBrush::ElasticDeform,
            radius: 0.6,
            strength: 0.5,
            ..Default::default()
        };
        let affected_elastic =
            apply_sculpt_stroke(&mut edit_mesh, hit_pt, hit_norm, Vec3::new(0.0, 0.5, 0.0), &elastic_settings);
        assert!(affected_elastic > 0);
    }
}
