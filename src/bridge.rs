use lunaris_ecs::prelude::{Entity, World};
use futures::channel::mpsc;
use lunaris_api::render::RawImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use lunaris_api::bridge::UiToWorldCommand; // Import from lunaris_api

// --- Type alias for Plugin IDs ---
pub type PluginId = usize;

// --- Messages from the World thread to the UI thread ---

/// A snapshot of all world state required for the UI to draw itself.
#[derive(Debug, Clone, Default)]
pub struct WorldToUiState {
    /// A map of entities to their most recently rendered image.
    /// The UI will be responsible for turning these `RawImage`s into displayable textures.
    pub latest_renders: HashMap<Entity, RawImage>,
    /// The current frame number of the playhead.
    pub playhead: u64,
    /// Plugin-specific UI states, keyed by their PluginId.
    pub plugin_ui_states: HashMap<PluginId, serde_json::Value>,
}

// --- The Bridge itself ---

/// A struct that holds the communication channels for the UI-World bridge.
pub struct Bridge {
    pub ui_to_world_sender: mpsc::Sender<UiToWorldCommand>,
    pub ui_to_world_receiver: mpsc::Receiver<UiToWorldCommand>,
    pub world_to_ui_sender: mpsc::Sender<WorldToUiState>,
    pub world_to_ui_receiver: mpsc::Receiver<WorldToUiState>,
}

impl Bridge {
    /// Creates a new bridge with a specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (ui_tx, ui_rx) = mpsc::channel(capacity);
        let (world_tx, world_rx) = mpsc::channel(capacity);
        Self {
            ui_to_world_sender: ui_tx,
            ui_to_world_receiver: ui_rx,
            world_to_ui_sender: world_tx,
            world_to_ui_receiver: world_rx,
        }
    }
}

/// A helper system to be run in the World thread.
/// It receives commands from the UI and applies them to the World.
pub fn apply_ui_commands_to_world(world: &mut World) {
    // Take the receiver out of the world resources to use it.
    if let Some(mut receiver) = world.get_resource_mut::<mpsc::Receiver<UiToWorldCommand>>() {
        // Process all available messages in the channel for this tick.
        while let Ok(Some(command)) = receiver.try_next() {
            match command {
                UiToWorldCommand::RequestRender { entity, frame } => {
                    println!("World received render request for entity {:?} at frame {}", entity, frame);
                    // Add the RenderRequest component to trigger the orchestrator.
                    world.entity_mut(entity).insert(crate::orchestrator::RenderRequest {
                        frame,
                        renderer_name: None, // Use default renderer
                    });
                }
                UiToWorldCommand::SetPlayhead(frame) => {
                    if let Some(mut playhead) = world.get_resource_mut::<lunaris_api::timeline::Playhead>() {
                        playhead.current = frame;
                    }
                }
                UiToWorldCommand::Quit => {
                    // This command is handled by the main loop, not the ECS system.
                }
            }
        }
    }
}
