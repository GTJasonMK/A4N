//! 角色属性生命周期错误类型与纯辅助函数。
//!
//! 数据库写入与 RAG 同步编排已迁入 `context/character.rs`。

mod contracts;

pub use contracts::CharacterAttributeLifecycleError;

use a4n_core::error::AppError;
use a4n_core::models::character::attribute_event;
use a4n_db::character::{
    AppliedCharacterImplicitDecisionRecord, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
    DeletionMarkListStatus,
};

/// 从隐式审核决策结果中提取需要同步到 RAG 的属性事件。
pub fn implicit_review_attribute_event(
    applied: &AppliedCharacterImplicitDecisionRecord,
) -> Option<&attribute_event::Model> {
    applied.event.as_ref()
}

/// 规范化角色属性写输入：章节号、分类、key、原因与证据门禁。
///
/// 文案与 DB 层 `require_non_empty` / `validate_category` 对齐，避免 Host 侧双路径校验。
pub fn normalize_character_attribute_value_change(
    input: CharacterAttributeValueChangeInput,
) -> Result<CharacterAttributeValueChangeInput, AppError> {
    if input.chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节号必须大于 0".to_string()));
    }
    let category = input.category.trim().to_string();
    if !matches!(category.as_str(), "explicit" | "implicit" | "social") {
        return Err(AppError::InvalidArgument(
            "属性分类必须是 explicit/implicit/social".to_string(),
        ));
    }
    let key = require_non_empty_field(&input.key, "属性 key")?;
    let cause = require_non_empty_field(&input.cause, "属性变更原因")?;
    let evidence = require_non_empty_field(&input.evidence, "属性变更证据")?;
    let change_description = match input.change_description {
        Some(value) => Some(require_non_empty_field(&value, "属性变更摘要")?),
        None => None,
    };
    Ok(CharacterAttributeValueChangeInput {
        profile_id: input.profile_id,
        chapter_number: input.chapter_number,
        category,
        key,
        value: input.value,
        change_description,
        cause,
        evidence,
    })
}

/// 规范化属性删除请求输入。
pub fn normalize_character_attribute_deletion_request(
    input: CharacterAttributeDeletionRequestInput,
) -> Result<CharacterAttributeDeletionRequestInput, AppError> {
    if input.chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节号必须大于 0".to_string()));
    }
    let category = require_attribute_category(&input.category)?;
    let key = require_non_empty_field(&input.key, "属性 key")?;
    let reason = require_non_empty_field(&input.reason, "删除原因")?;
    let evidence = require_non_empty_field(&input.evidence, "删除证据")?;
    Ok(CharacterAttributeDeletionRequestInput {
        profile_id: input.profile_id,
        category,
        key,
        chapter_number: input.chapter_number,
        reason,
        evidence,
    })
}

/// 规范化属性删除执行输入（Host 可选章节号/原因）。
pub fn normalize_character_attribute_deletion_execution(
    profile_id: i64,
    category: String,
    key: String,
    chapter_number: Option<i32>,
    cause: Option<String>,
    evidence: String,
) -> Result<CharacterAttributeDeletionExecutionInput, AppError> {
    let chapter_number = chapter_number
        .ok_or_else(|| AppError::InvalidArgument("执行属性删除缺少章节号".to_string()))?;
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument(
            "执行属性删除章节号必须大于 0".to_string(),
        ));
    }
    let category = require_attribute_category(&category)?;
    let key = require_non_empty_field(&key, "属性 key")?;
    let event_cause = match cause {
        Some(value) => require_non_empty_field(&value, "删除原因")?,
        None => {
            return Err(AppError::InvalidArgument("删除原因不能为空".to_string()));
        }
    };
    let evidence = require_non_empty_field(&evidence, "删除证据")?;
    Ok(CharacterAttributeDeletionExecutionInput {
        profile_id,
        category,
        key,
        event_cause,
        evidence,
        chapter_number,
    })
}

/// 解析删除标记列表的可选分类过滤。
pub fn parse_deletion_mark_list_category(
    category: Option<&str>,
) -> Result<Option<String>, AppError> {
    match category.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(category) => Ok(Some(require_attribute_category(category)?)),
    }
}

/// 解析删除标记列表 status 过滤。
pub fn parse_deletion_mark_list_status(
    status: Option<&str>,
) -> Result<DeletionMarkListStatus, AppError> {
    match status.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") => Ok(DeletionMarkListStatus::All),
        Some("pending") => Ok(DeletionMarkListStatus::Pending),
        Some("ready") => Ok(DeletionMarkListStatus::Ready),
        Some("executed") => Ok(DeletionMarkListStatus::Executed),
        Some(_) => Err(AppError::InvalidArgument(
            "删除标记 status 必须是 all/pending/ready/executed".to_string(),
        )),
    }
}

/// 规范化删除标记重置的分类与 key。
pub fn normalize_deletion_mark_reset_target(
    category: &str,
    key: &str,
) -> Result<(String, String), AppError> {
    Ok((
        require_attribute_category(category)?,
        require_non_empty_field(key, "属性 key")?,
    ))
}

fn require_attribute_category(category: &str) -> Result<String, AppError> {
    let category = category.trim().to_string();
    if matches!(category.as_str(), "explicit" | "implicit" | "social") {
        Ok(category)
    } else {
        Err(AppError::InvalidArgument(
            "属性分类必须是 explicit/implicit/social".to_string(),
        ))
    }
}

fn require_non_empty_field(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{field_name}不能为空")))
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests;
