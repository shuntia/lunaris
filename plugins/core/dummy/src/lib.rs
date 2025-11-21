use lunaris_api::{export_plugin, plugin::Plugin, util::error::Result};
extern crate lunaris_api;

export_plugin!(Dummy, id: "lunaris.core.dummy", name: "dummy");

pub struct Dummy {}

impl Plugin for Dummy {
    fn new() -> Self
    where
        Self: Sized,
    {
        Dummy {}
    }
    fn init(&self, _ctx: lunaris_api::plugin::PluginContext) -> Result {
        Ok(())
    }
    fn add_schedule(&self, _schedule: &mut lunaris_api::plugin::Schedule) -> Result {
        Ok(())
    }
    fn update_world(&mut self, _ctx: lunaris_api::plugin::PluginContext) -> Result {
        Ok(())
    }
    fn report(
        &self,
        _ctx: lunaris_api::plugin::PluginContext,
    ) -> lunaris_api::plugin::PluginReport {
        lunaris_api::plugin::PluginReport::Operational
    }
    fn shutdown(&mut self, _ctx: lunaris_api::plugin::PluginContext) {}
    fn reset(&mut self, _ctx: lunaris_api::plugin::PluginContext) {}
    fn register_menu(&self, _menu_bar: &mut lunaris_api::egui::MenuBar) {}
}
