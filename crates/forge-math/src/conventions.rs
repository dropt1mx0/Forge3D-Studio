use glam::Vec3;

/// Forge3D standard coordinate conventions:
/// - Right-handed coordinate system
/// - +Y is UP
/// - +X is RIGHT
/// - +Z is OUT towards viewer / backward in view space, -Z is forward into screen
/// - Units: Meters
pub const UP: Vec3 = Vec3::Y;
pub const DOWN: Vec3 = Vec3::NEG_Y;
pub const RIGHT: Vec3 = Vec3::X;
pub const LEFT: Vec3 = Vec3::NEG_X;
pub const FORWARD: Vec3 = Vec3::NEG_Z;
pub const BACKWARD: Vec3 = Vec3::Z;

/// Converts degrees to radians.
#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    deg.to_radians()
}

/// Converts radians to degrees.
#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    rad.to_degrees()
}

/// Converts a 3D vector of degrees to radians.
#[inline]
pub fn vec3_deg_to_rad(deg: Vec3) -> Vec3 {
    Vec3::new(deg.x.to_radians(), deg.y.to_radians(), deg.z.to_radians())
}

/// Converts a 3D vector of radians to degrees.
#[inline]
pub fn vec3_rad_to_deg(rad: Vec3) -> Vec3 {
    Vec3::new(rad.x.to_degrees(), rad.y.to_degrees(), rad.z.to_degrees())
}
