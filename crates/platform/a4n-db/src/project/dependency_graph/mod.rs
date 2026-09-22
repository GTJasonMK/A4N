use a4n_db_models::models::chapter::chapter_video_prompt;

pub use a4n_domain::project::{
    AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode, AssetGraphNodeDetail,
    AssetLineageRecord, AssetLineageRef, GenerationInputSource, PromptSource,
};

mod builder;
mod details;
mod scope;

use self::builder::{GraphBuilder, NodeSeedMetadata, node};
use self::scope::load_asset_graph_scope;

const VIDEO_SEGMENT_PLAN_PROMPT_TYPES: &[&str] =
    &[chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN];
const VIDEO_FRAME_PROMPT_TYPES: &[&str] = &[
    chapter_video_prompt::PROMPT_TYPE_FRAME,
    chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME,
    chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME,
];
const CHAPTER_VIDEO_PROMPT_TYPES: &[&str] = &[chapter_video_prompt::PROMPT_TYPE_VIDEO];

mod helpers;

mod nodes;

mod graph;
pub use self::graph::{asset_dependency_graph, asset_dependency_graph_node_detail};

#[cfg(test)]
mod tests;
