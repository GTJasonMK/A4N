//! 角色管理服务：主角档案 + 属性事件溯源
//!
//! 对应文档 06-角色管理系统、07-角色属性事件溯源

mod attribute_events;
mod behavior;
mod deletion_mark;
mod profile;
pub mod snapshot_store;
mod state;
mod state_index;
mod timeline;

use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::character::protagonist_profile;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};

#[cfg(test)]
use attribute_events::validate_attribute_history_filter;
pub use attribute_events::{AttributeHistoryFilter, AttributeValueChangeInput};
pub use behavior::BehaviorRecordInput;
#[cfg(test)]
use behavior::validate_behavior_classification_results;
pub use deletion_mark::DeletionMarkRewindSummary;
pub use deletion_mark::{AttributeDeletionExecutionInput, AttributeDeletionRequestInput};
#[cfg(test)]
use deletion_mark::{next_deletion_mark_count, retained_deletion_mark_count};
pub(crate) use profile::is_text_discovered_profile;
pub use profile::{CharacterProfileDeletionSummary, delete_profile_in, profile_first_seen_chapter};
#[cfg(test)]
use state::{apply_attribute_to_state, normalize_base_attributes};
pub use state::{apply_profile_current_state, snapshot_attributes_from_data};
pub use state_index::StateIndexUpsertInput;
pub use timeline::CharacterTimelineDeletionSummary;

#[cfg(test)]
use sea_orm::Set;

#[derive(Clone)]
pub struct CharacterService {
    db: Arc<DatabaseConnection>,
}

pub const DELETION_THRESHOLD: i32 = 5;

fn validate_category(category: &str) -> Result<(), AppError> {
    if matches!(category, "explicit" | "implicit" | "social") {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "属性分类必须是 explicit/implicit/social".to_string(),
        ))
    }
}

fn validate_operation(operation: &str) -> Result<(), AppError> {
    if matches!(operation, "add" | "modify" | "delete") {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "属性事件 operation 必须是 add/modify/delete".to_string(),
        ))
    }
}

fn default_attribute_change_description(operation: &str, category: &str, key: &str) -> String {
    let action = match operation {
        "add" => "新增",
        "modify" => "修改",
        "delete" => "删除",
        _ => "变更",
    };
    format!("{action}{category}属性: {key}")
}

fn require_non_empty(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{field_name}不能为空")))
    } else {
        Ok(value.to_string())
    }
}

fn validate_chapter_number(chapter_number: i32) -> Result<(), AppError> {
    if chapter_number <= 0 {
        Err(AppError::InvalidArgument("章节号必须大于 0".to_string()))
    } else {
        Ok(())
    }
}

async fn require_profile_in<C>(
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {profile_id}")))
}

#[cfg(test)]
mod tests;
