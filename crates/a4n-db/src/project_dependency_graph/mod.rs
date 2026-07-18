use a4n_core::error::AppError;
use a4n_core::models::{
    ai::generated_image,
    chapter::{chapter, chapter_manga_prompt, chapter_video_prompt},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    project::{novel_project, project_derivation},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod builder;
mod details;
mod scope;

use self::builder::{GraphBuilder, NodeSeedMetadata, node};
use self::details::*;
use self::scope::load_asset_graph_scope;
mod types;
pub use self::types::*;
use self::types::{
    CHAPTER_VIDEO_PROMPT_TYPES, VIDEO_FRAME_PROMPT_TYPES, VIDEO_SEGMENT_PLAN_PROMPT_TYPES,
};

mod helpers;
use self::helpers::*;

mod nodes;

mod graph;
pub use self::graph::*;

#[cfg(test)]
mod tests;
