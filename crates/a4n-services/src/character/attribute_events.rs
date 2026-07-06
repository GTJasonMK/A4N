use a4n_core::error::AppError;
use a4n_core::models::character::{
    attribute_event, protagonist_deletion_mark, protagonist_profile,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use super::state::{
    apply_attribute_to_state, apply_profile_current_state, attribute_value_in_state,
};
use super::{
    CharacterService, default_attribute_change_description, require_non_empty, require_profile_in,
    validate_category, validate_chapter_number, validate_operation,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeHistoryFilter {
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub category: Option<String>,
    pub key: Option<String>,
    pub operation: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug)]
pub struct AttributeValueChangeInput<'a> {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: &'a str,
    pub key: &'a str,
    pub value: serde_json::Value,
    pub change_description: Option<&'a str>,
    pub cause: &'a str,
    pub evidence: &'a str,
}

#[derive(Debug)]
pub(super) struct AttributeEventRecordInput<'a> {
    pub(super) profile_id: i64,
    pub(super) chapter_number: i32,
    pub(super) category: &'a str,
    pub(super) key: &'a str,
    pub(super) operation: &'a str,
    pub(super) old_value: serde_json::Value,
    pub(super) new_value: serde_json::Value,
    pub(super) change_description: &'a str,
    pub(super) cause: &'a str,
    pub(super) evidence: &'a str,
}

pub(super) fn validate_attribute_history_filter(
    filter: &AttributeHistoryFilter,
) -> Result<(), AppError> {
    if let Some(chapter_from) = filter.chapter_from {
        validate_chapter_number(chapter_from)?;
    }
    if let Some(chapter_to) = filter.chapter_to {
        validate_chapter_number(chapter_to)?;
    }
    if let (Some(chapter_from), Some(chapter_to)) = (filter.chapter_from, filter.chapter_to)
        && chapter_from > chapter_to
    {
        return Err(AppError::InvalidArgument(
            "属性事件起始章节不能大于结束章节".to_string(),
        ));
    }
    if let Some(category) = filter.category.as_deref() {
        validate_category(category)?;
    }
    if let Some(key) = filter.key.as_deref() {
        require_non_empty(key, "属性 key")?;
    }
    if let Some(operation) = filter.operation.as_deref() {
        validate_operation(operation)?;
    }
    if let Some(limit) = filter.limit
        && !(1..=300).contains(&limit)
    {
        return Err(AppError::InvalidArgument(
            "属性事件 limit 必须在 1..=300 之间".to_string(),
        ));
    }
    Ok(())
}

impl CharacterService {
    pub(super) async fn record_attribute_event_in<C>(
        &self,
        db: &C,
        input: AttributeEventRecordInput<'_>,
    ) -> Result<attribute_event::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeEventRecordInput {
            profile_id,
            chapter_number,
            category,
            key,
            operation,
            old_value,
            new_value,
            change_description,
            cause,
            evidence,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let operation = operation.trim();
        let change_description = require_non_empty(change_description, "属性变更摘要")?;
        let cause = require_non_empty(cause, "属性变更原因")?;
        let evidence = require_non_empty(evidence, "属性变更证据")?;
        validate_category(category)?;
        validate_operation(operation)?;

        let now = Utc::now().naive_utc();
        let profile = require_profile_in(db, profile_id).await?;
        let event = attribute_event::ActiveModel {
            profile_id: Set(profile_id),
            chapter_number: Set(chapter_number),
            timestamp: Set(now),
            category: Set(category.to_string()),
            key: Set(key.clone()),
            operation: Set(operation.to_string()),
            change_description: Set(change_description),
            old_value: Set(old_value),
            new_value: Set(new_value),
            cause: Set(cause),
            evidence: Set(evidence),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;

        let mut next_state = profile.current_state.clone();
        apply_attribute_to_state(
            &mut next_state,
            category,
            &key,
            operation,
            event.new_value.clone(),
        )?;
        let mut active: protagonist_profile::ActiveModel = profile.into();
        apply_profile_current_state(&mut active, next_state);
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;

        if matches!(operation, "add" | "modify") {
            protagonist_deletion_mark::Entity::delete_many()
                .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
                .filter(protagonist_deletion_mark::Column::Category.eq(category))
                .filter(protagonist_deletion_mark::Column::Key.eq(&key))
                .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
                .exec(db)
                .await
                .map_err(AppError::Database)?;
        }

        Ok(event)
    }

    pub async fn add_attribute_in<C>(
        &self,
        db: &C,
        input: AttributeValueChangeInput<'_>,
    ) -> Result<attribute_event::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeValueChangeInput {
            profile_id,
            chapter_number,
            category,
            key,
            value,
            change_description,
            cause,
            evidence,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let cause = require_non_empty(cause, "属性变更原因")?;
        let evidence = require_non_empty(evidence, "属性变更证据")?;
        validate_category(category)?;

        let profile = require_profile_in(db, profile_id).await?;

        if attribute_value_in_state(&profile.current_state, category, &key).is_some() {
            return Err(AppError::InvalidArgument(format!(
                "属性 {category}.{key} 已存在，请使用修改操作"
            )));
        }

        let default_description;
        let change_description = if let Some(value) = change_description {
            require_non_empty(value, "属性变更摘要")?
        } else {
            default_description = default_attribute_change_description("add", category, &key);
            default_description
        };

        self.record_attribute_event_in(
            db,
            AttributeEventRecordInput {
                profile_id,
                chapter_number,
                category,
                key: &key,
                operation: "add",
                old_value: serde_json::Value::Null,
                new_value: value,
                change_description: &change_description,
                cause: &cause,
                evidence: &evidence,
            },
        )
        .await
    }

    pub async fn modify_attribute_in<C>(
        &self,
        db: &C,
        input: AttributeValueChangeInput<'_>,
    ) -> Result<attribute_event::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let AttributeValueChangeInput {
            profile_id,
            chapter_number,
            category,
            key,
            value,
            change_description,
            cause,
            evidence,
        } = input;
        validate_chapter_number(chapter_number)?;
        let category = category.trim();
        let key = require_non_empty(key, "属性 key")?;
        let cause = require_non_empty(cause, "属性变更原因")?;
        let evidence = require_non_empty(evidence, "属性变更证据")?;
        validate_category(category)?;

        let profile = require_profile_in(db, profile_id).await?;
        let old_value = attribute_value_in_state(&profile.current_state, category, &key)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("属性 {category}.{key} 不存在，请使用新增操作"))
            })?;

        let default_description;
        let change_description = if let Some(value) = change_description {
            require_non_empty(value, "属性变更摘要")?
        } else {
            default_description = default_attribute_change_description("modify", category, &key);
            default_description
        };

        self.record_attribute_event_in(
            db,
            AttributeEventRecordInput {
                profile_id,
                chapter_number,
                category,
                key: &key,
                operation: "modify",
                old_value,
                new_value: value,
                change_description: &change_description,
                cause: &cause,
                evidence: &evidence,
            },
        )
        .await
    }

    pub async fn attribute_history(
        &self,
        profile_id: i64,
        filter: AttributeHistoryFilter,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        validate_attribute_history_filter(&filter)?;

        let mut query = attribute_event::Entity::find()
            .filter(attribute_event::Column::ProfileId.eq(profile_id));

        if let Some(chapter_from) = filter.chapter_from {
            query = query.filter(attribute_event::Column::ChapterNumber.gte(chapter_from));
        }
        if let Some(chapter_to) = filter.chapter_to {
            query = query.filter(attribute_event::Column::ChapterNumber.lte(chapter_to));
        }
        if let Some(category) = filter.category {
            query = query.filter(attribute_event::Column::Category.eq(category));
        }
        if let Some(key) = filter.key {
            query = query.filter(attribute_event::Column::Key.eq(key.trim().to_string()));
        }
        if let Some(operation) = filter.operation {
            query = query.filter(attribute_event::Column::Operation.eq(operation));
        }
        if let Some(limit) = filter.limit {
            query = query.limit(limit);
        }

        query
            .order_by_asc(attribute_event::Column::ChapterNumber)
            .order_by_asc(attribute_event::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}
