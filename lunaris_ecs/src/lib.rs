//! Lunaris ECS abstraction layer
//!
//! This module provides a minimal abstraction over the underlying ECS implementation.
//! Currently backed by bevy_ecs, but designed to be swappable in the future.

// Re-export bevy_ecs for derive macros to work
// The derive macros need to find bevy_ecs in scope
#[doc(hidden)]
pub use bevy_ecs;

// Core types that we expose
pub use bevy_ecs::entity::Entity;
pub use bevy_ecs::world::World;

// Component and Resource traits
pub use bevy_ecs::component::Component;
pub use bevy_ecs::resource::Resource;

// System types
pub use bevy_ecs::system::{BoxedSystem, Commands, Query, Res, ResMut, System};

// Event handling
pub use bevy_ecs::event::Event;

// Query filters
pub use bevy_ecs::query::{With, Without};

// Schedule for organizing systems
pub use bevy_ecs::schedule::Schedule;

// Prelude module for convenience imports
pub mod prelude {
    pub use super::{
        Commands, Component, Entity, Event, Query, Res, ResMut, Resource, Schedule, With, Without,
        World,
    };
}
