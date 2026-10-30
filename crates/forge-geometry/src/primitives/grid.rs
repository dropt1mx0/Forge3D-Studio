use crate::vertex::LineVertex;

/// Generates line vertices for a ground grid in the XZ plane with colored coordinate axes.
///
/// Returns a list of vertex pairs (for `PrimitiveTopology::LineList`).
pub fn generate_grid_lines(half_size: f32, step: f32) -> Vec<LineVertex> {
    let mut lines = Vec::new();
    let count = (half_size / step).round() as i32;

    let sub_color = [0.22, 0.22, 0.25, 1.0];
    let main_color = [0.35, 0.35, 0.40, 1.0];

    // Grid lines along X and Z
    for i in -count..=count {
        let coord = i as f32 * step;
        let color = if i % 5 == 0 { main_color } else { sub_color };

        // Line parallel to X (constant Z)
        lines.push(LineVertex::new([-half_size, 0.0, coord], color));
        lines.push(LineVertex::new([half_size, 0.0, coord], color));

        // Line parallel to Z (constant X)
        lines.push(LineVertex::new([coord, 0.0, -half_size], color));
        lines.push(LineVertex::new([coord, 0.0, half_size], color));
    }

    // Explicit coordinate axes with standard RGB colors:
    // +X (Red)
    lines.push(LineVertex::new([0.0, 0.001, 0.0], [0.9, 0.2, 0.2, 1.0]));
    lines.push(LineVertex::new(
        [half_size, 0.001, 0.0],
        [0.9, 0.2, 0.2, 1.0],
    ));

    // +Y (Green)
    lines.push(LineVertex::new([0.0, 0.0, 0.0], [0.2, 0.9, 0.2, 1.0]));
    lines.push(LineVertex::new(
        [0.0, half_size * 0.5, 0.0],
        [0.2, 0.9, 0.2, 1.0],
    ));

    // +Z (Blue)
    lines.push(LineVertex::new([0.0, 0.001, 0.0], [0.2, 0.4, 0.9, 1.0]));
    lines.push(LineVertex::new(
        [0.0, 0.001, half_size],
        [0.2, 0.4, 0.9, 1.0],
    ));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_generation() {
        let lines = generate_grid_lines(5.0, 1.0);
        assert!(!lines.is_empty());
        assert_eq!(lines.len() % 2, 0); // Each line has 2 vertices
    }
}
