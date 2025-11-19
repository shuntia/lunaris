use lunaris_ecs::{bevy_ecs, prelude::*};
use lunaris_api::{
    export_plugin,
    plugin::{Gui, Plugin, PluginContext, PluginReport, UiState},
    timeline::TimelineSpan,
    util::error::Result,
};
use std::any::Any;

// --- Plugin-specific UI State ---

/// The UI state for the TimelineTestPlugin.
#[derive(Debug, Clone)]
pub struct TimelineTestUiState {
    pub entity_id: Option<Entity>,
    pub last_rendered_frame: Option<u64>,
}

// UiState is automatically implemented via blanket impl

// --- TimelineTestPlugin ---

#[derive(Default)]
pub struct TimelineTestPlugin {
    // This plugin will store the ID of the entity it manages in the World as a resource.
    // This allows the plugin to be stateless itself, which is good for the shared instance model.
}

/// Resource to hold the entity ID managed by TimelineTestPlugin.
#[derive(Resource)]
pub struct TimelineTestEntity(pub Entity);

impl Plugin for TimelineTestPlugin {
    fn new() -> Self {
        Self::default()
    }

    fn name(&self) -> &'static str {
        "Timeline Test"
    }

    fn init(&self, _ctx: PluginContext<'_>) -> Result<()> {
        // TODO: This plugin needs refactoring to use public API only
        Ok(())
    }

    fn add_schedule(&self, _schedule: &mut lunaris_api::plugin::Schedule) -> Result<()> {
        Ok(())
    }

    fn update_world(&mut self, _ctx: PluginContext<'_>) -> Result<()> {
        // TODO: Implement proper world update
        Ok(())
    }

    fn report(&self, _ctx: PluginContext<'_>) -> PluginReport { PluginReport::Operational }
    fn shutdown(&mut self, _ctx: PluginContext<'_>) {}
    fn reset(&mut self, _ctx: PluginContext<'_>) {}
}

impl Gui for TimelineTestPlugin {
    fn ui(&self, ui: &mut egui::Ui, _ctx: PluginContext<'_>) {
        ui.heading("Timeline Test Plugin");
        ui.label("(Placeholder - needs refactoring)");
    }
}

export_plugin!(TimelineTestPlugin, id: "lunaris.core.timeline_test", [Gui]);
