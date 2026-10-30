use crate::id::ObjectId;

/// Trait implemented by any identifiable entity in the scene or engine.
pub trait Identifiable {
    fn id(&self) -> ObjectId;
}

/// Trait implemented by named entities (scene objects, materials, layers).
pub trait Named {
    fn name(&self) -> &str;
    fn set_name(&mut self, name: impl Into<String>);
}
