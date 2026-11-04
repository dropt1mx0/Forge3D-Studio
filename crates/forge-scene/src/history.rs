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
        old_mesh: Mesh,
        new_mesh: Mesh,
        name: String,
    },
    ApplyTransform {
        id: ObjectId,
        old_transform: Transform,
        old_mesh: Mesh,
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
        old_mesh: Mesh,
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
        old_mesh: Mesh,
        new_transform: Transform,
        new_mesh: Mesh,
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
}

/// Centralized Undo/Redo command history stack.
#[derive(Debug, Clone)]
pub struct HistoryStack {
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
    pub max_depth: usize,
}

impl Default for HistoryStack {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 64,
        }
    }
}

impl HistoryStack {
    pub fn new() -> Self {
        Self::default()
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
    pub fn push(&mut self, action: Action) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear();
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
                    obj.set_mesh(old_mesh.clone());
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
                    obj.set_mesh(old_mesh.clone());
                    scene.dirty = true;
                }
            }
            Action::Join {
                active_id,
                old_mesh,
                removed_objects,
            } => {
                if let Some(obj) = scene.get_object_mut(*active_id) {
                    obj.set_mesh(old_mesh.clone());
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
                    obj.set_mesh(old_mesh.clone());
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
                    obj.set_mesh(new_mesh.clone());
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
                    obj.set_mesh(new_mesh.clone());
                    scene.dirty = true;
                }
            }
        }

        self.undo_stack.push(action);
        Some(desc)
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::PrimitiveType;
    use forge_geometry::primitives::generate_cube;
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
}
