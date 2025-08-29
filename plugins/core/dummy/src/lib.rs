use lunaris_api::plugin::Plugin;

pub struct Dummy{}

impl Plugin for Dummy{
    fn new()->Self where Self:Sized{
        Dummy{}
    }
    fn name(&self) -> &'static str {
        "dummy"
    }
    fn reset(&mut self, ctx: lunaris_api::plugin::PluginContext) {}
    fn report(&self, ctx: lunaris_api::plugin::PluginContext) -> lunaris_api::plugin::PluginReport {
        lunaris_api::plugin::PluginReport::Operational
    }
    fn shutdown(self, ctx: lunaris_api::plugin::PluginContext) {}
    fn update_world(&mut self, ctx: lunaris_api::plugin::PluginContext) {}
    fn register_menu(&self, menu_bar: &mut lunaris_api::egui::MenuBar) {}
    fn init(&self, ctx: lunaris_api::plugin::PluginContext) {
    }
}

