#[cfg(test)]
use a4n_core::models::media::media_asset;
use a4n_core::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
    galgame::galgame_scene,
    generation::{generation_spec, prompt_artifact},
    project::{novel_project, source_snapshot_profile},
};
use chrono::Utc;
#[cfg(test)]
use sea_orm::DbErr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
#[cfg(test)]
use std::collections::BTreeSet;

const PROJECT_KIND_GALGAME: &str = "galgame";
const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
const GALGAME_ENTITY_LINE: &str = "galgame_line";
const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
const GALGAME_SCENE_ENTRY_MAIN: &str = "main";
const GALGAME_SCENE_ENTRY_BRANCH: &str = "branch";
const GALGAME_LINE_TYPE_CHOICE: &str = "choice";
const GALGAME_SPEC_STATUS_DRAFT: &str = "draft";
const GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED: &str = "snapshot_backed";
const MEDIA_KIND_IMAGE: &str = "image";
const MEDIA_BINDING_ROLE_BACKGROUND: &str = "background";
const MEDIA_BINDING_ROLE_CG: &str = "cg";
const MEDIA_STATUS_COMPLETED: &str = "completed";
const GALGAME_SPEC_REF_INSERT_CHUNK_SIZE: usize = 1000;

fn require_galgame_media_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, GalgameMediaRecordError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

fn validate_galgame_generation_id(
    value: i64,
    field: &str,
) -> Result<(), GalgameGenerationRecordError> {
    if value <= 0 {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "{field} 必须是正整数"
        )));
    }
    Ok(())
}

fn require_galgame_generation_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, GalgameGenerationRecordError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

mod types;
pub use types::*;

mod read_models;
pub use read_models::*;

mod generation_specs;
pub use generation_specs::*;

mod media;
pub use media::*;

mod script_records;
pub use script_records::*;

#[cfg(test)]
mod tests;
