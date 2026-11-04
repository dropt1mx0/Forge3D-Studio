use std::sync::Arc;

use forge_core::id::ObjectId;
use forge_geometry::Mesh;
use forge_math::Transform;

use crate::object::SceneObject;
use crate::scene::Scene;

/// A reversible user action recorded in the history stack.
#[derive(Debug, Clone)]
pub enum Action {
    Transform {
        id: ObjectId,
        old_transform: Transform,
        new_transform: Transform,
    },
    AddObject {
        object: SceneObject,
    },
    DeleteObject {
        object: SceneObject,
    },
    ModifyMesh {
        id: ObjectId,
        old_mesh: Arc<Mesh>,
        new_mesh: Arc<Mesh>,
        name: String,
    },
    ApplyTransform {
        id: ObjectId,
        old_transform: Transform,
        old_mesh: Arc<Mesh>,
    },
    MultiTransform {
        transforms: Vec<(ObjectId, Transform, Transform)>, // (id, old_tf, new_tf)
    },
    MultiDelete {
        objects: Vec<SceneObject>,
    },
    ImportObjects {
        objects: Vec<SceneObject>,
    },
    Join {
        active_id: ObjectId,
        old_mesh: Arc<Mesh>,
        removed_objects: Vec<SceneObject>,
    },
    ParentChange {
        child_id: ObjectId,
        old_parent: Option<ObjectId>,
        new_parent: Option<ObjectId>,
        old_local_tf: Transform,
        new_local_tf: Transform,
    },
    SetOrigin {
        id: ObjectId,
        old_transform: Transform,
        old_mesh: Arc<Mesh>,
        new_transform: Transform,
        new_mesh: Arc<Mesh>,
    },
}

impl Action {
    pub fn description(&self) -> &str {
        match self {
            Self::Transform { .. } => "Transform Object",
            Self::MultiTransform { .. } => "Transform Multiple Objects",
            Self::AddObject { .. } => "Add Object",
            Self::DeleteObject { .. } => "Delete Object",
            Self::MultiDelete { .. } => "Delete Multiple Objects",
            Self::ImportObjects { .. } => "Import Wavefront OBJ",
            Self::ModifyMesh { name, .. } => name.as_str(),
            Self::ApplyTransform { .. } => "Apply Transform",
            Self::Join { .. } => "Join Objects",
            Self::ParentChange { .. } => "Change Parent",
            Self::SetOrigin { .. } => "Set Origin",
        }
    }

    /// Convenience constructor for `Action::ModifyMesh` accepting `Mesh` or `Arc<Mesh>`.
    pub fn modify_mesh(
        id: ObjectId,
        old_mesh: impl Into<Arc<Mesh>>,
        new_mesh: impl Into<Arc<Mesh>>,
        name: impl Into<String>,
    ) -> Self {
        Self::ModifyMesh {
            id,
            old_mesh: old_mesh.into(),
            new_mesh: new_mesh.into(),
            name: name.into(),
        }
    }

    /// Convenience constructor for `Action::ApplyTransform` accepting `Mesh` or `Arc<Mesh>`.
    pub fn apply_transform(
        id: ObjectId,
        old_transform: Transform,
        old_mesh: impl Into<Arc<Mesh>>,
    ) -> Self {
        Self::ApplyTransform {
            id,
            old_transform,
            old_mesh: old_mesh.into(),
        }
    }

    /// Convenience constructor for `Action::Join` accepting `Mesh` or `Arc<Mesh>`.
    pub fn join(
        active_id: ObjectId,
        old_mesh: impl Into<Arc<Mesh>>,
        removed_objects: Vec<SceneObject>,
    ) -> Self {
        Self::Join {
            active_id,
            old_mesh: old_mesh.into(),
            removed_objects,
        }
    }

    /// Convenience constructor for `Action::SetOrigin` accepting `Mesh` or `Arc<Mesh>`.
    pub fn set_origin(
        id: ObjectId,
        old_transform: Transform,
        old_mesh: impl Into<Arc<Mesh>>,
        new_transform: Transform,
        new_mesh: impl Into<Arc<Mesh>>,
    ) -> Self {
        Self::SetOrigin {
            id,
            old_transform,
            old_mesh: old_mesh.into(),
            new_transform,
            new_mesh: new_mesh.into(),
        }
    }
}

/// Centralized Undo/Redo command history stack with memory optimization and bounded limits.
#[derive(Debug, Clone)]
pub struct HistoryStack {
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
    pub max_depth: usize,
    pub max_memory_bytes: usize,
}

impl Default for HistoryStack {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 64,
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB default memory budget
        }
    }
}

impl HistoryStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self.enforce_limits();
        self
    }

    pub fn with_max_memory_bytes(mut self, max_memory_bytes: usize) -> Self {
        self.max_memory_bytes = max_memory_bytes;
        self.enforce_limits();
        self
    }

    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
        self.enforce_limits();
    }

    pub fn set_max_memory_bytes(&mut self, max_memory_bytes: usize) {
        self.max_memory_bytes = max_memory_bytes;
        self.enforce_limits();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn last_undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|a| a.description())
    }

    pub fn last_redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|a| a.description())
    }

    /// Pushes a new action onto the undo stack and clears redo history.
    ///
    /// Automatically deduplicates consecutive mesh modifications on the same object
    /// by sharing immutable `Arc<Mesh>` references and skipping no-op edits.
    pub fn push(&mut self, mut action: Action) {
        if let Action::ModifyMesh {
            id,
            old_mesh,
            new_mesh,
            ..
        } = &mut action
        {
            // Skip no-op modifications if mesh hasn't changed at all
            if Arc::ptr_eq(old_mesh, new_mesh) || *old_mesh == *new_mesh {
                return;
            }

            // Deduplicate old_mesh if it matches previous action's new_mesh
            if let Some(Action::ModifyMesh {
                id: prev_id,
                new_mesh: prev_new,
                ..
            }) = self.undo_stack.last()
            {
                if prev_id == id && (Arc::ptr_eq(old_mesh, prev_new) || *old_mesh == *prev_new) {
                    *old_mesh = Arc::clone(prev_new);
                }
            }
        }

        self.undo_stack.push(action);
        self.redo_stack.clear();
        self.enforce_limits();
    }

    /// Enforces `max_depth` and `max_memory_bytes` limits by evicting the oldest undo actions.
    pub fn enforce_limits(&mut self) {
        while self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }

        if self.max_memory_bytes > 0 {
            while self.undo_stack.len() > 1 && self.memory_usage_bytes() > self.max_memory_bytes {
                self.undo_stack.remove(0);
            }
        }
    }

    /// Calculates the total memory footprint in bytes of all actions in the history stack,
    /// correctly accounting for shared `Arc<Mesh>` allocations without double-counting.
    pub fn memory_usage_bytes(&self) -> usize {
        use std::collections::HashSet;

        let mut seen_meshes: HashSet<usize> = HashSet::new();
        let mut total = std::mem::size_of::<Self>();
        total += self.undo_stack.capacity() * std::mem::size_of::<Action>();
        total += self.redo_stack.capacity() * std::mem::size_of::<Action>();

        for action in self.undo_stack.iter().chain(self.redo_stack.iter()) {
            match action {
                Action::ModifyMesh {
                    old_mesh,
                    new_mesh,
                    name,
                    ..
                } => {
                    total += name.capacity();
                    if seen_meshes.insert(Arc::as_ptr(old_mesh) as usize) {
                        total += old_mesh.memory_size();
                    }
                    if seen_meshes.insert(Arc::as_ptr(new_mesh) as usize) {
                        total += new_mesh.memory_size();
                    }
                }
                Action::ApplyTransform { old_mesh, .. } => {
                    if seen_meshes.insert(Arc::as_ptr(old_mesh) as usize) {
                        total += old_mesh.memory_size();
                    }
                }
                Action::SetOrigin {
                    old_mesh, new_mesh, ..
                } => {
                    if seen_meshes.insert(Arc::as_ptr(old_mesh) as usize) {
                        total += old_mesh.memory_size();
                    }
                    if seen_meshes.insert(Arc::as_ptr(new_mesh) as usize) {
                        total += new_mesh.memory_size();
                    }
                }
                Action::Join {
                    old_mesh,
                    removed_objects,
                    ..
                } => {
                    if seen_meshes.insert(Arc::as_ptr(old_mesh) as usize) {
                        total += old_mesh.memory_size();
                    }
                    total += removed_objects.capacity() * std::mem::size_of::<SceneObject>();
                    for obj in removed_objects {
                        total += obj.mesh.memory_size();
                    }
                }
                Action::AddObject { object } | Action::DeleteObject { object } => {
                    total += std::mem::size_of::<SceneObject>() + object.mesh.memory_size();
                }
                Action::MultiDelete { objects } | Action::ImportObjects { objects } => {
                    total += objects.capacity() * std::mem::size_of::<SceneObject>();
                    for obj in objects {
                        total += obj.mesh.memory_size();
                    }
                }
                Action::MultiTransform { transforms } => {
                    total += transforms.capacity()
                        * std::mem::size_of::<(ObjectId, Transform, Transform)>();
                }
                Action::Transform { .. } | Action::ParentChange { .. } => {}
            }
        }

        total
    }

    /// Undoes the last action, restoring scene state.
    pub fn undo(&mut self, scene: &mut Scene) -> Option<String> {
        let action = self.undo_stack.pop()?;
        let desc = action.description().to_string();

        match &action {
            Action::Transform {
                id, old_transform, ..
            } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.local_transform = *old_transform;
                    scene.dirty = true;
                }
            }
            Action::MultiTransform { transforms } => {
                for (id, old_tf, _) in transforms {
                    if let Some(obj) = scene.get_object_mut(*id) {
                        obj.local_transform = *old_tf;
                    }
                }
                scene.dirty = true;
            }
            Action::AddObject { object } => {
                scene.remove_object(object.id);
            }
            Action::DeleteObject { object } => {
                scene.add_object(object.clone());
            }
            Action::MultiDelete { objects } => {
                for obj in objects {
                    scene.add_object(obj.clone());
                }
            }
            Action::ImportObjects { objects } => {
                for obj in objects {
                    scene.remove_object(obj.id);
                }
                scene.dirty = true;
            }
            Action::ModifyMesh { id, old_mesh, .. } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.set_mesh((**old_mesh).clone());
                    scene.dirty = true;
                }
            }
            Action::ApplyTransform {
                id,
                old_transform,
                old_mesh,
            } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.local_transform = *old_transform;
                    obj.set_mesh((**old_mesh).clone());
                    scene.dirty = true;
                }
            }
            Action::Join {
                active_id,
                old_mesh,
                removed_objects,
            } => {
                if let Some(obj) = scene.get_object_mut(*active_id) {
                    obj.set_mesh((**old_mesh).clone());
                }
                for ro in removed_objects {
                    scene.add_object(ro.clone());
                }
                scene.dirty = true;
            }
            Action::ParentChange {
                child_id,
                old_parent,
                old_local_tf,
                ..
            } => {
                if let Some(child) = scene.objects.get_mut(child_id) {
                    child.parent = *old_parent;
                    child.local_transform = *old_local_tf;
                }
                scene.rebuild_hierarchy();
            }
            Action::SetOrigin {
                id,
                old_transform,
                old_mesh,
                ..
            } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.local_transform = *old_transform;
                    obj.set_mesh((**old_mesh).clone());
                    scene.dirty = true;
                }
            }
        }

        self.redo_stack.push(action);
        Some(desc)
    }

    /// Redoes the previously undone action.
    pub fn redo(&mut self, scene: &mut Scene) -> Option<String> {
        let action = self.redo_stack.pop()?;
        let desc = action.description().to_string();

        match &action {
            Action::Transform {
                id, new_transform, ..
            } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.local_transform = *new_transform;
                    scene.dirty = true;
                }
            }
            Action::MultiTransform { transforms } => {
                for (id, _, new_tf) in transforms {
                    if let Some(obj) = scene.get_object_mut(*id) {
                        obj.local_transform = *new_tf;
                    }
                }
                scene.dirty = true;
            }
            Action::AddObject { object } => {
                scene.add_object(object.clone());
            }
            Action::DeleteObject { object } => {
                scene.remove_object(object.id);
            }
            Action::MultiDelete { objects } => {
                for obj in objects {
                    scene.remove_object(obj.id);
                }
            }
            Action::ImportObjects { objects } => {
                for obj in objects {
                    scene.add_object(obj.clone());
                }
                scene.dirty = true;
            }
            Action::ModifyMesh { id, new_mesh, .. } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.set_mesh((**new_mesh).clone());
                    scene.dirty = true;
                }
            }
            Action::ApplyTransform { id, .. } => {
                scene.apply_transform_to_object(*id);
            }
            Action::Join {
                active_id,
                removed_objects,
                ..
            } => {
                let remove_ids: Vec<ObjectId> = removed_objects.iter().map(|o| o.id).collect();
                let _ = scene.join_objects(*active_id, &remove_ids);
            }
            Action::ParentChange {
                child_id,
                new_parent,
                new_local_tf,
                ..
            } => {
                if let Some(child) = scene.objects.get_mut(child_id) {
                    child.parent = *new_parent;
                    child.local_transform = *new_local_tf;
                }
                scene.rebuild_hierarchy();
            }
            Action::SetOrigin {
                id,
                new_transform,
                new_mesh,
                ..
            } => {
                if let Some(obj) = scene.get_object_mut(*id) {
                    obj.local_transform = *new_transform;
                    obj.set_mesh((**new_mesh).clone());
                    scene.dirty = true;
                }
            }
        }

        self.undo_stack.push(action);
        self.enforce_limits();
        Some(desc)
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.undo_stack.shrink_to_fit();
        self.redo_stack.shrink_to_fit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::PrimitiveType;
    use forge_geometry::primitives::{generate_cube, generate_uv_sphere};
    use forge_math::Vec3;

    #[test]
    fn test_history_transform_undo_redo() {
        let mut scene = Scene::new("Test");
        let cube = SceneObject::new("Cube", PrimitiveType::Cube, generate_cube(1.0));
        let id = scene.add_object(cube);

        let mut history = HistoryStack::new();

        let old_tf = Transform::IDENTITY;
        let new_tf = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));

        // Apply change
        scene.get_object_mut(id).unwrap().local_transform = new_tf;
        history.push(Action::Transform {
            id,
            old_transform: old_tf,
            new_transform: new_tf,
        });

        assert_eq!(
            scene.get_object(id).unwrap().local_transform.translation.x,
            5.0
        );

        // Undo
        history.undo(&mut scene);
        assert_eq!(
            scene.get_object(id).unwrap().local_transform.translation.x,
            0.0
        );

        // Redo
        history.redo(&mut scene);
        assert_eq!(
            scene.get_object(id).unwrap().local_transform.translation.x,
            5.0
        );
    }

    #[test]
    fn test_history_import_objects_undo_redo() {
        let mut scene = Scene::new("Test");
        let obj1 = SceneObject::new("Imported1", PrimitiveType::Custom, generate_cube(1.0));
        let obj2 = SceneObject::new("Imported2", PrimitiveType::Custom, generate_cube(1.0));
        let id1 = obj1.id;
        let id2 = obj2.id;
        let imported_list = vec![obj1.clone(), obj2.clone()];

        scene.add_object(obj1);
        scene.add_object(obj2);

        let mut history = HistoryStack::new();
        history.push(Action::ImportObjects {
            objects: imported_list,
        });

        assert_eq!(scene.objects.len(), 2);

        // Single Undo removes both imported objects
        history.undo(&mut scene);
        assert_eq!(scene.objects.len(), 0);
        assert!(scene.get_object(id1).is_none());
        assert!(scene.get_object(id2).is_none());

        // Single Redo restores both objects
        history.redo(&mut scene);
        assert_eq!(scene.objects.len(), 2);
        assert!(scene.get_object(id1).is_some());
        assert!(scene.get_object(id2).is_some());
    }

    #[test]
    fn test_consecutive_modify_mesh_shares_arc() {
        let mut scene = Scene::new("Test");
        let m0 = generate_cube(1.0);
        let mut m1 = m0.clone();
        m1.vertices[0].position[0] += 1.0;
        let mut m2 = m1.clone();
        m2.vertices[1].position[1] += 2.0;

        let obj = SceneObject::new("Cube", PrimitiveType::Cube, m0.clone());
        let id = scene.add_object(obj);

        let mut history = HistoryStack::new();

        // Edit 1: M0 -> M1
        history.push(Action::modify_mesh(id, m0.clone(), m1.clone(), "Edit 1"));

        // Edit 2: M1 -> M2
        history.push(Action::modify_mesh(id, m1.clone(), m2.clone(), "Edit 2"));

        assert_eq!(history.undo_stack.len(), 2);

        // Verify Arc sharing: Action 1's new_mesh and Action 2's old_mesh MUST share the exact same Arc allocation
        let arc_m1_act1 = match &history.undo_stack[0] {
            Action::ModifyMesh { new_mesh, .. } => Arc::clone(new_mesh),
            _ => panic!("Expected ModifyMesh"),
        };
        let arc_m1_act2 = match &history.undo_stack[1] {
            Action::ModifyMesh { old_mesh, .. } => Arc::clone(old_mesh),
            _ => panic!("Expected ModifyMesh"),
        };

        assert!(
            Arc::ptr_eq(&arc_m1_act1, &arc_m1_act2),
            "Consecutive modifications on the same object must share the intermediate Arc<Mesh>"
        );
        assert_eq!(
            Arc::strong_count(&arc_m1_act1),
            4, // arc_m1_act1 + arc_m1_act2 local clones + act1.new_mesh + act2.old_mesh
            "Arc strong count must reflect shared ownership"
        );
    }

    #[test]
    fn test_history_multiple_undo_redo_fidelity() {
        let mut scene = Scene::new("Test");
        let m0 = generate_cube(1.0);
        let obj = SceneObject::new("Cube", PrimitiveType::Cube, m0.clone());
        let id = scene.add_object(obj);

        let mut history = HistoryStack::new();
        let mut meshes = vec![m0];

        // Perform 5 successive modifications
        for i in 1..=5 {
            let mut next_mesh = meshes.last().unwrap().clone();
            next_mesh.vertices[0].position[0] += i as f32;
            scene
                .get_object_mut(id)
                .unwrap()
                .set_mesh(next_mesh.clone());
            history.push(Action::modify_mesh(
                id,
                meshes.last().unwrap().clone(),
                next_mesh.clone(),
                format!("Edit {i}"),
            ));
            meshes.push(next_mesh);
        }

        assert_eq!(history.undo_stack.len(), 5);

        // Step-by-step Undo
        for i in (0..5).rev() {
            history.undo(&mut scene);
            assert_eq!(
                scene.get_object(id).unwrap().mesh.vertices[0].position,
                meshes[i].vertices[0].position,
                "Undo step must precisely restore mesh state {i}"
            );
        }
        assert_eq!(history.undo_stack.len(), 0);
        assert_eq!(history.redo_stack.len(), 5);

        // Step-by-step Redo
        for i in 1..=5 {
            history.redo(&mut scene);
            assert_eq!(
                scene.get_object(id).unwrap().mesh.vertices[0].position,
                meshes[i].vertices[0].position,
                "Redo step must precisely restore mesh state {i}"
            );
        }
        assert_eq!(history.undo_stack.len(), 5);
        assert_eq!(history.redo_stack.len(), 0);
    }

    #[test]
    fn test_history_clear_releases_memory() {
        let mut history = HistoryStack::new();
        let sphere = generate_uv_sphere(1.0, 32, 16);
        let id = ObjectId::new();

        for i in 0..10 {
            let mut next = sphere.clone();
            next.vertices[0].position[1] += i as f32;
            history.push(Action::modify_mesh(id, sphere.clone(), next, "Step"));
        }

        let memory_before = history.memory_usage_bytes();
        assert!(
            memory_before > 10_000,
            "History must record non-zero memory for large meshes"
        );

        history.clear();
        let memory_after = history.memory_usage_bytes();

        assert_eq!(history.undo_stack.len(), 0);
        assert_eq!(history.redo_stack.len(), 0);
        assert!(
            memory_after < 500,
            "Clearing history must release all mesh memory (expected < 500 B, got {memory_after} B)"
        );
    }

    #[test]
    fn test_mesh_immutability_after_history_push() {
        let mut scene = Scene::new("Test");
        let m0 = generate_cube(1.0);
        let mut m1 = m0.clone();
        m1.vertices[0].position[0] = 5.0;

        let obj = SceneObject::new("Cube", PrimitiveType::Cube, m1.clone());
        let id = scene.add_object(obj);

        let mut history = HistoryStack::new();
        history.push(Action::modify_mesh(id, m0.clone(), m1.clone(), "Edit"));

        // Subsequent in-place modification of scene object mesh
        let obj_mut = scene.get_object_mut(id).unwrap();
        obj_mut.mesh_mut().vertices[0].position[0] = 999.0;
        assert_eq!(obj_mut.mesh.vertices[0].position[0], 999.0);

        // Historical action snapshots must remain untouched
        if let Action::ModifyMesh {
            old_mesh, new_mesh, ..
        } = &history.undo_stack[0]
        {
            assert_eq!(old_mesh.vertices[0].position[0], m0.vertices[0].position[0]);
            assert_eq!(new_mesh.vertices[0].position[0], 5.0);
        } else {
            panic!("Expected ModifyMesh");
        }

        // Undo restores pure snapshot
        history.undo(&mut scene);
        assert_eq!(
            scene.get_object(id).unwrap().mesh.vertices[0].position[0],
            m0.vertices[0].position[0]
        );
    }

    #[test]
    fn test_large_mesh_history_memory_efficiency() {
        let sphere = generate_uv_sphere(1.0, 32, 16);
        let id = ObjectId::new();
        let single_mesh_bytes = sphere.clone().memory_size();

        let mut history = HistoryStack::new();
        let mut current = sphere.clone();

        // Perform 10 consecutive edits
        for i in 1..=10 {
            let mut next = current.clone();
            next.vertices[0].position[2] += i as f32 * 0.1;
            history.push(Action::modify_mesh(id, current, next.clone(), "Edit"));
            current = next;
        }

        // Without Arc sharing, 10 actions would store 10 * (old + new) = 20 Mesh instances.
        // With Arc sharing, exactly 11 distinct Mesh instances exist (M0..M10).
        let total_memory = history.memory_usage_bytes();
        let unshared_estimated = 20 * single_mesh_bytes;

        // Verify shared memory cuts memory consumption by ~40-50%
        assert!(
            total_memory < (unshared_estimated * 65 / 100),
            "Arc sharing should cut memory by ~40-50%, measured total {total_memory} vs unshared {unshared_estimated}"
        );

        let mesh_memory_only = total_memory
            - (history.undo_stack.capacity() * std::mem::size_of::<Action>()
                + std::mem::size_of::<HistoryStack>());
        let expected_meshes = 11 * single_mesh_bytes;
        assert!(
            (mesh_memory_only as i64 - expected_meshes as i64).abs() < 500,
            "Shared history mesh memory should match 11 unique meshes within tolerance"
        );
    }

    #[test]
    fn test_history_max_depth_eviction() {
        let mut history = HistoryStack::new().with_max_depth(3);
        let id = ObjectId::new();

        for i in 0..5 {
            history.push(Action::Transform {
                id,
                old_transform: Transform::from_translation(Vec3::new(i as f32, 0.0, 0.0)),
                new_transform: Transform::from_translation(Vec3::new(i as f32 + 1.0, 0.0, 0.0)),
            });
        }

        assert_eq!(history.undo_stack.len(), 3);
        // FIFO order: oldest actions 0 and 1 were evicted, remaining are 2, 3, 4
        if let Action::Transform { old_transform, .. } = &history.undo_stack[0] {
            assert_eq!(old_transform.translation.x, 2.0);
        } else {
            panic!("Expected Transform");
        }
    }

    #[test]
    fn test_history_memory_budget_eviction() {
        let sphere = generate_uv_sphere(1.0, 32, 16);
        let mesh_bytes = sphere.memory_size();
        let id = ObjectId::new();

        // Set budget sufficient for ~3 unique meshes
        let budget = mesh_bytes * 3 + 2000;
        let mut history = HistoryStack::new().with_max_memory_bytes(budget);
        let mut current = sphere;

        for i in 1..=6 {
            let mut next = current.clone();
            next.vertices[0].position[0] += i as f32;
            history.push(Action::modify_mesh(id, current, next.clone(), "Edit"));
            current = next;
        }

        // Memory must remain within budget
        assert!(
            history.memory_usage_bytes() <= budget,
            "Memory usage {} must not exceed budget {}",
            history.memory_usage_bytes(),
            budget
        );
        // At least 1 action must always be preserved
        assert!(
            !history.undo_stack.is_empty(),
            "History must retain at least the most recent action"
        );
    }

    #[test]
    fn test_history_skips_noop_modify_mesh() {
        let mut history = HistoryStack::new();
        let cube = generate_cube(1.0);
        let id = ObjectId::new();

        // Pushing identical old_mesh and new_mesh
        history.push(Action::modify_mesh(id, cube.clone(), cube.clone(), "No-op"));

        assert_eq!(
            history.undo_stack.len(),
            0,
            "No-op ModifyMesh must not be recorded in history stack"
        );
    }
}
