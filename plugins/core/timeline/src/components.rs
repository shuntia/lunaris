use lunaris_ecs::prelude::*;
use lunaris_api::{render::RawImage, util::error::Result};

#[derive(Debug, Clone, Copy)]
pub struct TimelineSpan {
    pub start: u64,
    pub end: u64,
}

#[derive(Component)]
pub struct Playhead {
    pub current: u64,
}

#[derive(Component, Debug)]
pub struct TimelineElement {
    /// Track number of Timeline Element, or in other words, the Z-index.
    pub track_num: u64,
    pub position: TimelineSpan,
}

#[derive(Component, Debug)]
pub struct BindTo {
    pub id: Entity,
}

#[derive(Component, Debug)]
pub struct Renderable {
    pub render_result: Result<RawImage>,
}
