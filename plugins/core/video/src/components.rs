use lunaris_ecs::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct VideoSource {
    pub path: String,
}
