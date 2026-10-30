use forge_core::id::ObjectId;

/// Tracks selection state in the editor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    pub active_id: Option<ObjectId>,
    pub hovered_id: Option<ObjectId>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects an object by ID.
    pub fn select(&mut self, id: ObjectId) {
        self.active_id = Some(id);
    }

    /// Deselects the current object.
    pub fn deselect(&mut self) {
        self.active_id = None;
    }

    /// Checks if a specific object is selected.
    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.active_id == Some(id)
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
