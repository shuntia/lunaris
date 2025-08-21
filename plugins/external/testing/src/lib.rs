use lunaris_api::plugin::Plugin;

pub struct TestPlugin {}

impl Plugin for TestPlugin {
    fn name(&self) -> &'static str {
        "test plugin 1"
    }
    fn ui(&mut self, ctx: lunaris_api::plugin::PluginContext, ui: lunaris_api::egui::Ui) {}
    fn reset(&mut self, ctx: lunaris_api::plugin::PluginContext) {}
    fn report(&self, ctx: lunaris_api::plugin::PluginContext) -> lunaris_api::plugin::PluginReport {
        lunaris_api::plugin::PluginReport::Operational
    }
    fn shutdown(self, ctx: lunaris_api::plugin::PluginContext) {}
    fn update_world(&mut self, ctx: lunaris_api::plugin::PluginContext) {}
    fn register_menu(&self, menu_bar: &mut lunaris_api::egui::MenuBar) {}
    fn init(&self, ctx: lunaris_api::plugin::PluginContext) {}
}
