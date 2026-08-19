mod inspiration_story;
pub(super) use inspiration_story::{add_inspiration_nodes, add_story_nodes};

mod profiles;
pub(super) use profiles::add_profile_nodes;

mod manga;
pub(super) use manga::add_manga_nodes;

mod video;
pub(super) use video::add_video_nodes;

mod generation;
pub(super) use generation::{add_derived_project_nodes, add_generation_nodes};
