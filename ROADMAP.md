## Lunaris Roadmap (to v0.1)

This is the single source of truth for the near-term plan. No other planning docs should contradict this file.

### Guiding Principles
- **Headless-first core, UI-optional:** the ECS+orchestrator pipeline runs identically in CLI and GUI builds.
- **Deterministic scheduling:** every system and job runs in a predictable order; shared state flows world → UI via a single registry.
- **Plugins after spine:** third parties only matter once the host has a stable API and demo.
- **Milestones end with a demo:** if there’s no runnable proof, the milestone isn’t done.

---

### Milestone 0 — World/UI Spine (NOW)
Goal: replace the dummy world/UI bridge with deterministic plumbing so plugins interact with the real ECS.

1. **Deterministic Plugin IDs**
   - Introduce `PluginKey(&'static str)` newtype; registration macros require `id: "com.lunaris.core.timeline"`.
   - Build a startup registry that panics on duplicate IDs and exposes metadata for later use (commands, registry, CLI).

2. **Shared UI State Registry**
   - Implement `SharedUiStateRegistry` (resource in `lunaris_core`) built on `DashMap<PluginKey, PluginUiEntry>`.
   - Each `PluginUiEntry` wraps `ArcSwap<Box<dyn UiState>>` + version counter for cheap publish/snapshot.
   - Provide API: `publish(&self, key, Box<dyn UiState>) -> u64`, `snapshot(&self, key) -> Option<(u64, Arc<Box<dyn UiState>>)>`.

3. **PluginContext Hooks**
   - Extend `PluginContext` with the plugin’s key and helpers:
     - `publish_ui_state(Box<dyn UiState>)`
     - `ui_state_snapshot()`
     - `send_command(UiCommand)` (using the existing `ui_to_world_sender`)
   - These methods enforce isolation—plugins never touch the registry directly.

4. **World + UI Wiring**
   - Insert the registry as a resource when the world thread boots; keep an `Arc` for UI reads.
   - Remove the dummy `World`/`Orchestrator` creation inside `AppBehavior`. Real data must flow through the registry.
   - UI panes fetch snapshots by plugin key and redraw only when versions change.

**Exit demo:** builtin test plugin publishes a counter resource from the world, UI pane reflects it live, and button clicks send commands back that mutate the world.

---

### Milestone 1 — Timeline Vertical Slice
Goal: load one project, run a render request through the orchestrator, and display the resulting frame.

Tasks
- Define minimal timeline components/resources (Clip, Track, Playhead, RenderRequest).
- Build a hard-coded project loader + JSON save.
- Flesh out `render_orchestrator_system`: create jobs per frame, await results, store `RenderOutput`.
- Add a built-in viewer pane (not a plugin yet) that pulls `RenderOutput`, uploads to egui texture, and shows it alongside metadata.
- Thread orchestrator join/flush into the world loop so playhead scrubbing waits for the right frame batch.

**Exit demo:** start app, load canned project, scrub playhead, see frames render in the viewer pane, close/reopen and reload the same state.

---

### Milestone 2 — Plugin Surface Freeze (v0.1 target)
Goal: stabilize `lunaris_api` and prove third-party plugins work end-to-end.

Tasks
- Document/export the `lunaris_api::prelude`; hide Bevy internals behind `lunaris_ecs`.
- Introduce `templates/plugin_basic` + smoke test that runs against the host and exercises state/command flow.
- Add CI job (or `just` target) that builds the template and launches a headless smoke run.
- Version the API surface (e.g., `API_VERSION` const + mismatch error).

**Exit demo:** `templates/plugin_basic` compiles, registers, publishes state, and responds to commands in a real host run.

---

### Milestone 3 — Dispatcher + Orchestrator Polish
Goal: move from ad-hoc render requests to a structured render DAG with observability.

Tasks
- Flesh out `lunaris_core/src/dispatcher`: DAG structure, submission API, integration with the orchestrator queues.
- Implement frame-level handles/counters so UI can await “frame N complete” without busy waits.
- Fix WorkerPool reconfigure wake-ups and add queue-depth metrics.
- Expose a profiling panel (built-in UI tab) showing queue sizes, running jobs, and per-frame latency.

**Exit demo:** open profiler view, submit renders, watch live metrics update; reconfigure threads without deadlocks.

---

### Milestone 4 — Headless + CLI
Goal: prove headless-first claims with a working CLI that shares the same ECS pipeline.

Tasks
- Add CLI commands: `lunaris_core headless --project <path> --range <start:end>`.
- Reuse the same world schedule and orchestrator; no alternative code paths.
- Emit deterministic hashes for rendered frames; add integration test to compare two runs.

**Exit demo:** run CLI render twice, compare hashes, confirm determinism; UI build uses identical engine path.

---

### Milestone 5 — Registry + Command Infrastructure
Goal: lock down plugin/command IDs ahead of distribution work.

Tasks
- Implement command registration (inventory entries with `plugin_id`, `command_name`, serialize/deserialize hooks).
- Log commands (world-bound) for future undo; verify they can be replayed from disk.
- CLI/UI stubs for listing installed plugins and their registered commands; enforce ID uniqueness at load time.

**Exit demo:** host lists installed plugins + commands, rejects duplicates, and can serialize + replay a simple command log.

---

### Backlog / Later (not active)
- Distributed rendering prototype (needs Milestones 0–4 complete).
- Telemetry/analytics opt-in.
- Hot reload for GUI-only plugins.
- Web/desktop installer tooling.

Keep these ideas parked until the active milestones land; reopen this section only when earlier milestones ship.
