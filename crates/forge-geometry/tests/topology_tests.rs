use forge_geometry::topology::*;
use forge_geometry::{EditFace, EditVertex, EditableMesh, generate_cube};
use forge_math::Vec3;

#[test]
fn test_build_single_triangle() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2]];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Failed to build single triangle");

    assert_eq!(mesh.num_vertices(), 3);
    assert_eq!(mesh.num_faces(), 1);
    assert_eq!(mesh.num_edges(), 3);
    assert_eq!(mesh.num_half_edges(), 6); // 3 face half-edges + 3 boundary half-edges

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert!(!report.is_closed);
    assert_eq!(report.boundary_loop_count, 1);
    assert_eq!(report.num_components, 1);
}

#[test]
fn test_build_cube() {
    let positions = vec![
        Vec3::new(-1.0, -1.0, -1.0), // 0
        Vec3::new(1.0, -1.0, -1.0),  // 1
        Vec3::new(1.0, 1.0, -1.0),   // 2
        Vec3::new(-1.0, 1.0, -1.0),  // 3
        Vec3::new(-1.0, -1.0, 1.0),  // 4
        Vec3::new(1.0, -1.0, 1.0),   // 5
        Vec3::new(1.0, 1.0, 1.0),    // 6
        Vec3::new(-1.0, 1.0, 1.0),   // 7
    ];

    let faces = vec![
        vec![0, 3, 2, 1], // -Z back
        vec![4, 5, 6, 7], // +Z front
        vec![0, 1, 5, 4], // -Y bottom
        vec![2, 3, 7, 6], // +Y top
        vec![0, 4, 7, 3], // -X left
        vec![1, 2, 6, 5], // +X right
    ];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Failed to build cube");

    assert_eq!(mesh.num_vertices(), 8);
    assert_eq!(mesh.num_faces(), 6);
    assert_eq!(mesh.num_edges(), 12);
    assert_eq!(mesh.num_half_edges(), 24);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert!(report.is_closed);
    assert_eq!(report.boundary_loop_count, 0);
    assert_eq!(report.euler_characteristic, 2); // 8 - 12 + 6 = 2
}

#[test]
fn test_build_open_plane() {
    let positions = vec![
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Failed to build open plane");

    assert_eq!(mesh.num_vertices(), 4);
    assert_eq!(mesh.num_faces(), 1);
    assert_eq!(mesh.num_edges(), 4);
    assert_eq!(mesh.num_half_edges(), 8);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert!(!report.is_closed);
    assert_eq!(report.boundary_loop_count, 1);
}

#[test]
fn test_build_multi_component() {
    let positions = vec![
        // Component 0: triangle
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        // Component 1: triangle
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(11.0, 0.0, 0.0),
        Vec3::new(10.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2], vec![3, 4, 5]];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Failed to build multi-component");

    assert_eq!(mesh.num_vertices(), 6);
    assert_eq!(mesh.num_faces(), 2);
    assert_eq!(mesh.num_edges(), 6);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert_eq!(report.num_components, 2);
}

#[test]
fn test_manifold_mesh_validation() {
    // Octahedron (8 triangles, 6 vertices, 12 edges, closed manifold)
    let positions = vec![
        Vec3::new(1.0, 0.0, 0.0),  // 0 +X
        Vec3::new(-1.0, 0.0, 0.0), // 1 -X
        Vec3::new(0.0, 1.0, 0.0),  // 2 +Y
        Vec3::new(0.0, -1.0, 0.0), // 3 -Y
        Vec3::new(0.0, 0.0, 1.0),  // 4 +Z
        Vec3::new(0.0, 0.0, -1.0), // 5 -Z
    ];
    let faces = vec![
        vec![0, 2, 4],
        vec![2, 1, 4],
        vec![1, 3, 4],
        vec![3, 0, 4],
        vec![2, 0, 5],
        vec![1, 2, 5],
        vec![3, 1, 5],
        vec![0, 3, 5],
    ];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Failed to build octahedron");
    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert!(report.is_closed);
    assert_eq!(report.euler_characteristic, 2); // 6 - 12 + 8 = 2
}

#[test]
fn test_non_manifold_mesh_builder_rejection() {
    // 3 faces sharing edge (0, 1) -> Non-manifold edge
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    let faces = vec![
        vec![0, 1, 2],
        vec![1, 0, 3],
        vec![0, 1, 4], // Third face on edge (0, 1)
    ];

    let result = TopologyBuilder::build(&positions, &faces);
    assert!(matches!(result, Err(TopologyError::NonManifold(_))));
}

#[test]
fn test_non_manifold_vertex_detection() {
    // Hourglass configuration: two triangles touching at apex vertex 0
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),   // 0 shared apex
        Vec3::new(-1.0, 1.0, 0.0),  // 1
        Vec3::new(-1.0, -1.0, 0.0), // 2
        Vec3::new(1.0, 1.0, 0.0),   // 3
        Vec3::new(1.0, -1.0, 0.0),  // 4
    ];
    let faces = vec![vec![0, 1, 2], vec![0, 3, 4]];

    let mesh = TopologyBuilder::build(&positions, &faces).expect("Built hourglass mesh");
    let report = mesh.validate_topology();
    assert!(!report.is_manifold);
    assert!(!report.non_manifold_vertices.is_empty());
}

#[test]
fn test_twin_relationships() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    for he_h in mesh.half_edges() {
        let he = mesh.half_edge(he_h).unwrap();
        let twin = mesh.half_edge(he.twin).unwrap();
        assert_eq!(twin.twin, he_h, "Twin involution failed for {he_h}");
        assert_eq!(he.edge, twin.edge, "Twin edge mismatch for {he_h}");
    }
}

#[test]
fn test_next_prev_relationships() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    for he_h in mesh.half_edges() {
        let he = mesh.half_edge(he_h).unwrap();
        let next = mesh.half_edge(he.next).unwrap();
        let prev = mesh.half_edge(he.prev).unwrap();
        assert_eq!(next.prev, he_h, "next.prev == self failed for {he_h}");
        assert_eq!(prev.next, he_h, "prev.next == self failed for {he_h}");
    }
}

#[test]
fn test_boundary_traversal() {
    let positions = vec![
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let loops = mesh.boundary_loops();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);

    for &bnd_he in &loops[0] {
        let he = mesh.half_edge(bnd_he).unwrap();
        assert!(he.is_boundary());
        assert!(he.face.is_none());
    }
}

#[test]
fn test_vertex_adjacency() {
    // Tetrahedron
    let positions = vec![
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
    ];
    let faces = vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 1], vec![1, 3, 2]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let v0 = mesh.vertices().next().unwrap();
    assert_eq!(mesh.vertex_valence(v0), 3);
    assert_eq!(mesh.outgoing_half_edges(v0).len(), 3);
    assert_eq!(mesh.incoming_half_edges(v0).len(), 3);
    assert_eq!(mesh.incident_edges(v0).len(), 3);
    assert_eq!(mesh.adjacent_vertices(v0).len(), 3);
    assert_eq!(mesh.adjacent_faces(v0).len(), 3);
    assert!(!mesh.is_vertex_boundary(v0));
}

#[test]
fn test_face_adjacency_and_geometry() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let f0 = mesh.faces().next().unwrap();
    assert_eq!(mesh.face_vertex_count(f0), 4);
    assert_eq!(mesh.face_vertices(f0).len(), 4);
    assert_eq!(mesh.face_edges(f0).len(), 4);
    assert_eq!(mesh.face_half_edges(f0).len(), 4);

    let area = mesh.face_area(f0);
    assert!((area - 4.0).abs() < 1e-4, "Expected area 4.0, got {area}");

    let center = mesh.face_center(f0);
    assert!((center - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-4);

    let normal = mesh.face_normal(f0);
    assert!((normal - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-4);
}

#[test]
fn test_edge_loop_traversal() {
    // 2x2 quad plane: 9 vertices, 4 quads
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
    ];
    let faces = vec![
        vec![0, 1, 4, 3],
        vec![1, 2, 5, 4],
        vec![3, 4, 7, 6],
        vec![4, 5, 8, 7],
    ];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    // Edge between (3, 4)
    let e = mesh
        .edges()
        .find(|&eh| {
            let (v0, v1) = mesh.edge_vertices(eh).unwrap();
            (v0.index() == 3 && v1.index() == 4) || (v0.index() == 4 && v1.index() == 3)
        })
        .unwrap();

    let loop_edges = mesh.edge_loop(e);
    assert!(loop_edges.len() >= 2);
}

#[test]
fn test_edge_ring_traversal() {
    // 2x2 quad plane
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
    ];
    let faces = vec![
        vec![0, 1, 4, 3],
        vec![1, 2, 5, 4],
        vec![3, 4, 7, 6],
        vec![4, 5, 8, 7],
    ];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let e = mesh.edges().next().unwrap();
    let ring_edges = mesh.edge_ring(e);
    assert!(!ring_edges.is_empty());
}

#[test]
fn test_shortest_edge_and_vertex_path() {
    // Tetrahedron
    let positions = vec![
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
    ];
    let faces = vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 1], vec![1, 3, 2]];
    let mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let v_list: Vec<_> = mesh.vertices().collect();
    let v_start = v_list[0];
    let v_goal = v_list[1];

    let edge_path = mesh
        .shortest_edge_path(v_start, v_goal)
        .expect("Path exists");
    assert_eq!(edge_path.len(), 1);

    let vert_path = mesh
        .shortest_vertex_path(v_start, v_goal)
        .expect("Path exists");
    assert_eq!(vert_path.len(), 2);
    assert_eq!(vert_path[0], v_start);
    assert_eq!(vert_path[1], v_goal);
}

#[test]
fn test_edge_split() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let e = mesh.edges().next().unwrap();
    let res = mesh.split_edge(e, 0.5).expect("Split edge failed");
    let v_new = res.vertex;
    let e_new = res.new_edge;

    assert!(mesh.is_valid_vertex(v_new));
    assert!(mesh.is_valid_edge(e_new));
    assert_eq!(mesh.num_vertices(), 4);
    assert_eq!(mesh.num_edges(), 4);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_edge_collapse() {
    // Octahedron
    let positions = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    let faces = vec![
        vec![0, 2, 4],
        vec![2, 1, 4],
        vec![1, 3, 4],
        vec![3, 0, 4],
        vec![2, 0, 5],
        vec![1, 2, 5],
        vec![3, 1, 5],
        vec![0, 3, 5],
    ];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let e = mesh.edges().next().unwrap();
    let old_v_count = mesh.num_vertices();
    let old_f_count = mesh.num_faces();

    let res = mesh.collapse_edge(e, 0.5).expect("Collapse edge failed");
    let v_merged = res.surviving_vertex();
    assert!(mesh.is_valid_vertex(v_merged));
    assert_eq!(mesh.num_vertices(), old_v_count - 1);
    assert_eq!(mesh.num_faces(), old_f_count - 2);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_edge_collapse_link_condition_rejection() {
    // Single triangle with 3 vertices
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let e = mesh.edges().next().unwrap();
    // Collapsing a single triangle's edge would collapse the mesh into too few faces
    let res = mesh.collapse_edge(e, 0.5);
    assert!(res.is_err());
}

#[test]
fn test_edge_flip() {
    // Quad formed by 2 triangles:
    // (0,0) - (2,0) - (2,2) - (0,2)
    // Diagonal connects (0, 2)
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0), // 0
        Vec3::new(2.0, 0.0, 0.0), // 1
        Vec3::new(2.0, 2.0, 0.0), // 2
        Vec3::new(0.0, 2.0, 0.0), // 3
    ];
    let faces = vec![
        vec![0, 1, 2], // Triangle 0
        vec![0, 2, 3], // Triangle 1
    ];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    // Find the interior shared edge (0, 2)
    let shared_edge = mesh
        .edges()
        .find(|&eh| !mesh.is_edge_boundary(eh))
        .expect("Shared edge exists");

    mesh.flip_edge(shared_edge).expect("Flip edge failed");

    // The new edge should now connect vertices 1 and 3
    let (v_a, v_b) = mesh.edge_vertices(shared_edge).unwrap();
    let idx_a = v_a.index();
    let idx_b = v_b.index();
    assert!((idx_a == 1 && idx_b == 3) || (idx_a == 3 && idx_b == 1));

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_face_split() {
    // Quad plane (0, 1, 2, 3)
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let f = mesh.faces().next().unwrap();
    let verts: Vec<_> = mesh.vertices().collect();
    let v0 = verts[0];
    let v2 = verts[2];

    let split_res = mesh.split_face(f, v0, v2).expect("Split face failed");
    let (f_new, e_new) = (split_res.new_face, split_res.new_edge);
    assert_eq!(split_res.original_face, f);
    assert_eq!(split_res.face1_vertices.len(), 3);
    assert_eq!(split_res.face2_vertices.len(), 3);
    assert!(mesh.is_valid_face(f_new));
    assert!(mesh.is_valid_edge(e_new));
    assert_eq!(mesh.num_faces(), 2);
    assert_eq!(mesh.num_edges(), 5);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_face_merge() {
    // 2 triangles sharing an edge
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let shared_edge = mesh.edges().find(|&eh| !mesh.is_edge_boundary(eh)).unwrap();
    let res = mesh
        .dissolve_edge(shared_edge)
        .expect("Dissolve edge failed");

    assert!(mesh.is_valid_face(res.surviving_face()));
    assert!(!mesh.is_valid_face(res.removed_face()));
    assert!(!mesh.is_valid_edge(res.dissolved_edge()));
    assert_eq!(res.boundary_vertices().len(), 4);
    assert_eq!(mesh.num_faces(), 1);
    assert_eq!(mesh.face_vertex_count(res.surviving_face()), 4);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_poke_face() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
    let f = mesh.faces().next().unwrap();

    let res = mesh
        .poke_face(f, Vec3::new(1.0, 1.0, 0.0))
        .expect("Poke face failed");
    assert!(mesh.is_valid_face(res.original_face()));
    assert!(mesh.is_valid_vertex(res.new_vertex()));
    assert_eq!(res.new_faces().len(), 3);
    assert_eq!(res.new_edges().len(), 4);
    assert_eq!(res.new_half_edges().len(), 8);
    assert_eq!(mesh.num_faces(), 4);
    assert_eq!(mesh.num_edges(), 8);
    assert_eq!(mesh.num_vertices(), 5);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
}

#[test]
fn test_remove_face_topological() {
    // 2 triangles sharing an edge
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2], vec![0, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let f_to_remove = mesh.faces().next().unwrap();
    mesh.remove_face(f_to_remove).expect("Remove face failed");

    assert_eq!(mesh.num_faces(), 1);
    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert_eq!(report.boundary_loop_count, 1);
}

#[test]
fn test_remove_isolated_vertex() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(5.0, 5.0, 5.0), // isolated vertex 3
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    assert_eq!(mesh.num_vertices(), 4);
    let report = mesh.validate_topology();
    assert_eq!(report.isolated_vertices.len(), 1);

    let v_isolated = report.isolated_vertices[0];
    mesh.remove_isolated_vertex(v_isolated)
        .expect("Remove isolated vertex failed");

    assert_eq!(mesh.num_vertices(), 3);
    let report_after = mesh.validate_topology();
    assert!(report_after.isolated_vertices.is_empty());

    // Attempting to remove connected vertex should fail
    let v_connected = mesh.vertices().next().unwrap();
    assert!(mesh.remove_isolated_vertex(v_connected).is_err());
}

#[test]
fn test_conversion_roundtrip_editable_mesh() {
    let edit_mesh = EditableMesh::from_mesh(&generate_cube(1.0));
    let half_edge_mesh = HalfEdgeMesh::from_editable_mesh(&edit_mesh).expect("To HalfEdgeMesh");

    let report = half_edge_mesh.validate_topology();
    assert!(report.is_valid);

    let roundtrip_edit = half_edge_mesh.to_editable_mesh();
    assert_eq!(roundtrip_edit.vertices.len(), half_edge_mesh.num_vertices());
    assert_eq!(roundtrip_edit.faces.len(), half_edge_mesh.num_faces());
}

#[test]
fn test_conversion_roundtrip_gpu_mesh() {
    let gpu_cube = generate_cube(1.0);
    let half_edge_mesh = HalfEdgeMesh::from_mesh(&gpu_cube).expect("From GPU Mesh");

    assert_eq!(half_edge_mesh.num_vertices(), 8);
    let report = half_edge_mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_closed);

    let reconstructed_gpu = half_edge_mesh.to_mesh();
    assert!(!reconstructed_gpu.vertices.is_empty());
    assert!(!reconstructed_gpu.indices.is_empty());
    assert_eq!(reconstructed_gpu.indices.len(), 36); // 12 triangles * 3
}

#[test]
fn test_preservation_of_uv_seams_and_hard_normals() {
    let gpu_cube = generate_cube(1.0);
    let half_edge_mesh = HalfEdgeMesh::from_mesh(&gpu_cube).unwrap();

    // Verify each half-edge carries corner UV and normal
    for he_h in half_edge_mesh.half_edges() {
        let he = half_edge_mesh.half_edge(he_h).unwrap();
        if he.face.is_some() {
            assert!(
                he.uv.is_some(),
                "Corner UV should be preserved on face half-edge"
            );
            assert!(
                he.normal.is_some(),
                "Corner normal should be preserved on face half-edge"
            );
        }
    }
}

#[test]
fn test_invalid_handles_and_generation() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(5.0, 5.0, 5.0),
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let isolated_v = mesh
        .vertices()
        .find(|&v| mesh.vertex(v).unwrap().half_edge.is_none())
        .unwrap();
    let stale_handle = isolated_v;

    // Delete vertex, generation counter increments
    mesh.remove_isolated_vertex(isolated_v).unwrap();

    // Stale handle should now be invalid
    assert!(!mesh.is_valid_vertex(stale_handle));
    assert!(mesh.vertex(stale_handle).is_none());

    // Out of bounds handle
    let out_of_bounds = VertexHandle::new(99999, 0);
    assert!(!mesh.is_valid_vertex(out_of_bounds));
}

#[test]
fn test_transactional_rollback_on_failed_operation() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();

    let initial_verts = mesh.num_vertices();
    let initial_edges = mesh.num_edges();

    // Try illegal flip on boundary edge
    let bnd_edge = mesh.edges().next().unwrap();
    let res = mesh.flip_edge(bnd_edge);
    assert!(res.is_err());

    // Mesh state must be completely untouched
    assert_eq!(mesh.num_vertices(), initial_verts);
    assert_eq!(mesh.num_edges(), initial_edges);
}

// =========================================================================
// Phase 6.2 — Half-Edge Topology Builder Tests
// =========================================================================

fn make_test_editable_mesh(positions: &[Vec3], faces: &[Vec<usize>]) -> EditableMesh {
    EditableMesh {
        vertices: positions
            .iter()
            .map(|&p| EditVertex::new(p, Vec3::Y, forge_math::Vec2::ZERO))
            .collect(),
        edges: Vec::new(),
        faces: faces
            .iter()
            .map(|f| EditFace::new(f.clone(), Vec3::Y))
            .collect(),
    }
}

#[test]
fn test_builder_single_triangle() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 2]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).expect("Build single triangle");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.edge_count(), 3);
    assert_eq!(mesh.half_edge_count(), 6);
    assert!(!mesh.is_empty());
}

#[test]
fn test_builder_quad() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 2, 3]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).expect("Build quad");

    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.edge_count(), 4);
    assert_eq!(mesh.half_edge_count(), 8);
}

#[test]
fn test_builder_two_adjacent_triangles() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
        ],
        &[vec![0, 1, 2], vec![0, 2, 3]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).expect("Build adjacent triangles");

    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.face_count(), 2);
    assert_eq!(mesh.edge_count(), 5); // 4 boundary edges + 1 shared diagonal
    assert_eq!(mesh.half_edge_count(), 10); // 2*3 face half-edges + 4 boundary half-edges = 10
}

#[test]
fn test_builder_twin_correctness() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
        ],
        &[vec![0, 1, 2], vec![0, 2, 3]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).unwrap();

    // Verify involution and shared edge for all half-edges
    for he_h in mesh.half_edges() {
        let he = mesh.half_edge(he_h).unwrap();
        let twin = mesh.half_edge(he.twin).unwrap();
        assert_eq!(twin.twin, he_h, "Twin of twin must be self");
        assert_eq!(
            he.edge, twin.edge,
            "Half-edge and twin must share the same EdgeHandle"
        );
    }

    // Specifically test interior shared edge between (0, 2)
    let interior_edges: Vec<_> = mesh
        .edges()
        .filter(|&eh| {
            let (h0, h1) = mesh.edge_half_edges(eh).unwrap();
            mesh.half_edge(h0).unwrap().face.is_some() && mesh.half_edge(h1).unwrap().face.is_some()
        })
        .collect();
    assert_eq!(
        interior_edges.len(),
        1,
        "There must be exactly one interior shared edge"
    );
}

#[test]
fn test_builder_next_prev_consistency() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 2]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).unwrap();

    for he_h in mesh.half_edges() {
        let he = mesh.half_edge(he_h).unwrap();
        let next_he = mesh.half_edge(he.next).expect("next must exist");
        let prev_he = mesh.half_edge(he.prev).expect("prev must exist");
        assert_eq!(next_he.prev, he_h, "next.prev must equal self");
        assert_eq!(prev_he.next, he_h, "prev.next must equal self");
    }
}

#[test]
fn test_builder_boundary_half_edges() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 2]],
    );

    let mesh = TopologyBuilder::from_editable_mesh(&edit_mesh).unwrap();

    let mut boundary_count = 0;
    let mut interior_count = 0;
    for he_h in mesh.half_edges() {
        let he = mesh.half_edge(he_h).unwrap();
        if he.is_boundary() {
            assert!(
                he.face.is_none(),
                "Boundary half-edge must have face == None"
            );
            boundary_count += 1;
        } else {
            assert!(
                he.face.is_some(),
                "Interior face half-edge must have face == Some"
            );
            interior_count += 1;
        }
    }
    assert_eq!(boundary_count, 3);
    assert_eq!(interior_count, 3);
}

#[test]
fn test_builder_disconnected_polygons() {
    let edit_mesh = make_test_editable_mesh(
        &[
            // Triangle 1
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            // Triangle 2 (disconnected)
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(11.0, 0.0, 0.0),
            Vec3::new(10.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 2], vec![3, 4, 5]],
    );

    let mesh =
        TopologyBuilder::from_editable_mesh(&edit_mesh).expect("Build disconnected polygons");
    assert_eq!(mesh.vertex_count(), 6);
    assert_eq!(mesh.face_count(), 2);
    assert_eq!(mesh.edge_count(), 6);
    assert_eq!(mesh.half_edge_count(), 12);
}

#[test]
fn test_builder_invalid_vertex_index() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        &[vec![0, 1, 99]], // 99 is out of bounds
    );

    let res = TopologyBuilder::from_editable_mesh(&edit_mesh);
    assert!(matches!(res, Err(TopologyError::BuildError(_))));
}

#[test]
fn test_builder_polygon_insufficient_vertices() {
    let edit_mesh = make_test_editable_mesh(
        &[Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
        &[vec![0, 1]], // only 2 vertices
    );

    let res = TopologyBuilder::from_editable_mesh(&edit_mesh);
    assert!(matches!(res, Err(TopologyError::Degenerate(_))));
}

#[test]
fn test_builder_non_manifold_edge() {
    let edit_mesh = make_test_editable_mesh(
        &[
            Vec3::new(0.0, 0.0, 0.0),  // 0
            Vec3::new(1.0, 0.0, 0.0),  // 1
            Vec3::new(0.0, 1.0, 0.0),  // 2
            Vec3::new(0.0, -1.0, 0.0), // 3
            Vec3::new(0.0, 0.0, 1.0),  // 4
        ],
        &[
            vec![0, 1, 2],
            vec![1, 0, 3],
            vec![0, 1, 4], // 3rd face incident on edge (0, 1)
        ],
    );

    let res = TopologyBuilder::from_editable_mesh(&edit_mesh);
    assert!(matches!(res, Err(TopologyError::NonManifold(_))));
}

#[test]
fn test_builder_no_partial_mesh_on_error() {
    let edit_mesh = make_test_editable_mesh(
        &[Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
        &[vec![0, 1, 0]], // consecutive duplicate vertex
    );

    let res = TopologyBuilder::from_editable_mesh(&edit_mesh);
    assert!(res.is_err());
    // Safe transactional semantic: error returned, no HalfEdgeMesh returned or leaked
    assert!(matches!(res, Err(TopologyError::Degenerate(_))));
}

#[test]
fn test_extrude_face() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
    let f = mesh.faces().next().unwrap();

    let res = mesh.extrude_face(f, 2.0).expect("Extrude face failed");
    assert_eq!(res.original_face(), f);
    assert!(mesh.is_valid_face(res.top_face()));
    assert_eq!(res.new_vertices().len(), 4);
    assert_eq!(res.new_edges().len(), 8);
    assert_eq!(res.new_faces().len(), 5);
    assert_eq!(res.new_half_edges().len(), 16);

    assert_eq!(mesh.num_vertices(), 8);
    assert_eq!(mesh.num_edges(), 12);
    assert_eq!(mesh.num_faces(), 6);
    assert_eq!(mesh.num_half_edges(), 24);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert!(report.is_closed);
    assert_eq!(report.boundary_loop_count, 0);
    assert_eq!(report.euler_characteristic, 2);
}

#[test]
fn test_inset_face() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3]];
    let mut mesh = TopologyBuilder::build(&positions, &faces).unwrap();
    let f = mesh.faces().next().unwrap();

    let res = mesh.inset_face(f, 0.5).expect("Inset face failed");
    assert_eq!(res.original_face(), f);
    assert!(mesh.is_valid_face(res.inner_face()));
    assert_eq!(res.new_vertices().len(), 4);
    assert_eq!(res.new_edges().len(), 8);
    assert_eq!(res.new_side_faces().len(), 3);
    assert_eq!(res.side_faces().len(), 4);
    assert_eq!(res.new_half_edges().len(), 16);

    assert_eq!(mesh.num_vertices(), 8);
    assert_eq!(mesh.num_edges(), 12);
    assert_eq!(mesh.num_faces(), 5);
    assert_eq!(mesh.num_half_edges(), 24);

    let report = mesh.validate_topology();
    assert!(report.is_valid);
    assert!(report.is_manifold);
    assert_eq!(report.boundary_loop_count, 1);
    assert_eq!(report.euler_characteristic, 1);
}
