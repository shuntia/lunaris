use std::collections::VecDeque;
use std::time::{Duration, Instant};

use lunaris_api::egui::Ui;
use lunaris_api::plugin::{Gui, Plugin, PluginContext, PluginReport};
use lunaris_api::request::OrchestratorProfile;
use lunaris_api::{export_plugin, util::error::Result};

export_plugin!(Profiler, id: "lunaris.core.profiler", [Gui]);

#[derive(Clone)]
pub struct Profiler {
    start: Instant,
    last_frame: Instant,
    frame_count: u64,
    last_dt: Duration,
    recent_fps: VecDeque<f32>,
    max_samples: usize,
    profiles: Option<OrchestratorProfile>,
}

impl Profiler {
    fn push_fps(&mut self, fps: f32) {
        if self.recent_fps.len() >= self.max_samples {
            self.recent_fps.pop_front();
        }
        self.recent_fps.push_back(fps);
    }
    fn avg_fps(&self) -> f32 {
        if self.recent_fps.is_empty() {
            0.0
        } else {
            self.recent_fps.iter().sum::<f32>() / (self.recent_fps.len() as f32)
        }
    }
    fn min_fps(&self) -> f32 {
        if self
            .recent_fps
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
            .is_finite()
        {
            {
                self.recent_fps
                    .iter()
                    .copied()
                    .fold(f32::INFINITY, f32::min)
            }
        } else {
            0.0
        }
    }
    fn max_fps(&self) -> f32 {
        if self
            .recent_fps
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
            .is_finite()
        {
            {
                self.recent_fps
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max)
            }
        } else {
            0.0
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            frame_count: 0,
            last_dt: Duration::from_millis(0),
            recent_fps: VecDeque::with_capacity(240),
            max_samples: 240,
            profiles: None,
        }
    }
}

impl Plugin for Profiler {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self::default()
    }

    fn name(&self) -> &'static str {
        "Profiler"
    }

    fn init(&self, _ctx: PluginContext<'_>) -> Result {
        Ok(())
    }

    fn add_schedule(&self, _schedule: &mut lunaris_api::plugin::Schedule) -> Result {
        Ok(())
    }

    fn update_world(&mut self, ctx: PluginContext<'_>) -> Result {
        // Frame timing
        self.profiles = Some(ctx.orch.profile());
        let now = Instant::now();
        self.last_dt = now.saturating_duration_since(self.last_frame);
        self.last_frame = now;
        self.frame_count += 1;
        let fps = if self.last_dt.as_secs_f32() > 0.0 {
            1.0 / self.last_dt.as_secs_f32()
        } else {
            0.0
        };
        self.push_fps(fps);

        // Keep an up-to-date orchestrator profile for the UI
        self.profiles = Some(ctx.orch.profile());

        // Optionally: touch world to keep stats fresh (entity count shown in UI)
        let _entity_count = ctx.world.entities().len();
        Ok(())
    }

    fn report(&self, _ctx: PluginContext<'_>) -> PluginReport {
        PluginReport::Operational
    }

    fn shutdown(&mut self, _ctx: PluginContext<'_>) {}

    fn reset(&mut self, _ctx: PluginContext<'_>) {
        *self = Self::default();
    }
}

impl Gui for Profiler {
    fn ui(&self, ui: &mut Ui, ctx: PluginContext<'_>) {
        ui.heading("Profiler");

        let uptime = self.start.elapsed();
        let entity_count = ctx.world.entities().len();
        let fps_now = if self.last_dt.as_secs_f32() > 0.0 {
            1.0 / self.last_dt.as_secs_f32()
        } else {
            0.0
        };

        ui.label(format!(
            "Uptime: {:>5.2}s | Frames: {}",
            uptime.as_secs_f32(),
            self.frame_count
        ));
        ui.label(format!(
            "FPS: {:>6.2} | avg: {:>6.2} | min: {:>6.2} | max: {:>6.2}",
            fps_now,
            self.avg_fps(),
            self.min_fps(),
            self.max_fps()
        ));
        ui.label(format!("Entities: {}", entity_count));

        ui.separator();
        ui.label("Orchestrator utils");
        if let Some(profile) = self.profiles.as_ref() {
            ui.label(format!(
                "Contented tasks: i:{} n:{} d:{} f:{}",
                profile.immediate, profile.normal, profile.deferred, profile.frame
            ));
            ui.label(format!("Running tasks: {}", profile.running_tasks));
        } else {
            ui.label("Profiling data unavailable yet");
        }
        if ui.button("Spawn CPU job (~1000ms)").clicked() {
            let _ = ctx.orch.submit_job_boxed(
                Box::new(|| {
                    let start = Instant::now();
                    // Busy-loop ~10ms
                    while start.elapsed() < Duration::from_millis(1000) {
                        std::hint::spin_loop();
                    }
                }),
                lunaris_api::request::Priority::Normal,
            );
        }
        if ui.button("Spawn frame job (fast)").clicked() {
            let _ = ctx.orch.submit_job_boxed(
                Box::new(|| {
                    // Small computation to simulate frame task
                    let mut x = 0u64;
                    for i in 0..100_000 {
                        x = x.wrapping_add(i);
                    }
                    let _ = x;
                }),
                lunaris_api::request::Priority::VideoFrame,
            );
        }
        if ui.button("Spawn async sleep (100ms)").clicked() {
            let _ = ctx.orch.submit_async_boxed(
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }),
                lunaris_api::request::Priority::Background,
            );
        }
        if ui.button("Join foreground").clicked() {
            let _ = ctx.orch.join_foreground();
        }

        ui.separator();
        ui.label("Thread config");
        // Small presets
        if ui.button("Threads: low (1/1/1)").clicked() {
            ctx.orch.set_threads(1, 1, 1);
        }
        if ui.button("Threads: balanced").clicked() {
            // Balanced-ish defaults; see WorkerPool::balanced
            let p = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            let def = p.max(1);
            let frame = (p / 2).max(1);
            let bg = 1usize;
            ctx.orch.set_threads(def, frame, bg);
        }
    }
}
