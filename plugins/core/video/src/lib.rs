use lunaris_api::{
    export_plugin,
    plugin::{Plugin, PluginContext, PluginReport, Renderer, RenderJob, RenderTask},
    util::error::{Result, LunarisError},
};
use lunaris_ecs::prelude::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod components;
mod decoder;
use decoder::Decoder;

export_plugin!(VideoPlugin, id: "lunaris.core.video", name: "Video Backend", [Renderer]);

pub struct VideoPlugin {
    // Cache decoders by path to avoid re-opening files
    decoders: Arc<Mutex<HashMap<String, Arc<Mutex<Decoder>>>>>,
}

impl Plugin for VideoPlugin {
    fn new() -> Self
    where
        Self: Sized,
    {
        #[cfg(feature = "real_ffmpeg")]
        {
            // Initialize ffmpeg
            if let Err(e) = ffmpeg_next::init() {
                eprintln!("Warning: FFmpeg initialization failed: {}", e);
            }
        }
        #[cfg(not(feature = "real_ffmpeg"))]
        {
            eprintln!("Warning: VideoPlugin running in MOCK mode (no ffmpeg)");
        }
        
        Self {
            decoders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn init(&self, _ctx: PluginContext<'_>) -> Result {
        Ok(())
    }

    fn add_schedule(&self, _schedule: &mut lunaris_ecs::Schedule) -> Result {
        Ok(())
    }

    fn update_world(&mut self, _ctx: PluginContext<'_>) -> Result {
        Ok(())
    }

    fn report(&self, _ctx: PluginContext<'_>) -> PluginReport {
        PluginReport::Operational
    }

    fn shutdown(&mut self, _ctx: PluginContext<'_>) {}

    fn reset(&mut self, _ctx: PluginContext<'_>) {
        self.decoders.lock().unwrap().clear();
    }
}

impl Renderer for VideoPlugin {
    fn schedule_render(&self, job: RenderJob) -> Result<RenderTask> {
        let path_prop = job.parameter("path").ok_or(LunarisError::InvalidArgument {
            name: "path".to_string(),
            reason: Some("Missing 'path' property for video render job".to_string()),
        })?;

        let path_str = match path_prop {
            lunaris_api::types::Property::String(s) => s.clone(),
            lunaris_api::types::Property::Path(p) => p.to_string_lossy().to_string(),
            _ => return Err(LunarisError::InvalidArgument {
                name: "path".to_string(),
                reason: Some("Property 'path' must be String or Path".to_string()),
            }),
        };

        // Get or create decoder
        let decoder = {
            let mut cache = self.decoders.lock().unwrap();
            if let Some(d) = cache.get(&path_str) {
                d.clone()
            } else {
                let d = Arc::new(Mutex::new(Decoder::new(&PathBuf::from(&path_str))?));
                cache.insert(path_str.clone(), d.clone());
                d
            }
        };

        // Calculate timestamp (assuming 60fps for now, should be in job params)
        // job.frame is the global timeline frame. We need local time.
        // Assuming job.frame IS the local time for this clip for now.
        // In a real DAG, the time would be mapped.
        let timestamp_ms = (job.frame * 1000 / 60) as i64;

        // Return a future that executes the decode on a thread pool (or here if blocking)
        // Since RenderTask is BoxFuture, we can use async block.
        // However, ffmpeg is blocking. We should ideally spawn_blocking.
        // But we don't have access to tokio runtime here directly unless we use Handle::current().
        
        Ok(Box::pin(async move {
            // This will block the executor thread if not careful. 
            // For high-perf, this should be offloaded.
            // Assuming the caller (Orchestrator) runs this on a worker thread.
            let mut dec = decoder.lock().unwrap();
            dec.decode_frame(timestamp_ms)
        }))
    }
}
