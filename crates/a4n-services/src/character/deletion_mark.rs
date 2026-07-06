use a4n_core::error::AppError;
use a4n_core::models::character::{attribute_event, protagonist_deletion_mark};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::attribute_events::AttributeEventRecordInput;
use super::state::attribute_value_in_state;
use super::{
    CharacterService, DELETION_THRESHOLD, default_attribute_change_description, require_non_empty,
    require_profile_in, validate_category, validate_chapter_number,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeletionMarkRewindSummary {
    pub deleted: u64,
    pub rewound: u64,
}

#[derive(Debug)]
pub struct AttributeDeletionRequestInput<'a> {
    pub profile_id: i64,
    pub category: &'a str,
    pub key: &'a str,
    pub chapter_number: i32,
    pub reason: &'a str,
    pub evidence: &'a str,
}

#[derive(Debug)]
pub struct AttributeDeletionExecutionInput<'a> {
    pub profile_id: i64,
    pub category: &'a str,
    pub key: &'a str,
    pub event_cause: &'a str,
    pub evidence: &'a str,
    pub chapter_number: i32,
}

#[derive(Debug)]
struct AttributeDeletionRecordInput<'a> {
    profile_id: i64,
    category: &'a str,
    key: &'a str,
    mark_count: i32,
    event_cause: &'a str,
    evidence: &'a str,
    chapter_number: i32,
}

pub(super) fn next_deletion_mark_count(
    current_count: i32,
    last_marked_chapter: i32,
    chapter_number: i32,
) -> i32 {
    if chapter_number == last_marked_chapter + 1 {
        current_count.max(1) + 1
    } else {
        1
    }
}

pub(super) fn retained_deletion_mark_count(
    first_chapter: i32,
    rewind_to_chapter: i32,
) -> Option<i32> {
    if rewind_to_chapter <= 0 {
        return None;
    }
    if first_chapter <= 0 {
        return None;
    }
    if first_chapter > rewind_to_chapter {
        return None;
    }
    Some(rewind_to_chapter - first_chapter + 1)
}

impl CharacterService {
    async fn upsert_deletion_mark_in<C>(
        &self,
        db: &C,
        input: AttributeDeletionRequestInput<'_>,
    ) -> Result<protagonist_deletion_mark::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeDeletionRequestInput {
            profile_id,
            category,
            key,
            chapter_number,
            reason,
            evidence,
        } = input;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let reason = require_non_empty(reason, "删除原因")?;
        let evidence = require_non_empty(evidence, "删除证据")?;
        validate_category(category)?;
        validate_chapter_number(chapter_number)?;

        let now = Utc::now().naive_utc();
        let existing = protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::Category.eq(category))
            .filter(protagonist_deletion_mark::Column::Key.eq(&key))
            .one(db)
            .await
            .map_err(AppError::Database)?;

        if let Some(mark) = existing {
            if mark.is_executed {
                protagonist_deletion_mark::Entity::delete_by_id(mark.id)
                    .exec(db)
                    .await
                    .map_err(AppError::Database)?;
            } else {
                let mut active: protagonist_deletion_mark::ActiveModel = mark.clone().into();
                let next_count = next_deletion_mark_count(
                    mark.consecutive_count,
                    mark.last_marked_chapter,
                    chapter_number,
                );
                if chapter_number != mark.last_marked_chapter + 1 {
                    active.chapter_number = Set(chapter_number);
                }
                active.consecutive_count = Set(next_count);
                active.last_marked_chapter = Set(chapter_number);
                active.reason = Set(reason.clone());
                active.evidence = Set(evidence.clone());
                active.updated_at = Set(now);
                return active.update(db).await.map_err(AppError::Database);
            }
        }

        protagonist_deletion_mark::ActiveModel {
            profile_id: Set(profile_id),
            category: Set(category.to_string()),
            key: Set(key),
            chapter_number: Set(chapter_number),
            reason: Set(reason),
            evidence: Set(evidence),
            consecutive_count: Set(1),
            is_executed: Set(false),
            last_marked_chapter: Set(chapter_number),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)
    }

    pub async fn request_attribute_deletion_in<C>(
        &self,
        db: &C,
        input: AttributeDeletionRequestInput<'_>,
    ) -> Result<protagonist_deletion_mark::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeDeletionRequestInput {
            profile_id,
            category,
            key,
            chapter_number,
            reason,
            evidence,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let reason = require_non_empty(reason, "删除原因")?;
        let evidence = require_non_empty(evidence, "删除证据")?;
        validate_category(category)?;

        let profile = require_profile_in(db, profile_id).await?;

        if attribute_value_in_state(&profile.current_state, category, &key).is_none() {
            return Err(AppError::InvalidArgument(format!(
                "无法标记不存在的角色属性: {category}.{key}"
            )));
        }

        self.upsert_deletion_mark_in(
            db,
            AttributeDeletionRequestInput {
                profile_id,
                category,
                key: &key,
                chapter_number,
                reason: &reason,
                evidence: &evidence,
            },
        )
        .await
    }

    async fn execute_attribute_deletion_in<C>(
        &self,
        db: &C,
        input: AttributeDeletionRecordInput<'_>,
    ) -> Result<attribute_event::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeDeletionRecordInput {
            profile_id,
            category,
            key,
            mark_count,
            event_cause,
            evidence,
            chapter_number,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let event_cause = require_non_empty(event_cause, "删除原因")?;
        let evidence = require_non_empty(evidence, "删除证据")?;
        validate_category(category)?;
        if mark_count <= 0 {
            return Err(AppError::InvalidArgument(
                "删除标记次数必须大于 0".to_string(),
            ));
        }

        let profile = require_profile_in(db, profile_id).await?;

        let old_value = attribute_value_in_state(&profile.current_state, category, &key)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("无法删除不存在的角色属性: {category}.{key}"))
            })?;
        let change_description = default_attribute_change_description("delete", category, &key);
        let event = self
            .record_attribute_event_in(
                db,
                AttributeEventRecordInput {
                    profile_id,
                    chapter_number,
                    category,
                    key: &key,
                    operation: "delete",
                    old_value,
                    new_value: serde_json::Value::Null,
                    change_description: &change_description,
                    cause: &event_cause,
                    evidence: &evidence,
                },
            )
            .await?;

        let updated = protagonist_deletion_mark::Entity::update_many()
            .col_expr(
                protagonist_deletion_mark::Column::IsExecuted,
                sea_orm::sea_query::Expr::value(true),
            )
            .col_expr(
                protagonist_deletion_mark::Column::ConsecutiveCount,
                sea_orm::sea_query::Expr::value(mark_count),
            )
            .col_expr(
                protagonist_deletion_mark::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
            )
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::Category.eq(category))
            .filter(protagonist_deletion_mark::Column::Key.eq(&key))
            .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        if updated.rows_affected == 0 {
            return Err(AppError::InvalidArgument(format!(
                "删除执行缺少未执行标记: {category}.{key}"
            )));
        }

        Ok(event)
    }

    pub async fn execute_ready_attribute_deletion_in<C>(
        &self,
        db: &C,
        input: AttributeDeletionExecutionInput<'_>,
    ) -> Result<(attribute_event::Model, i32), AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeDeletionExecutionInput {
            profile_id,
            category,
            key,
            event_cause,
            evidence,
            chapter_number,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let event_cause = require_non_empty(event_cause, "删除原因")?;
        let evidence = require_non_empty(evidence, "删除证据")?;
        validate_category(category)?;

        let mark = protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::Category.eq(category))
            .filter(protagonist_deletion_mark::Column::Key.eq(&key))
            .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("删除执行缺少未执行标记: {category}.{key}"))
            })?;

        if mark.consecutive_count < DELETION_THRESHOLD {
            return Err(AppError::InvalidArgument(format!(
                "未达到删除阈值，当前连续标记次数: {}/{}",
                mark.consecutive_count, DELETION_THRESHOLD
            )));
        }

        let count = mark.consecutive_count;
        let event = self
            .execute_attribute_deletion_in(
                db,
                AttributeDeletionRecordInput {
                    profile_id,
                    category,
                    key: &key,
                    mark_count: count,
                    event_cause: &event_cause,
                    evidence: &evidence,
                    chapter_number,
                },
            )
            .await?;
        Ok((event, count))
    }

    pub async fn rewind_deletion_marks_after<C>(
        &self,
        db: &C,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<DeletionMarkRewindSummary, AppError>
    where
        C: ConnectionTrait,
    {
        let marks = protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::LastMarkedChapter.gt(chapter_number))
            .all(db)
            .await
            .map_err(AppError::Database)?;

        let now = Utc::now().naive_utc();
        let mut summary = DeletionMarkRewindSummary::default();
        for mark in marks {
            if let Some(retained_count) =
                retained_deletion_mark_count(mark.chapter_number, chapter_number)
            {
                let mut active: protagonist_deletion_mark::ActiveModel = mark.into();
                active.consecutive_count = Set(retained_count);
                active.last_marked_chapter = Set(chapter_number);
                active.is_executed = Set(false);
                active.updated_at = Set(now);
                active.update(db).await.map_err(AppError::Database)?;
                summary.rewound += 1;
            } else {
                protagonist_deletion_mark::Entity::delete_by_id(mark.id)
                    .exec(db)
                    .await
                    .map_err(AppError::Database)?;
                summary.deleted += 1;
            }
        }

        Ok(summary)
    }

    pub async fn reset_deletion_marks(
        &self,
        profile_id: i64,
        category: &str,
        key: &str,
    ) -> Result<u64, AppError> {
        self.reset_deletion_marks_in(self.db.as_ref(), profile_id, category, key)
            .await
    }

    pub async fn reset_deletion_marks_in<C>(
        &self,
        db: &C,
        profile_id: i64,
        category: &str,
        key: &str,
    ) -> Result<u64, AppError>
    where
        C: ConnectionTrait,
    {
        let category = category.trim();
        let key = key.trim();
        validate_category(category)?;
        if key.is_empty() {
            return Err(AppError::InvalidArgument("属性 key 不能为空".to_string()));
        }

        protagonist_deletion_mark::Entity::delete_many()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::Category.eq(category))
            .filter(protagonist_deletion_mark::Column::Key.eq(key))
            .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
            .exec(db)
            .await
            .map(|result| result.rows_affected)
            .map_err(AppError::Database)
    }
}
