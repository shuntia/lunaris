codex
Here’s what I found and how to improve frame orchestration.

What It Is

- Core: Rust workspace with lunaris_core (engine + UI shell) and lunaris_api (plugin/dev API).
- UI: egui/eframe shell, plugins via generated registry, currently a dummy plugin.
- Orchestrator: Custom worker pool with priorities in lunaris_core/src/orchestrator/worker.rs.
- Design intent: Headless-first, deterministic engine, explicit scheduling, strong error semantics (see PLANS.md).

Coding Principles

- Strong typing and explicit APIs: small, focused types (Job, AsyncJob, Priority).
- Structured errors: rich LunarisError with granular variants.
- Separation of concerns: API crate vs. core runtime vs. plugins.
- Determinism focus: scheduling, caching, pipeline stages planned in PLANS.md.
- Performance-aware: dedicated queues, bounded frame queue, async runtime for async jobs.

Current Orchestration

- Queues: default_q (Immediate/Normal/Deferred), frame_q (VideoFrame; bounded lock-free), bg_q (Background).
- Threads: Separate workers per queue; tokio runtime for async jobs.
- Counters: fg_jobs and bg_jobs plus a Condvar for join.
- Joins: join_foreground waits for all non-background jobs; join_all waits for all jobs.
- Backpressure: frame_q bounded (1024); add_job returns RenderQueueFull on overflow.

Gaps (Frame Completion)

- No per-frame tracking: fg_jobs is global, so you can’t tell when one specific frame finishes.
- No job/group handles: add_job returns only NResult, not something awaitable.
- No cancellation: No way to cancel work for superseded frames (e.g., viewer scrubs).
- Limited observability: No per-frame metrics, queue depth, or progress reporting.
- Reconfigure bug: reconfigure_threads sets stopping = true and immediately join()s without waking workers waiting on condvars (risk of deadlock).

Concrete Improvements (Frame-Level Completion)

- Frame IDs and grouping:
    - Add FrameId and group semantics to Job/AsyncJob (e.g., JobMeta { priority, group: Option<GroupId> }).
    - Track per-frame counts: FrameTracker holds HashMap<FrameId, FrameState { pending: AtomicU32, cv/waker }.
- Frame builder API:
    - Orchestrator::begin_frame(frame_id) -> FrameBuilder.
    - FrameBuilder::submit_tile(...), submit_async(...) wraps jobs to increment/decrement the frame counter.
    - FrameBuilder::finish() -> FrameHandle that is awaitable (implements Future) and also offers wait(timeout) and is_complete().
- Completion signaling:
    - Publish FrameComplete(frame_id, stats) event or callback for UI/consumer.
    - Expose non-blocking poll_frame(frame_id) -> Option<Progress> for rendering HUD/progress.
- Cancellation:
    - Add cooperative cancellation tokens on grouped jobs; cancel prior frames on scrubs/playhead jumps.
    - Dropping FrameHandle can cancel pending work for that frame (optional but valuable).
- Backpressure controls:
    - For VideoFrame: prefer bounded admission with either blocking enqueue (short wait) or TryEnqueue/EnqueueResult::Full plus retry policy.
    - Expose queue depth metrics so producers can choose tile batch sizes.

Other Orchestration Improvements

- Fix reconfigure deadlock:
    - In reconfigure_threads, call notify_all() on default_q.cv, frame_q.cv, bg_q.cv before joining to wake sleepers.
- Handles for single jobs:
    - Return a JobHandle/AsyncHandle from add_job/add_job_async that resolves when the job completes (oneshot channel or small Future).
- Observability:
    - Track per-queue depth, wait time, execution time; emit tracing spans with frame_id/tile_id.
    - Expose a lightweight metrics snapshot API for UI overlays and logs.
- Fairness/aging:
    - Consider aging in PriorityQueues so Deferred isn’t starved under sustained Immediate/Normal.
    - Optional: use per-worker local queues with crossbeam_deque and work-stealing for better throughput.
- Async tuning:
    - If async background load grows, consider a small admission stage into tokio with concurrency caps per priority class.

Quick Wins

- Reconfigure wake-up: In WorkerPool::reconfigure_threads, notify_all() on all CVs before join() to avoid hangs.
- Add per-frame counters:
    - Minimal path: add FrameId and a FrameWaiter map with a blocking join_frame(frame_id) that waits until its counter hits zero.
    - Wrap Priority::VideoFrame submissions to increment/decrement the frame’s counter.
- Add enqueue_frame(frame_id, job) helpers:
    - Encourage explicit grouping now, even if full FrameBuilder comes later.

If you want, I can:

- Patch the reconfigure_threads wake-up bug.
- Sketch the minimal FrameId + join_frame path in worker.rs and request/mod.rs.
- Add a FrameHandle that you can await or poll from the UI.
