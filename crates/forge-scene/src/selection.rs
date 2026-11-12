use std::collections::HashSet;

use forge_core::id::ObjectId;

/// Tracks selection state in the editor supporting single and multi-selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    pub active_id: Option<ObjectId>,
    pub selected_ids: HashSet<ObjectId>,
    pub hovered_id: Option<ObjectId>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects an object, clearing previous selection.
    pub fn select(&mut self, id: ObjectId) {
        self.selected_ids.clear();
        self.selected_ids.insert(id);
        self.active_id = Some(id);
    }

    /// Adds an object to the current selection (Shift-select).
    pub fn add_to_selection(&mut self, id: ObjectId) {
        self.selected_ids.insert(id);
        self.active_id = Some(id);
    }

    /// Removes an object from the current selection.
    pub fn remove_from_selection(&mut self, id: ObjectId) {
        self.selected_ids.remove(&id);
        if self.active_id == Some(id) {
            self.active_id = self.selected_ids.iter().next().copied();
        }
    }

    /// Toggles selection of an object.
    pub fn toggle_selection(&mut self, id: ObjectId) {
        if self.selected_ids.contains(&id) {
            self.selected_ids.remove(&id);
            if self.active_id == Some(id) {
                self.active_id = self.selected_ids.iter().next().copied();
            }
        } else {
            self.selected_ids.insert(id);
            self.active_id = Some(id);
        }
    }

    /// Clears all selection.
    pub fn deselect(&mut self) {
        self.selected_ids.clear();
        self.active_id = None;
    }

    /// Checks if a specific object is selected.
    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selected_ids.contains(&id) || self.active_id == Some(id)
    }

    /// Checks if selection is empty.
    pub fn is_empty(&self) -> bool {
        self.selected_ids.is_empty() && self.active_id.is_none()
    }

    /// Sets the hovered object (under cursor).
    pub fn set_hovered(&mut self, id: Option<ObjectId>) {
        self.hovered_id = id;
    }

    /// Checks if a specific object is hovered.
    pub fn is_hovered(&self, id: ObjectId) -> bool {
        self.hovered_id == Some(id)
    }
}
