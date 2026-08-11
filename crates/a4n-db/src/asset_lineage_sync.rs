//! Canonical manga/video artifact projection into the unified generation/media chain.

use std::collections::BTreeMap;

use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{
        chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
        chapter_video_prompt,
    },
    project::{project_source_snapshot, source_snapshot_chapter},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

mod persistence;

use persistence::{
    GenerationSpecUpsertInput, sync_video_prompt_media_refs, upsert_generation_spec,
    upsert_prompt_artifact,
};

const CANONICAL_SCHEMA_VERSION: u64 = 1;
const PROJECT_KIND_TEXT: &str = "text";
const SOURCE_MODE_SELF: &str = "self";
const SOURCE_MODE_FREEFORM: &str = "freeform";

mod types;
#[cfg(test)]
use types::ProjectLineageContext;
pub use types::*;
use types::{FamilyLineageContext, SyncedPromptRef};

mod rules;
use rules::*;

mod context;
use context::*;

mod manga;
use manga::sync_manga_prompt;

mod video;
use video::sync_video_prompt;

mod media;
use media::{sync_generated_image_media, sync_generated_video_media};

mod sync;
pub use sync::*;

#[cfg(test)]
mod tests;
