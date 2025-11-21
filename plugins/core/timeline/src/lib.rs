use lunaris_api::util::error::Result;
use lunaris_api::{
    consts::tps,
    egui, export_plugin,
    plugin::{Gui, Plugin, PluginContext, PluginReport},
};
use lunaris_ecs::prelude::*;
use std::collections::HashSet;

mod components;
use components::TimelineElement;

export_plugin!(Timeline, id: "lunaris.core.timeline", name: "Timeline", [Gui]);

pub struct Timeline {
    tick_freq: u64,
}

#[derive(Resource, Clone)]
pub struct TimelineUiState {
    ticks_per_px: f64,
    scroll_x_ticks: f64,
    scroll_y_px: f32,
    track_height: f32,
    track_gap: f32,
    playhead_tick: u64,
    selection: HashSet<Entity>,
}

impl Default for TimelineUiState {
    fn default() -> Self {
        Self {
            // ~100 px per second to start
            ticks_per_px: tps() as f64 / 100.0,
            scroll_x_ticks: 0.0,
            scroll_y_px: 0.0,
            track_height: 28.0,
            track_gap: 6.0,
            playhead_tick: 0,
            selection: HashSet::new(),
        }
    }
}

impl Plugin for Timeline {
    fn init(&self, ctx: PluginContext<'_>) -> Result {
        ctx.world
            .insert_resource(lunaris_api::plugin::UiContext::new_clonable(
                TimelineUiState::default(),
            ));
        Ok(())
    }

    fn add_schedule(&self, _schedule: &mut lunaris_ecs::Schedule) -> Result {
        Ok(())
    }

    fn reset(&mut self, _ctx: PluginContext<'_>) {}

    fn report(&self, _ctx: PluginContext<'_>) -> PluginReport {
        PluginReport::Operational
    }

    fn shutdown(&mut self, _ctx: PluginContext<'_>) {}

    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result {
        if !ctx.world.contains_resource::<TimelineUiState>() {
            ctx.world.insert_resource(TimelineUiState::default());
        }
        Ok(())
    }

    fn register_menu(&self, _menu_bar: &mut lunaris_api::egui::MenuBar) {}

    fn new() -> Self
    where
        Self: Sized,
    {
        Self { tick_freq: tps() }
    }
}

impl Gui for Timeline {
    fn ui(&self, ui: &mut egui::Ui, ctx: PluginContext<'_>) {
        // Get UI state from World resource and clone it
        let mut st = {
            let ui_ctx = ctx.world.resource::<lunaris_api::plugin::UiContext<
                lunaris_api::plugin::ArcSwapStorage<TimelineUiState>,
            >>();
            ui_ctx.read().clone()
        };

        // Layout constants
        const TOP_H: f32 = 22.0;
        const LEFT_W: f32 = 80.0;
        const BOTTOM_H: f32 = 18.0;

        let avail = ui.available_size();
        let (outer, resp_outer) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());

        // Sub-rects
        let top_ruler = egui::Rect::from_min_max(
            egui::pos2(outer.left() + LEFT_W, outer.top()),
            egui::pos2(outer.right(), outer.top() + TOP_H),
        );
        let left_gutter = egui::Rect::from_min_max(
            egui::pos2(outer.left(), outer.top() + TOP_H),
            egui::pos2(outer.left() + LEFT_W, outer.bottom() - BOTTOM_H),
        );
        let canvas = egui::Rect::from_min_max(
            egui::pos2(outer.left() + LEFT_W, outer.top() + TOP_H),
            egui::pos2(outer.right(), outer.bottom() - BOTTOM_H),
        );
        let bottom = egui::Rect::from_min_max(
            egui::pos2(outer.left() + LEFT_W, outer.bottom() - BOTTOM_H),
            egui::pos2(outer.right(), outer.bottom()),
        );

        // Backgrounds
        ui.painter()
            .rect_filled(outer, 0.0, ui.visuals().extreme_bg_color);
        ui.painter()
            .rect_filled(top_ruler, 0.0, ui.visuals().faint_bg_color);
        ui.painter()
            .rect_filled(left_gutter, 0.0, ui.visuals().faint_bg_color);
        ui.painter()
            .rect_filled(bottom, 0.0, ui.visuals().faint_bg_color);

        // Input: zoom (Ctrl+wheel) and pan (MMB drag); wheel vertical scroll, Shift+wheel horizontal
        let (scroll_delta, hover_pos, mods) =
            ui.input(|i| (i.smooth_scroll_delta, i.pointer.hover_pos(), i.modifiers));
        if mods.ctrl && scroll_delta.y != 0.0 {
            if let Some(m) = hover_pos {
                let before = st.scroll_x_ticks + ((m.x - canvas.left()) as f64 * st.ticks_per_px);
                st.ticks_per_px =
                    (st.ticks_per_px * (1.0 - (scroll_delta.y as f64) * 0.001)).max(1.0);
                let after = st.scroll_x_ticks + ((m.x - canvas.left()) as f64 * st.ticks_per_px);
                st.scroll_x_ticks += before - after;
            }
        } else {
            if scroll_delta.y != 0.0 {
                st.scroll_y_px = (st.scroll_y_px - scroll_delta.y).max(0.0);
            }
            if mods.shift && scroll_delta.x != 0.0 {
                st.scroll_x_ticks =
                    (st.scroll_x_ticks - (scroll_delta.x as f64) * st.ticks_per_px).max(0.0);
            }
        }
        if resp_outer.dragged() && resp_outer.drag_started_by(egui::PointerButton::Middle) {
            let d = resp_outer.drag_delta();
            st.scroll_x_ticks = (st.scroll_x_ticks - (d.x as f64) * st.ticks_per_px).max(0.0);
            st.scroll_y_px = (st.scroll_y_px - d.y).max(0.0);
        }

        // Ruler: grid and labels (horizontal scroll only)
        {
            let p = ui.painter_at(top_ruler);
            draw_time_grid(
                &p,
                top_ruler,
                st.scroll_x_ticks,
                st.ticks_per_px,
                self.tick_freq,
            );
        }

        // Gutter: tracks (vertical scroll only)
        {
            let p = ui.painter_at(left_gutter);
            draw_track_gutter(
                &p,
                left_gutter,
                st.scroll_y_px,
                st.track_height,
                st.track_gap,
            );
        }

        // Canvas: clips (both scroll axes)
        {
            let p = ui.painter_at(canvas);
            draw_clips(&p, canvas, &st, ctx.world);
        }

        // Playhead over ruler+canvas
        let x_play = canvas.left()
            + ((st.playhead_tick as f64 - st.scroll_x_ticks) / st.ticks_per_px) as f32;
        ui.painter().line_segment(
            [
                egui::pos2(x_play, top_ruler.bottom()),
                egui::pos2(x_play, canvas.bottom()),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 64, 64)),
        );

        // Scrub on click in ruler
        if let Some(pos) = ui.ctx().pointer_latest_pos()
            && top_ruler.contains(pos)
            && ui.ctx().input(|i| i.pointer.primary_clicked())
        {
            st.playhead_tick =
                (st.scroll_x_ticks + (pos.x - canvas.left()) as f64 * st.ticks_per_px) as u64;
        }

        // Bottom tools (zoom slider)
        let mut z_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bottom)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let mut zoom = (self.tick_freq as f64 / st.ticks_per_px) as f32; // px per second
        let resp = z_ui.add(egui::Slider::new(&mut zoom, 10.0..=800.0).text("px/s"));
        if resp.changed() {
            st.ticks_per_px = self.tick_freq as f64 / zoom.max(1.0) as f64;
        }

        // Save state back to World
        let ui_ctx =
            ctx.world.resource::<lunaris_api::plugin::UiContext<
                lunaris_api::plugin::ArcSwapStorage<TimelineUiState>,
            >>();
        let mut write = ui_ctx.write();
        *write = st;
        write.swap();
    }
}

fn draw_time_grid(
    p: &egui::Painter,
    rect: egui::Rect,
    scroll_x_ticks: f64,
    ticks_per_px: f64,
    tps: u64,
) {
    let step = choose_grid_step(ticks_per_px, tps);
    let start_tick = scroll_x_ticks.max(0.0) as u64;
    let end_tick = (scroll_x_ticks + rect.width() as f64 * ticks_per_px) as u64;
    let first = (start_tick / step) * step;
    let col = p
        .ctx()
        .style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .linear_multiply(0.6);
    let font = egui::FontId::monospace(11.0);
    let mut t = first;
    while t <= end_tick {
        let x = rect.left() + ((t as f64 - scroll_x_ticks) / ticks_per_px) as f32;
        p.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, col),
        );
        if ((t - first) / step).is_multiple_of(5) {
            p.text(
                egui::pos2(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                format_time_ticks(t, tps),
                font.clone(),
                p.ctx().style().visuals.text_color(),
            );
        }
        t = t.saturating_add(step);
    }
}

fn draw_track_gutter(
    p: &egui::Painter,
    rect: egui::Rect,
    scroll_y_px: f32,
    track_h: f32,
    gap: f32,
) {
    let col = p
        .ctx()
        .style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color;
    let step = track_h + gap;
    let mut y = rect.top() - (scroll_y_px % step);
    let mut idx = (scroll_y_px / step).floor() as u64;
    while y < rect.bottom() {
        let r = egui::Rect::from_min_size(
            egui::pos2(rect.left(), y),
            egui::vec2(rect.width(), track_h),
        );
        p.rect_filled(r, 0.0, p.ctx().style().visuals.widgets.inactive.bg_fill);
        p.rect_stroke(
            r,
            0.0,
            egui::Stroke::new(1.0, col),
            egui::StrokeKind::Outside,
        );
        p.text(
            r.left_top() + egui::vec2(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            format!("Track {}", idx),
            egui::FontId::proportional(12.0),
            p.ctx().style().visuals.text_color(),
        );
        y += step;
        idx += 1;
    }
}

fn draw_clips(p: &egui::Painter, rect: egui::Rect, st: &TimelineUiState, world: &mut World) {
    let start_tick = st.scroll_x_ticks.max(0.0) as u64;
    let end_tick = (st.scroll_x_ticks + rect.width() as f64 * st.ticks_per_px) as u64;
    let mut q = world.query::<(Entity, &TimelineElement)>();
    for (ent, el) in q.iter(world) {
        if el.position.end < start_tick || el.position.start > end_tick {
            continue;
        }
        let x0 =
            rect.left() + ((el.position.start as f64 - st.scroll_x_ticks) / st.ticks_per_px) as f32;
        let x1 =
            rect.left() + ((el.position.end as f64 - st.scroll_x_ticks) / st.ticks_per_px) as f32;
        let y0 = rect.top()
            + ((el.track_num as f32) * (st.track_height + st.track_gap) - st.scroll_y_px);
        let y1 = y0 + st.track_height;
        let clip = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
        let sel = st.selection.contains(&ent);
        let fill = if sel {
            egui::Color32::from_rgb(80, 120, 220)
        } else {
            p.ctx().style().visuals.widgets.inactive.bg_fill
        };
        p.rect_filled(clip, 3.0, fill);
        p.rect_stroke(
            clip,
            3.0,
            egui::Stroke::new(
                1.0,
                p.ctx()
                    .style()
                    .visuals
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .color,
            ),
            egui::StrokeKind::Outside,
        );
    }
}

fn choose_grid_step(ticks_per_px: f64, tps: u64) -> u64 {
    let target_px = 60.0;
    let candidates_ms = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10_000];
    for ms in candidates_ms {
        let ticks = ((tps as u128) * (ms as u128) / 1000u128) as u64;
        if (ticks as f64 / ticks_per_px) >= target_px {
            return ticks;
        }
    }
    // Fallback to 1s
    tps
}

fn format_time_ticks(t: u64, tps: u64) -> String {
    let secs = t / tps;
    let ms = (t % tps) * 1000 / tps;
    format!("{secs}.{ms:03}")
}
