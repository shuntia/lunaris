**Architecture Plan**

- **Core Goal**: Headless-first, deterministic, crash-tolerant video engine with a Bevy ECS-centric architecture; UI is optional and pluggable.

**Engine Structure**

- **System Sets**: Ingest → Decode → GraphEval → Composite → Output → Persist.
- **Run Criteria**: Playback state, frame budget, dirty spans; deterministic ordering.
- **Broadcast Events**: Typed `Events<T>` for edits, transport, cache invalidation, plugin control.
- **Resources**: Clock/Timebase, RenderGraph, CacheManager, JobQueues, ColorProfile, ProjectStore.
- **Determinism**: Edits as diffs; stable schedule; content-addressed caches; reproducible exports.

**Timeline Model**

- **Entities**: Clip, Track, EffectNode, GraphEdge, Playhead, ProxyAsset, CacheEntry.
- **Edges**: Components describing connections in the processing graph.
- **Diff Log**: Apply edits as diffs; emit events; recompute only affected subgraphs/time ranges.

**Jobs and Scheduling**

- **Pools**: Frame-critical, Default, I/O, Background; cooperative cancellation tokens.
- **Priorities**: Immediate, VideoFrame, Normal, Deferred, Background.
- **Join Semantics**: Foreground join waits until frame/normal work reaches zero; background optional.

**Rendering Pipeline**

- **GPU-first**: wgpu-based nodes, minimal render graph, texture/buffer pools.
- **Incremental**: Dirty-region/dirty-span recompute; cache node outputs aggressively.
- **Color**: OCIO/ACES planned; linear float pipeline; HDR metadata awareness.
- **Proxies**: Automatic generation and transparent switching based on zoom/quality.

**Plugins**

- **Registration**: Plugins add components/events/systems into named sets; declare capabilities (GPU, disk, network).
- **Features**: plugin.toml declares features (e.g., Gui). Build-time codegen maps declared features to trait implementations; compile fails if claimed features are missing.
- **Isolation Path**: In-process today; future sandbox (WASM/WASI or OOP) for risky effects/codecs.

**UI Layer (Optional Feature)**

- **Toolkit**: egui + egui_tiles for docking; UI systems live in separate UI sets.
- **Viewer**: A consumer of frames via shared pools; headless mode omits UI sets entirely.
- **Abstraction**: Keep a thin UI boundary to allow future swaps/custom GPU widgets for the timeline.

**Reliability**

- **Autosave/Recovery**: Periodic snapshots + event log; atomic writes.
- **Watchdogs**: Timeouts per node/op; kill and recover misbehaving tasks/plugins.
- **Error Semantics**: Structured errors (no panics in release) with recovery strategies.

**Internationalization**

- **Core**: English-only structured errors for logs/diagnostics.
- **UI**: Localized strings (JP/EN) for user-facing messages; map error codes to localized text.

**CLI/Headless**

- **Commands**: Load project, set playhead, render range, export.
- **Automation**: Script/macros for batch workflows; deterministic runs produce identical outputs.

**Roadmap (High Level)**

- **Milestone 1**: Engine schedule + events/resources; minimal render graph; proxies; autosave; headless CLI.
- **Milestone 2**: Golden-frame tests; tracing; example plugins (text, LUT); deterministic exports.
- **Milestone 3**: UI feature crate with egui_tiles shell; timeline/viewer basics; capability enforcement.
- **Milestone 4**: Advanced color; HDR preview/export; improved audio engine; sandbox path for risky plugins.

**Extreme Performance Plan**

- **Scheduling and Concurrency**
  - Replace Mutex+Condvar queues with lock-free/bounded MPMC (e.g., crossbeam) for hot paths; per-thread local queues + work-stealing.
  - Per-priority executors: Immediate/Frame/Default/Background each have quotas; implement aging to avoid starvation.
  - Admission control: bounded queues with backpressure signals to producers; expose saturation metrics (queue depth, wait time).
  - Cancellation/timeouts: cooperative cancel tokens for tasks; enforce frame budgets with soft preemption points.
  - OS-level tuning: optional real-time/priority hints for frame workers; thread affinity for cache locality; NUMA awareness later.
  - Async layer: small admission stage that moves jobs from priority queues to Tokio with limited concurrency per class.

- **ECS/Data Model**
  - Store timeline in SoA-friendly components; minimize archetype churn; prefer fixed-capacity SmallVec for small lists.
  - Explicit change flags instead of broad change detection; batch edits into diffs applied at system boundaries.
  - Use stable system ordering and SystemSets with clear dependencies; minimize world locks during frame-critical sets.

- **Media I/O**
  - Prefetch + read-ahead windows; memory-map proxies where appropriate (large sequential reads).
  - Hardware decode path with zero-copy surfaces: NVDEC/D3D11/VideoToolbox/VA-API → share to wgpu (D3D11 shared textures, IOSurface, dma-buf).
  - Prefer NV12/YUV pipelines in GPU; convert only at composite/display; avoid CPU colorspace conversion.
  - Background thumbnailer and hash-based media indexing; stagger IO to avoid bursts.

- **GPU Render Graph**
  - Minimal render graph with pass fusion; persistent pipelines and descriptor pools; avoid rebinding churn.
  - Tile/region-of-interest rendering for timeline preview; partial redraw on edits; cached intermediates with content-addressed keys.
  - Compute-based color convert/LUT/tonemap; bindless resources where supported; double-buffer command encoders and fences.
  - Texture/buffer pools with strict lifetime tracking; avoid map/unmap on the render thread; use staging buffers and async transfers.

- **Caching and Proxies**
  - Content-addressed cache: keys derived from inputs + params + code version; LRU with size/age heuristics.
  - Automatic proxy pyramid (full/half/quarter) and bit-depth reductions; adaptive selection by zoom/quality.
  - Constant folding for node graphs; skip passes with identity transforms.

- **Audio Engine**
  - Dedicated real-time audio thread; lock-free ring buffers to the engine; zero allocations in callback.
  - High-quality resampling/time-stretch (phase vocoder/WSOLA) with SIMD; prefetch windows; precise A/V sync primitives.

- **Memory Discipline**
  - Arena/slab allocators for transient data; bump allocators for per-frame scratch; prefer stack/SmallVec for tiny collections.
  - Pre-size vectors and reuse; avoid per-frame heap traffic; consider huge pages for large buffers if beneficial.

- **Determinism/Testing**
  - Golden-frame tests for nodes and pipelines; scenario replay for timeline operations; seeded RNGs; stable ordering.
  - Frame HUD/tracing with spans across CPU/GPU; export timing and cache hit metrics; CLI to dump render graphs.

- **UI Performance (Optional Feature)**
  - Decouple UI framerate from engine; coalesce input; throttle expensive layouts.
  - Custom GPU timeline widget for heavy scenes; keep egui for docks; batch text rendering.

- **Build/Tooling**
  - Split crates to isolate hot paths; stable inner crates to minimize rebuilds; enable `sccache` + `lld/mold`.
  - Dev: incremental builds with higher codegen-units; Release: LTO/thin-LTO selectively on hot crates.
  - Feature-gate expensive subsystems/effects; produce a small “preview” build for older hardware.

**Previously Suggested Upgrades (Consolidated)**

- Scheduler fairness and quotas; per-priority async executors; bounded queues and backpressure; cancel tokens and timeouts.
- Metrics: queue depths, wait times, task durations; tracing spans across CPU/GPU; simple HUD/CLI metrics dump.
- Plugin dispatch: codegen registry with compile-time feature checks; consider hybrid trait-object dispatch to cap compile time for many plugins.
- Build pipeline: avoid destructive Cargo.toml rewrites; generate a registry file without renaming root; pin root toolchain; atomic file ops.
- Memory/GPU pools: persistent descriptor sets; texture/buffer pools; content-addressed caches; dirty-span recompute.
- Determinism: stable system order; seeded RNG; golden-frame CI; scenario replay harness.
