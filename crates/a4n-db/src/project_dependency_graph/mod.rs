use a4n_core::error::AppError;
use a4n_core::models::{
    ai::generated_image,
    chapter::{
        chapter,
        chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
        chapter_video_prompt,
    },
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    project::novel_project,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub use a4n_domain::project::{
    AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode, AssetGraphNodeDetail,
    AssetLineageRecord, AssetLineageRef, GenerationInputSource, PromptSource,
};

mod builder;
mod details;
mod scope;

use self::builder::{GraphBuilder, NodeSeedMetadata, node};
use self::details::*;
use self::scope::load_asset_graph_scope;

const VIDEO_SEGMENT_PLAN_PROMPT_TYPES: &[&str] = &["frame_segment_plan"];
const VIDEO_FRAME_PROMPT_TYPES: &[&str] = &[
    "frame_prompt",
    "first_frame_prompt",
    "chat_video_frame_prompt",
];
const CHAPTER_VIDEO_PROMPT_TYPES: &[&str] = &["video_prompt"];

mod helpers;
use self::helpers::*;

mod nodes;

mod graph;
pub use self::graph::*;

#[cfg(test)]
mod tests;
