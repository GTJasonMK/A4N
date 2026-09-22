//! 项目级联删除与相关清理（阶段 11 收口：从 a4n-services::project 迁入）。

use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait,
};

use crate::generation;
use crate::project as project_db;

mod types;
pub use types::{
    ChapterGenerationCleanupResult, GeneratedImageDeleteScope, ProfileSceneCleanupResult,
    ProjectDeleteDbError,
};

mod chapter_generation;
pub(crate) use chapter_generation::delete_chapter_generation_outputs;

mod profile_scene;
pub(crate) use profile_scene::delete_project_profile_scene_data;

mod cascade;
pub use cascade::delete_project_cascade_record;

#[cfg(test)]
mod tests;
