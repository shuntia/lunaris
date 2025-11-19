use lunaris_ecs::prelude::*;
use eframe::{
    App,
    egui::{CentralPanel, MenuBar, TopBottomPanel, TextureHandle},
};
use egui_tiles::{Behavior, Tiles, Tree};
use futures::channel::mpsc;
use lunaris_api::plugin::{GuiRegistration, PluginContext, RendererRegistration, UiState};
use slab::Slab;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use crate::{
    bridge::{self, Bridge, WorldToUiState},
    orchestrator::{self, AvailableRenderers, Orchestrator},
    plugin::{CorePluginNode, GuiPluginNode, PluginNode},
};
use lunaris_api::bridge::UiToWorldCommand;

pub struct LunarisApp {
    /// Handle to the dedicated world thread.
    world_thread: Option<JoinHandle<()>>,
    /// Sender to send commands to the world thread.
    command_sender: mpsc::Sender<UiToWorldCommand>,
    /// Receiver for state updates from the world thread.
    state_receiver: mpsc::Receiver<WorldToUiState>,
    /// The most recent state received from the world.
    latest_world_state: WorldToUiState,
    /// UI-side cache for turning RawImages into displayable textures.
    texture_handles: HashMap<Entity, TextureHandle>,

    // UI-specific state, managed only on the UI thread
    plugins: Slab<Box<dyn PluginNode>>,
    tree: Tree<lunaris_api::plugin::PluginId>,
    gui_plugins: HashMap<lunaris_api::plugin::PluginId, Box<dyn lunaris_api::plugin::Gui<PluginUiState = Box<dyn UiState>>>>,
}

impl Default for LunarisApp {
    fn default() -> Self {
        let mut bridge = Bridge::new(8);

        // The UI thread keeps the sending end for commands and the receiving end for state.
        let command_sender = bridge.ui_to_world_sender;
        let state_receiver = bridge.world_to_ui_receiver;

        // --- Initialize UI-specific state first ---
        let mut tiles: Tiles<lunaris_api::plugin::PluginId> = Tiles::default();
        let mut plugins: Slab<Box<dyn PluginNode>> = Slab::new();
        let mut gui_plugins: HashMap<lunaris_api::plugin::PluginId, Box<dyn lunaris_api::plugin::Gui<PluginUiState = Box<dyn UiState>>>> = HashMap::new();

        // --- Spawn the dedicated World thread ---
        let world_thread = thread::spawn(move || {
            let mut world = World::new();
            let mut schedule = Schedule::default();

            // --- Initialize World Resources ---
            world.insert_resource(Orchestrator::default());
            world.insert_resource(lunaris_api::timeline::Playhead { current: 0 });
            world.insert_resource(bridge.ui_to_world_receiver); // Add channel receiver
            world.insert_resource(bridge.world_to_ui_sender); // Add channel sender

            // Discover and initialize renderers
            let mut available_renderers = AvailableRenderers::default();
            for reg in inventory::iter::<RendererRegistration> {
                available_renderers.renderers.insert(reg.name, (reg.build)());
            }
            if let Some(name) = available_renderers.renderers.keys().next() {
                available_renderers.active_renderer = Some(name);
            }
            world.insert_resource(available_renderers);

            // --- Plugin Initialization (World-side) ---
            let mut plugin_ui_state_senders: HashMap<lunaris_api::plugin::PluginId, mpsc::Sender<Box<dyn UiState>>> = HashMap::new();
            let mut plugin_ui_state_receivers: HashMap<lunaris_api::plugin::PluginId, mpsc::Receiver<Box<dyn UiState>>> = HashMap::new();

            let mut world_plugins: Slab<Box<dyn lunaris_api::plugin::Plugin>> = Slab::new();
            let mut gui_plugin_ids: Vec<lunaris_api::plugin::PluginId> = Vec::new();

            for reg in inventory::iter::<GuiRegistration> {
                let (tx, rx) = mpsc::channel(1);
                let plugin_id = world_plugins.insert((reg.build)());
                plugin_ui_state_senders.insert(plugin_id, tx);
                plugin_ui_state_receivers.insert(plugin_id, rx);
                gui_plugin_ids.push(plugin_id);
            }
            for reg in inventory::iter::<lunaris_api::plugin::PluginRegistration> {
                world_plugins.insert((reg.build)());
            }

            // Initialize all plugins
            for (plugin_id, p) in world_plugins.iter() {
                let ctx = PluginContext {
                    world: &mut world,
                    orch: world.get_resource::<Orchestrator>().unwrap() as &dyn lunaris_api::request::DynOrchestrator,
                    ui_state_sender: plugin_ui_state_senders.get(&plugin_id).unwrap().clone(),
                    ui_to_world_sender: bridge.ui_to_world_sender.clone(),
                };
                let _ = p.init(ctx);
            }

            // --- Add Systems to the Schedule ---
            schedule.add_system(bridge::apply_ui_commands_to_world);
            schedule.add_system(orchestrator::render_orchestrator_system);
            schedule.add_system(move |world: &mut World| {
                // System to collect plugin UI states and send to UI thread
                let mut world_to_ui_sender = world.get_resource_mut::<mpsc::Sender<WorldToUiState>>().unwrap();
                let mut current_world_to_ui_state = WorldToUiState::default();

                // Collect latest renders
                let mut query = world.query::<(Entity, &orchestrator::RenderOutput)>();
                for (entity, output) in query.iter(&world) {
                    current_world_to_ui_state.latest_renders.insert(entity, output.image.clone());
                }
                // Collect playhead
                if let Some(playhead) = world.get_resource::<lunaris_api::timeline::Playhead>() {
                    current_world_to_ui_state.playhead = playhead.current;
                }

                // Collect plugin-specific UI states
                for (plugin_id, mut receiver) in plugin_ui_state_receivers.iter_mut() {
                    if let Ok(Some(plugin_state)) = receiver.try_next() {
                        // Serialize the plugin-specific state into a serde_json::Value
                        // This requires the PluginUiState to be serializable.
                        // For now, we'll just store a placeholder.
                        // TODO: Proper serialization of Box<dyn UiState> to serde_json::Value
                        current_world_to_ui_state.plugin_ui_states.insert(*plugin_id, serde_json::Value::Null);
                    }
                }

                if world_to_ui_sender.try_send(current_world_to_ui_state).is_err() {
                    // UI thread has likely closed, so we can exit.
                    println!("UI channel closed, world thread exiting.");
                }
            });

            // --- Main World Loop ---
            loop {
                // Check for a quit command first.
                if let Ok(Some(UiToWorldCommand::Quit)) = world.get_resource_mut::<mpsc::Receiver<UiToWorldCommand>>().unwrap().try_next() {
                    println!("World thread received quit command.");
                    break;
                }

                // Run all systems in the schedule!
                schedule.run(&mut world);

                thread::sleep(std::time::Duration::from_millis(16));
            }
        });

        // --- UI-side plugin setup ---
        for reg in inventory::iter::<GuiRegistration> {
            let plugin_id = plugins.insert(Box::new(CorePluginNode::new((reg.build)())));
            gui_plugins.insert(plugin_id, (reg.build)());
        }
        let root = tiles.insert_tab_tile(plugins.iter().map(|(id, _)| id).collect());

        Self {
            world_thread: Some(world_thread),
            command_sender,
            state_receiver,
            latest_world_state: WorldToUiState::default(),
            texture_handles: HashMap::new(),
            plugins,
            tree: Tree::new("main_tree", root, tiles),
            gui_plugins,
        }
    }
}

struct AppBehavior<'a> {
    plugins: &'a mut Slab<Box<dyn PluginNode>>,
    gui_plugins: &'a HashMap<lunaris_api::plugin::PluginId, Box<dyn lunaris_api::plugin::Gui<PluginUiState = Box<dyn UiState>>>>,
    world_state: &'a WorldToUiState,
    texture_handles: &'a HashMap<Entity, TextureHandle>,
    command_sender: &'a mpsc::Sender<UiToWorldCommand>,
}

impl<'a> Behavior<lunaris_api::plugin::PluginId> for AppBehavior<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut eframe::egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane_id: &mut lunaris_api::plugin::PluginId,
    ) -> egui_tiles::UiResponse {
        if let Some(gui_plugin) = self.gui_plugins.get(pane_id) {
            // Create a dummy PluginContext for the UI side. Only the senders are real.
            let dummy_world = &mut World::new();
            let dummy_orch = &Orchestrator::default() as &dyn lunaris_api::request::DynOrchestrator;
            let (dummy_tx, _dummy_rx) = mpsc::channel(1);
            let ctx = PluginContext {
                world: dummy_world,
                orch: dummy_orch,
                ui_state_sender: dummy_tx.clone(), // Dummy sender
                ui_to_world_sender: self.command_sender.clone(), // Real sender
            };

            // Attempt to get the plugin's UI state from the latest_world_state
            let plugin_ui_state: Box<dyn UiState> = self.world_state.plugin_ui_states.get(pane_id)
                .and_then(|val| {
                    // This is where we'd deserialize the specific PluginUiState.
                    // For now, we'll just return a default empty state.
                    Some(Box::new(()) as Box<dyn UiState>)
                })
                .unwrap_or_else(|| Box::new(()) as Box<dyn UiState>); // Default empty state if not found

            gui_plugin.ui(ui, ctx, &*plugin_ui_state, self.texture_handles);
        }
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane_id: &mut lunaris_api::plugin::PluginId) -> eframe::egui::WidgetText {
        self.gui_plugins.get(pane_id).map_or("<missing>".into(), |p| p.name().into())
    }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<lunaris_api::plugin::PluginId>,
        ui: &mut eframe::egui::Ui,
        _tab_container_id: egui_tiles::TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        // This functionality would need to be re-thought. Adding a tab would
        // now involve sending a command to the world thread.
    }
}

impl App for LunarisApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // --- Receive latest state from world thread ---
        while let Ok(Some(new_state)) = self.state_receiver.try_next() {
            self.latest_world_state = new_state;
        }

        // --- Update UI-side textures ---
        for (entity, raw_image) in &self.latest_world_state.latest_renders {
            if !self.texture_handles.contains_key(entity) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [raw_image.width() as usize, raw_image.height() as usize],
                    raw_image.as_bytes(),
                );
                let handle = ctx.load_texture(format!("render_{:?}", entity), color_image, Default::default());
                self.texture_handles.insert(*entity, handle);
            }
        }

        // --- Main UI drawing ---
        let mut behavior = AppBehavior {
            plugins: &mut self.plugins,
            gui_plugins: &self.gui_plugins,
            world_state: &self.latest_world_state,
            texture_handles: &self.texture_handles,
            command_sender: &self.command_sender,
        };

        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        self.command_sender.try_send(UiToWorldCommand::Quit).ok();
                    }
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| self.tree.ui(&mut behavior, ui));

        if self.command_sender.is_closed() {
            // If the world thread has panicked and closed the channel, close the UI.
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Ensure the world thread is shut down cleanly when the app exits.
        if let Some(thread) = self.world_thread.take() {
            self.command_sender.try_send(UiToWorldCommand::Quit).ok();
            thread.join().expect("World thread panicked!");
        }
    }
}
