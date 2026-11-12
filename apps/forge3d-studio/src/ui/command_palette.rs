use eframe::egui::{self, Key};
use forge_modifiers::{
    ArrayModifier, BevelModifier, DecimateModifier, DisplaceModifier, MirrorModifier, ModifierType,
    SolidifyModifier, SubdivisionModifier, WeightedNormalModifier,
};
use forge_scene::{EditMode, PrimitiveType, WorkspaceMode};

use crate::app::Forge3dStudioApp;

pub struct CommandItem {
    pub category: &'static str,
    pub title: &'static str,
    pub shortcut: &'static str,
    pub action_id: &'static str,
}

pub fn get_all_commands() -> Vec<CommandItem> {
    vec![
        // Add Objects
        CommandItem {
            category: "Add",
            title: "Add Cube",
            shortcut: "Shift+A",
            action_id: "add_cube",
        },
        CommandItem {
            category: "Add",
            title: "Add Cylinder",
            shortcut: "",
            action_id: "add_cylinder",
        },
        CommandItem {
            category: "Add",
            title: "Add UV Sphere",
            shortcut: "",
            action_id: "add_uv_sphere",
        },
        CommandItem {
            category: "Add",
            title: "Add Icosphere",
            shortcut: "",
            action_id: "add_icosphere",
        },
        CommandItem {
            category: "Add",
            title: "Add Monkey (Suzanne)",
            shortcut: "",
            action_id: "add_monkey",
        },
        CommandItem {
            category: "Add",
            title: "Add Plane",
            shortcut: "",
            action_id: "add_plane",
        },
        CommandItem {
            category: "Add",
            title: "Add Cone",
            shortcut: "",
            action_id: "add_cone",
        },
        CommandItem {
            category: "Add",
            title: "Add Torus",
            shortcut: "",
            action_id: "add_torus",
        },
        CommandItem {
            category: "Add",
            title: "Add Capsule",
            shortcut: "",
            action_id: "add_capsule",
        },
        CommandItem {
            category: "Add",
            title: "Add Point Light",
            shortcut: "",
            action_id: "add_point_light",
        },
        CommandItem {
            category: "Add",
            title: "Add Directional Light",
            shortcut: "",
            action_id: "add_directional_light",
        },
        CommandItem {
            category: "Add",
            title: "Add Camera",
            shortcut: "",
            action_id: "add_camera",
        },
        // Mesh Modeling
        CommandItem {
            category: "Mesh",
            title: "Extrude Faces",
            shortcut: "E",
            action_id: "mesh_extrude",
        },
        CommandItem {
            category: "Mesh",
            title: "Inset Faces",
            shortcut: "I",
            action_id: "mesh_inset",
        },
        CommandItem {
            category: "Mesh",
            title: "Bevel Edges",
            shortcut: "Ctrl+B",
            action_id: "mesh_bevel",
        },
        CommandItem {
            category: "Mesh",
            title: "Loop Cut and Slide",
            shortcut: "Ctrl+R",
            action_id: "mesh_loop_cut",
        },
        CommandItem {
            category: "Mesh",
            title: "Dissolve Selection",
            shortcut: "Ctrl+X",
            action_id: "mesh_dissolve",
        },
        CommandItem {
            category: "Mesh",
            title: "Bridge Edge Loops",
            shortcut: "",
            action_id: "mesh_bridge",
        },
        CommandItem {
            category: "Mesh",
            title: "Separate Selection",
            shortcut: "P",
            action_id: "mesh_separate",
        },
        CommandItem {
            category: "Mesh",
            title: "Knife Connect Vertices",
            shortcut: "K",
            action_id: "mesh_knife",
        },
        CommandItem {
            category: "Mesh",
            title: "Bisect Mesh Plane",
            shortcut: "Alt+B",
            action_id: "mesh_bisect",
        },
        CommandItem {
            category: "Mesh",
            title: "Spin Revolve Around Axis",
            shortcut: "Alt+S",
            action_id: "mesh_spin",
        },
        CommandItem {
            category: "Mesh",
            title: "Fill Face / Edge",
            shortcut: "F",
            action_id: "mesh_fill",
        },
        CommandItem {
            category: "Mesh",
            title: "Select Linked",
            shortcut: "L",
            action_id: "mesh_select_linked",
        },
        CommandItem {
            category: "Mesh",
            title: "Deselect Linked",
            shortcut: "Shift+L",
            action_id: "mesh_deselect_linked",
        },
        CommandItem {
            category: "Mesh",
            title: "Merge at Center",
            shortcut: "M",
            action_id: "mesh_merge",
        },
        CommandItem {
            category: "Mesh",
            title: "Subdivide Mesh",
            shortcut: "",
            action_id: "mesh_subdivide",
        },
        CommandItem {
            category: "Mesh",
            title: "Weld Co-located Vertices",
            shortcut: "",
            action_id: "mesh_weld",
        },
        CommandItem {
            category: "Mesh",
            title: "Tris to Quads",
            shortcut: "",
            action_id: "mesh_tris_to_quads",
        },
        // UV Operations
        CommandItem {
            category: "UV",
            title: "Smart UV Project",
            shortcut: "U",
            action_id: "uv_smart_project",
        },
        CommandItem {
            category: "UV",
            title: "Cube / Box UV Project",
            shortcut: "",
            action_id: "uv_box_project",
        },
        CommandItem {
            category: "UV",
            title: "Sphere UV Project",
            shortcut: "",
            action_id: "uv_sphere_project",
        },
        CommandItem {
            category: "UV",
            title: "Cylinder UV Project",
            shortcut: "",
            action_id: "uv_cylinder_project",
        },
        CommandItem {
            category: "UV",
            title: "Pack UV Islands",
            shortcut: "",
            action_id: "uv_pack_islands",
        },
        // Modifiers
        CommandItem {
            category: "Modifier",
            title: "Add Mirror Modifier",
            shortcut: "",
            action_id: "mod_mirror",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Subdivision Surface Modifier",
            shortcut: "",
            action_id: "mod_subdivide",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Solidify Modifier",
            shortcut: "",
            action_id: "mod_solidify",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Array Modifier",
            shortcut: "",
            action_id: "mod_array",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Bevel Modifier",
            shortcut: "",
            action_id: "mod_bevel",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Decimate Modifier",
            shortcut: "",
            action_id: "mod_decimate",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Displace Modifier",
            shortcut: "",
            action_id: "mod_displace",
        },
        CommandItem {
            category: "Modifier",
            title: "Add Weighted Normal Modifier",
            shortcut: "",
            action_id: "mod_weighted_normal",
        },
        // Workspaces
        CommandItem {
            category: "Workspace",
            title: "Switch to Modeling Workspace",
            shortcut: "",
            action_id: "ws_modeling",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Sculpting Workspace",
            shortcut: "",
            action_id: "ws_sculpting",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to UV Editing Workspace",
            shortcut: "",
            action_id: "ws_uv",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Shading Workspace",
            shortcut: "",
            action_id: "ws_shading",
        },
        CommandItem {
            category: "Shading",
            title: "Open Shader Node Editor",
            shortcut: "",
            action_id: "open_shader_editor",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Animation Workspace",
            shortcut: "",
            action_id: "ws_animation",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Rendering Workspace",
            shortcut: "",
            action_id: "ws_rendering",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Geometry Nodes Workspace",
            shortcut: "",
            action_id: "ws_geonodes",
        },
        CommandItem {
            category: "Workspace",
            title: "Switch to Scripting Workspace",
            shortcut: "",
            action_id: "ws_scripting",
        },
        // File & Project
        CommandItem {
            category: "File",
            title: "New Project",
            shortcut: "Ctrl+N",
            action_id: "file_new",
        },
        CommandItem {
            category: "File",
            title: "Save Project",
            shortcut: "Ctrl+S",
            action_id: "file_save",
        },
        CommandItem {
            category: "File",
            title: "Import STL Mesh",
            shortcut: "",
            action_id: "import_stl",
        },
        CommandItem {
            category: "File",
            title: "Export STL Mesh",
            shortcut: "",
            action_id: "export_stl",
        },
        CommandItem {
            category: "File",
            title: "Import PLY Mesh",
            shortcut: "",
            action_id: "import_ply",
        },
        CommandItem {
            category: "File",
            title: "Export PLY Mesh",
            shortcut: "",
            action_id: "export_ply",
        },
        CommandItem {
            category: "File",
            title: "Import glTF 2.0 / GLB",
            shortcut: "",
            action_id: "import_gltf",
        },
        CommandItem {
            category: "File",
            title: "Export glTF 2.0 / GLB",
            shortcut: "",
            action_id: "export_gltf",
        },
        // View & Navigation
        CommandItem {
            category: "View",
            title: "Toggle Edit Mode",
            shortcut: "Tab",
            action_id: "toggle_edit_mode",
        },
        CommandItem {
            category: "View",
            title: "Frame Selected",
            shortcut: "Numpad .",
            action_id: "frame_selected",
        },
        CommandItem {
            category: "View",
            title: "Reset View / Home",
            shortcut: "Home",
            action_id: "view_home",
        },
        CommandItem {
            category: "Plugin",
            title: "Open Plugin & Extension Manager",
            shortcut: "",
            action_id: "open_plugin_manager",
        },
        CommandItem {
            category: "View",
            title: "Toggle Wireframe Overlay",
            shortcut: "Shift+Z",
            action_id: "toggle_wireframe",
        },
        CommandItem {
            category: "View",
            title: "Toggle X-Ray Mode",
            shortcut: "Alt+Z",
            action_id: "toggle_xray",
        },
        CommandItem {
            category: "View",
            title: "Toggle Local View (Isolate Selected)",
            shortcut: "Numpad /",
            action_id: "toggle_local_view",
        },
        CommandItem {
            category: "View",
            title: "Toggle Viewport Overlays",
            shortcut: "Shift+Alt+Z",
            action_id: "toggle_overlays",
        },
        // Outliner & Restriction Toggles
        CommandItem {
            category: "Outliner",
            title: "Outliner: Filter Meshes",
            shortcut: "",
            action_id: "outliner_filter_meshes",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Filter Lights",
            shortcut: "",
            action_id: "outliner_filter_lights",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Filter Cameras",
            shortcut: "",
            action_id: "outliner_filter_cameras",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Show All Types",
            shortcut: "",
            action_id: "outliner_filter_all",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Make All Selectable",
            shortcut: "",
            action_id: "outliner_all_selectable",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Make All Unselectable",
            shortcut: "",
            action_id: "outliner_all_unselectable",
        },
        CommandItem {
            category: "Outliner",
            title: "Outliner: Toggle Selectability Column",
            shortcut: "",
            action_id: "outliner_toggle_selectability_col",
        },
        // Edit Operations
        CommandItem {
            category: "Edit",
            title: "Undo",
            shortcut: "Ctrl+Z",
            action_id: "edit_undo",
        },
        CommandItem {
            category: "Edit",
            title: "Redo",
            shortcut: "Ctrl+Shift+Z",
            action_id: "edit_redo",
        },
        CommandItem {
            category: "Edit",
            title: "Select All",
            shortcut: "A",
            action_id: "select_all",
        },
        CommandItem {
            category: "Edit",
            title: "Deselect All",
            shortcut: "Alt+A",
            action_id: "deselect_all",
        },
        CommandItem {
            category: "Edit",
            title: "Delete Selected",
            shortcut: "X",
            action_id: "delete_selected",
        },
    ]
}

pub fn execute_command(app: &mut Forge3dStudioApp, action_id: &str) {
    match action_id {
        "add_cube" => {
            app.add_primitive(PrimitiveType::Cube);
        }
        "add_cylinder" => {
            app.add_primitive(PrimitiveType::Cylinder);
        }
        "add_uv_sphere" => {
            app.add_primitive(PrimitiveType::Sphere);
        }
        "add_icosphere" => {
            app.add_primitive(PrimitiveType::Icosphere);
        }
        "add_monkey" => {
            app.add_primitive(PrimitiveType::Monkey);
        }
        "add_plane" => {
            app.add_primitive(PrimitiveType::Plane);
        }
        "add_cone" => {
            app.add_primitive(PrimitiveType::Cone);
        }
        "add_torus" => {
            app.add_primitive(PrimitiveType::Torus);
        }
        "add_capsule" => {
            app.add_primitive(PrimitiveType::Capsule);
        }
        "add_point_light" => {
            app.add_point_light();
        }
        "add_directional_light" => {
            app.add_directional_light();
        }
        "add_camera" => {
            app.add_camera();
        }
        "mesh_extrude" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            let center = forge_math::Vec2::new(400.0, 300.0);
            if !app.begin_interactive_edit_tool(crate::app::InteractiveEditToolType::Extrude, center)
            {
                app.extrude_selected_geometry(0.5);
            }
        }
        "mesh_inset" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            let center = forge_math::Vec2::new(400.0, 300.0);
            if !app.begin_interactive_edit_tool(crate::app::InteractiveEditToolType::Inset, center)
            {
                app.inset_selected_geometry(0.15);
            }
        }
        "mesh_bevel" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.bevel_selected_geometry(0.1);
        }
        "mesh_loop_cut" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            let center = forge_math::Vec2::new(400.0, 300.0);
            if !app.begin_interactive_edit_tool(crate::app::InteractiveEditToolType::LoopCut, center)
            {
                app.loop_cut_selected_geometry(0.5);
            }
        }
        "mesh_dissolve" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.dissolve_selected_geometry();
        }
        "mesh_bridge" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.bridge_selected_geometry();
        }
        "mesh_separate" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.separate_selected_geometry();
        }
        "mesh_knife" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.knife_connect_selected_geometry();
        }
        "mesh_bisect" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.bisect_selected_geometry(forge_math::Vec3::ZERO, forge_math::Vec3::Y, true, true);
        }
        "mesh_spin" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.spin_selected_geometry(12, std::f32::consts::TAU);
        }
        "mesh_fill" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.fill_selected_geometry();
        }
        "mesh_select_linked" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.select_linked(false);
        }
        "mesh_deselect_linked" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.select_linked(true);
        }
        "mesh_merge" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.merge_selected_geometry();
        }
        "mesh_subdivide" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.subdivide_selected_geometry();
        }
        "mesh_weld" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.weld_colocated_selected_geometry(1e-4);
        }
        "mesh_tris_to_quads" => {
            if app.editor_state.edit_mode != EditMode::Edit {
                app.toggle_edit_mode();
            }
            app.tris_to_quads_selected_geometry();
        }
        "uv_smart_project" => {
            app.smart_project_selected_geometry();
        }
        "uv_box_project" => {
            app.box_project_selected_geometry(1.0);
        }
        "uv_sphere_project" => {
            app.sphere_project_selected_geometry();
        }
        "uv_cylinder_project" => {
            app.cylinder_project_selected_geometry(2.0);
        }
        "uv_pack_islands" => {
            app.pack_uv_islands_selected_geometry(0.02);
        }
        "mod_mirror" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Mirror(MirrorModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Mirror modifier".to_string();
            }
        }
        "mod_subdivide" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Subdivide(SubdivisionModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Subdivision Surface modifier".to_string();
            }
        }
        "mod_solidify" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Solidify(SolidifyModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Solidify modifier".to_string();
            }
        }
        "mod_array" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Array(ArrayModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Array modifier".to_string();
            }
        }
        "mod_bevel" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Bevel(BevelModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Bevel modifier".to_string();
            }
        }
        "mod_decimate" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Decimate(DecimateModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Decimate modifier".to_string();
            }
        }
        "mod_displace" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers
                    .add(ModifierType::Displace(DisplaceModifier::default()));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Displace modifier".to_string();
            }
        }
        "mod_weighted_normal" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object_mut(id)
            {
                obj.modifiers.add(ModifierType::WeightedNormal(
                    WeightedNormalModifier::default(),
                ));
                obj.invalidate_evaluated_mesh();
                app.status_message = "Added Weighted Normal modifier".to_string();
            }
        }
        "import_stl" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STL Mesh", &["stl"])
                .set_title("Import STL Mesh")
                .pick_file()
                && let Err(e) = app.import_stl_from_path(&path)
            {
                app.status_message = format!("Import error: {e}");
            }
        }
        "export_stl" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STL Mesh", &["stl"])
                .set_file_name("scene.stl")
                .set_title("Export STL Mesh")
                .save_file()
                && let Err(e) = app.export_stl_to_path(&path, false)
            {
                app.status_message = format!("Export error: {e}");
            }
        }
        "import_ply" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Stanford PLY", &["ply"])
                .set_title("Import Stanford PLY")
                .pick_file()
                && let Err(e) = app.import_ply_from_path(&path)
            {
                app.status_message = format!("Import error: {e}");
            }
        }
        "export_ply" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Stanford PLY", &["ply"])
                .set_file_name("scene.ply")
                .set_title("Export Stanford PLY")
                .save_file()
                && let Err(e) = app.export_ply_to_path(&path, false)
            {
                app.status_message = format!("Export error: {e}");
            }
        }
        "import_gltf" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("glTF 2.0 / GLB", &["glb", "gltf"])
                .set_title("Import glTF 2.0 / GLB")
                .pick_file()
                && let Err(e) = app.import_gltf_from_path(&path)
            {
                app.status_message = format!("Import error: {e}");
            }
        }
        "export_gltf" => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Binary glTF (GLB)", &["glb"])
                .set_file_name("scene.glb")
                .set_title("Export glTF 2.0 (GLB)")
                .save_file()
                && let Err(e) = app.export_gltf_to_path(&path, false)
            {
                app.status_message = format!("Export error: {e}");
            }
        }
        "ws_modeling" => {
            app.set_workspace(WorkspaceMode::Modeling);
        }
        "ws_sculpting" => {
            app.set_workspace(WorkspaceMode::Sculpting);
        }
        "ws_uv" => {
            app.set_workspace(WorkspaceMode::UvEditing);
        }
        "ws_shading" => {
            app.set_workspace(WorkspaceMode::Shading);
        }
        "open_shader_editor" => {
            app.open_shader_editor_for_selected();
            app.status_message = "Opened Shader Node Editor".into();
        }
        "ws_animation" => {
            app.set_workspace(WorkspaceMode::Animation);
        }
        "ws_rendering" => {
            app.set_workspace(WorkspaceMode::Rendering);
        }
        "ws_geonodes" => {
            app.set_workspace(WorkspaceMode::GeometryNodes);
        }
        "ws_scripting" => {
            app.set_workspace(WorkspaceMode::Scripting);
        }
        "toggle_edit_mode" => {
            app.toggle_edit_mode();
        }
        "edit_undo" => {
            app.undo();
        }
        "edit_redo" => {
            app.redo();
        }
        "select_all" => {
            app.select_all();
        }
        "deselect_all" => {
            app.deselect_all();
        }
        "delete_selected" => {
            app.delete_selected();
        }
        "toggle_wireframe" => {
            app.editor_state.show_wireframe = !app.editor_state.show_wireframe;
            app.status_message = format!("Wireframe: {}", app.editor_state.show_wireframe);
        }
        "toggle_xray" => {
            app.toggle_xray();
        }
        "toggle_local_view" => {
            app.toggle_local_view();
        }
        "toggle_overlays" => {
            app.toggle_overlays();
        }
        "outliner_filter_meshes" => {
            app.outliner_filter.type_filter = crate::ui::outliner::OutlinerTypeFilter::Meshes;
            app.status_message = "Outliner: Filtering Meshes".to_string();
        }
        "outliner_filter_lights" => {
            app.outliner_filter.type_filter = crate::ui::outliner::OutlinerTypeFilter::Lights;
            app.status_message = "Outliner: Filtering Lights".to_string();
        }
        "outliner_filter_cameras" => {
            app.outliner_filter.type_filter = crate::ui::outliner::OutlinerTypeFilter::Cameras;
            app.status_message = "Outliner: Filtering Cameras".to_string();
        }
        "outliner_filter_all" => {
            app.outliner_filter.type_filter = crate::ui::outliner::OutlinerTypeFilter::All;
            app.status_message = "Outliner: Showing All Types".to_string();
        }
        "outliner_all_selectable" => {
            app.make_all_objects_selectable(true);
        }
        "outliner_all_unselectable" => {
            app.make_all_objects_selectable(false);
        }
        "outliner_toggle_selectability_col" => {
            app.toggle_outliner_selectability_column();
        }
        "frame_selected" => {
            if let Some(id) = app.selection.active_id
                && let Some(obj) = app.scene.get_object(id)
            {
                app.camera.target = obj.local_transform.translation;
                app.status_message = format!("Framed {}", obj.name);
            }
        }
        "view_home" => {
            app.camera.target = forge_math::Vec3::ZERO;
            app.camera.eye = forge_math::Vec3::new(6.0, 5.0, 8.0);
            app.camera.up = forge_math::Vec3::Y;
            app.status_message = "View reset to Home".to_string();
        }
        "open_plugin_manager" => {
            app.show_plugin_manager = true;
        }
        _ => {}
    }
}

pub fn render_command_palette(app: &mut Forge3dStudioApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    let mut close = false;
    let mut execute_action = None;

    egui::Window::new("Quick Operator Search")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .fixed_size(egui::vec2(520.0, 360.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⌕").size(16.0));
                let search_input = ui.add(
                    egui::TextEdit::singleline(&mut app.command_palette_query)
                        .hint_text("Search operator or command (e.g. Cube, Bevel, Extrude)...")
                        .desired_width(450.0),
                );
                search_input.request_focus();
            });

            ui.separator();

            let all_commands = get_all_commands();
            let query_lower = app.command_palette_query.to_lowercase();
            let matching: Vec<&CommandItem> = all_commands
                .iter()
                .filter(|c| {
                    c.title.to_lowercase().contains(&query_lower)
                        || c.category.to_lowercase().contains(&query_lower)
                        || c.shortcut.to_lowercase().contains(&query_lower)
                })
                .collect();

            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }

            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !matching.is_empty() {
                app.command_palette_selected_index =
                    (app.command_palette_selected_index + 1) % matching.len();
            }

            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !matching.is_empty() {
                if app.command_palette_selected_index == 0 {
                    app.command_palette_selected_index = matching.len() - 1;
                } else {
                    app.command_palette_selected_index -= 1;
                }
            }

            if ctx.input(|i| i.key_pressed(Key::Enter)) && !matching.is_empty() {
                let idx = app.command_palette_selected_index.min(matching.len() - 1);
                execute_action = Some(matching[idx].action_id.to_string());
                close = true;
            }

            egui::ScrollArea::vertical()
                .id_salt("command_palette_scroll")
                .max_height(280.0)
                .show(ui, |ui| {
                    if matching.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("No matching commands found")
                                    .weak()
                                    .italics(),
                            );
                        });
                    } else {
                        for (i, cmd) in matching.iter().enumerate() {
                            let is_selected = i == app.command_palette_selected_index;
                            let text = format!("{}: {}", cmd.category, cmd.title);
                            ui.horizontal(|ui| {
                                let resp = ui.selectable_label(is_selected, text);
                                if resp.clicked() {
                                    execute_action = Some(cmd.action_id.to_string());
                                    close = true;
                                }
                                if !cmd.shortcut.is_empty() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(cmd.shortcut).weak().size(11.0),
                                            );
                                        },
                                    );
                                }
                            });
                        }
                    }
                });
        });

    if let Some(action) = execute_action {
        execute_command(app, &action);
    }

    if close {
        app.show_command_palette = false;
        app.command_palette_query.clear();
        app.command_palette_selected_index = 0;
    }
}
