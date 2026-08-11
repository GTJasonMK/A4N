use a4n_core::{
    blueprint::{increment_blueprint_version, project_status_after_blueprint_generation},
    models::{
        chapter::part_outline,
        project::{blueprint_version, novel_blueprint},
    },
};
use a4n_domain::project::ProjectStatus;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde_json::{Value, json};

use super::{
    BlueprintDbError, BlueprintMutationStatusChange, BlueprintRefinePatchInput,
    InsertBlueprintVersionInput, ManualBlueprintUpsertInput, SaveManualBlueprintInput,
    SyncProjectAfterBlueprintInput, UpdateBlueprintWorldSettingInput,
    VersionedBlueprintMutationResult, apply_blueprint_refine_patch_record,
    insert_blueprint_version_record, sync_project_after_blueprint_record,
    upsert_manual_blueprint_record,
};
use crate::{
    generation::PROJECT_GATE_ACTIVE,
    outline::{OutlineGenerationSourceRows, load_outline_generation_source_rows_for_update},
    project::{
        CHAPTERS_PER_PART, PROJECT_KIND_TEXT, ProjectStatusHistoryInput, ProjectStatusWriteError,
        insert_project_status_history, needs_part_outlines_for_chapters,
    },
};

const MAX_BLUEPRINT_TOTAL_CHAPTERS: i32 = 10_000;

pub async fn save_manual_blueprint_records_transaction<T>(
    db: &T,
    input: SaveManualBlueprintInput,
) -> Result<VersionedBlueprintMutationResult, BlueprintDbError>
where
    T: TransactionTrait,
{
    validate_project_id(input.project_id)?;
    let input = normalize_manual_input(input)?;
    let txn = db.begin().await?;
    let rows = locked_blueprint_sources(&txn, input.project_id, "保存正式蓝图").await?;
    validate_blueprint_chapter_plan(&rows, input.total_chapters)?;

    let now = Utc::now().naive_utc();
    let next_version = next_blueprint_version_record(
        &txn,
        input.project_id,
        rows.blueprint
            .as_ref()
            .map(|blueprint| blueprint.current_version.as_str()),
    )
    .await?;
    let needs_part_outlines = needs_part_outlines_for_chapters(input.total_chapters);
    let blueprint = upsert_manual_blueprint_record(
        &txn,
        ManualBlueprintUpsertInput {
            project_id: input.project_id,
            title: rows.project.title.clone(),
            genre: input.genre,
            style: input.style,
            tone: input.tone,
            target_audience: input.target_audience,
            one_sentence_summary: input.one_sentence_summary,
            full_synopsis: input.full_synopsis,
            needs_part_outlines,
            total_chapters: input.total_chapters,
            chapters_per_part: needs_part_outlines.then_some(CHAPTERS_PER_PART),
            current_version: next_version.clone(),
            now,
        },
        rows.blueprint.clone(),
    )
    .await?;
    let snapshot = blueprint_snapshot(&blueprint, &rows);
    let status_change = persist_blueprint_version_and_project(
        &txn,
        &rows,
        &blueprint,
        next_version,
        "用户保存正式蓝图",
        snapshot,
        input.created_by,
        now,
    )
    .await?;
    txn.commit().await?;

    Ok(VersionedBlueprintMutationResult {
        blueprint,
        status_change,
    })
}

pub async fn update_blueprint_world_setting_records_transaction<T>(
    db: &T,
    input: UpdateBlueprintWorldSettingInput,
) -> Result<VersionedBlueprintMutationResult, BlueprintDbError>
where
    T: TransactionTrait,
{
    validate_project_id(input.project_id)?;
    if input.world_setting.is_null() {
        return Err(BlueprintDbError::InvalidArgument(
            "world_setting 不能为 null".to_string(),
        ));
    }
    let txn = db.begin().await?;
    let rows = locked_blueprint_sources(&txn, input.project_id, "保存世界观").await?;
    let existing = rows
        .blueprint
        .clone()
        .ok_or_else(|| BlueprintDbError::NotFound("蓝图不存在".to_string()))?;
    let now = Utc::now().naive_utc();
    let next_version = next_blueprint_version_record(
        &txn,
        input.project_id,
        Some(existing.current_version.as_str()),
    )
    .await?;
    let blueprint = apply_blueprint_refine_patch_record(
        &txn,
        existing,
        BlueprintRefinePatchInput {
            style: None,
            full_synopsis: None,
            world_setting: Some(input.world_setting),
            current_version: next_version.clone(),
            now,
        },
    )
    .await?;
    let snapshot = blueprint_snapshot(&blueprint, &rows);
    let status_change = persist_blueprint_version_and_project(
        &txn,
        &rows,
        &blueprint,
        next_version,
        "用户更新世界观",
        snapshot,
        input.created_by,
        now,
    )
    .await?;
    txn.commit().await?;

    Ok(VersionedBlueprintMutationResult {
        blueprint,
        status_change,
    })
}

pub(crate) async fn locked_blueprint_sources<C>(
    db: &C,
    project_id: i64,
    operation: &str,
) -> Result<OutlineGenerationSourceRows, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let rows = load_outline_generation_source_rows_for_update(db, project_id)
        .await?
        .ok_or_else(|| BlueprintDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if rows.project.lifecycle_gate != PROJECT_GATE_ACTIVE {
        return Err(BlueprintDbError::InvalidArgument(format!(
            "项目当前状态不允许{operation}: {}",
            rows.project.lifecycle_gate
        )));
    }
    if rows.project.project_kind != PROJECT_KIND_TEXT {
        return Err(BlueprintDbError::InvalidArgument(
            "只有文本创作项目可以保存故事蓝图".to_string(),
        ));
    }
    Ok(rows)
}

fn normalize_manual_input(
    mut input: SaveManualBlueprintInput,
) -> Result<SaveManualBlueprintInput, BlueprintDbError> {
    input.genre = required_text(input.genre, "蓝图题材", 256)?;
    input.style = optional_text(input.style, "写作风格", 1024)?;
    input.tone = optional_text(input.tone, "情感基调", 1024)?;
    input.target_audience = optional_text(input.target_audience, "目标读者", 1024)?;
    input.one_sentence_summary = optional_text(input.one_sentence_summary, "一句话概要", 4096)?;
    input.full_synopsis = optional_text(input.full_synopsis, "完整梗概", 131_072)?;
    if !(1..=MAX_BLUEPRINT_TOTAL_CHAPTERS).contains(&input.total_chapters) {
        return Err(BlueprintDbError::InvalidArgument(format!(
            "总章节数必须是 1 到 {MAX_BLUEPRINT_TOTAL_CHAPTERS} 之间的整数"
        )));
    }
    Ok(input)
}

pub fn validate_blueprint_chapter_plan(
    rows: &OutlineGenerationSourceRows,
    total_chapters: i32,
) -> Result<(), BlueprintDbError> {
    let content_chapter = max_positive_chapter(
        rows.chapters
            .iter()
            .filter(|chapter| chapter.has_available_content())
            .map(|chapter| chapter.chapter_number),
        "正文",
    )?;
    let outline_chapter = max_positive_chapter(
        rows.chapter_outlines
            .iter()
            .map(|outline| outline.chapter_number),
        "章节大纲",
    )?;
    let part_chapter = max_part_end_chapter(&rows.parts)?;
    let required = content_chapter.max(outline_chapter).max(part_chapter);
    if total_chapters < required {
        return Err(BlueprintDbError::InvalidArgument(format!(
            "总章节数不能小于现有正式资料覆盖的第 {required} 章（正文最高 {content_chapter}，章节大纲最高 {outline_chapter}，分部范围最高 {part_chapter}）"
        )));
    }
    Ok(())
}

fn max_positive_chapter(
    chapters: impl Iterator<Item = i32>,
    label: &str,
) -> Result<i32, BlueprintDbError> {
    let mut maximum = 0;
    for chapter in chapters {
        if chapter <= 0 {
            return Err(BlueprintDbError::InvalidArgument(format!(
                "已有{label}章节号无效: {chapter}"
            )));
        }
        maximum = maximum.max(chapter);
    }
    Ok(maximum)
}

fn max_part_end_chapter(parts: &[part_outline::Model]) -> Result<i32, BlueprintDbError> {
    let mut seen_parts = std::collections::BTreeSet::new();
    let mut ranges = Vec::with_capacity(parts.len());
    for part in parts {
        if part.part_number <= 0 || !seen_parts.insert(part.part_number) {
            return Err(BlueprintDbError::InvalidArgument(format!(
                "已有分部编号无效或重复: {}",
                part.part_number
            )));
        }
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(BlueprintDbError::InvalidArgument(format!(
                "已有第 {} 部章节范围无效: {}..={}",
                part.part_number, part.start_chapter, part.end_chapter
            )));
        }
        ranges.push((part.start_chapter, part.end_chapter, part.part_number));
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        if pair[1].0 <= pair[0].1 {
            return Err(BlueprintDbError::InvalidArgument(format!(
                "已有分部章节范围重叠：第 {} 部 {}..={} 与第 {} 部 {}..={}",
                pair[0].2, pair[0].0, pair[0].1, pair[1].2, pair[1].0, pair[1].1
            )));
        }
    }
    Ok(parts.iter().map(|part| part.end_chapter).max().unwrap_or(0))
}

pub async fn next_blueprint_version_record<C>(
    db: &C,
    project_id: i64,
    current_version: Option<&str>,
) -> Result<String, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let mut candidate = match current_version {
        Some(version) => increment_blueprint_version(version)
            .map_err(|error| BlueprintDbError::InvalidArgument(error.to_string()))?,
        None => "1.0".to_string(),
    };
    loop {
        let exists = blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .filter(blueprint_version::Column::Version.eq(&candidate))
            .one(db)
            .await?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
        candidate = increment_blueprint_version(&candidate)
            .map_err(|error| BlueprintDbError::InvalidArgument(error.to_string()))?;
    }
}

fn blueprint_snapshot(
    blueprint: &novel_blueprint::Model,
    rows: &OutlineGenerationSourceRows,
) -> Value {
    json!({
        "title": blueprint.title,
        "genre": blueprint.genre,
        "style": blueprint.style,
        "tone": blueprint.tone,
        "target_audience": blueprint.target_audience,
        "one_sentence_summary": blueprint.one_sentence_summary,
        "full_synopsis": blueprint.full_synopsis,
        "world_setting": blueprint.world_setting,
        "needs_part_outlines": blueprint.needs_part_outlines,
        "total_chapters": blueprint.total_chapters,
        "chapters_per_part": blueprint.chapters_per_part,
        "avatar_svg": blueprint.avatar_svg,
        "avatar_animal": blueprint.avatar_animal,
        "characters": rows.blueprint_characters,
        "relationships": rows.blueprint_relationships,
    })
}

#[allow(clippy::too_many_arguments)]
async fn persist_blueprint_version_and_project<C>(
    db: &C,
    rows: &OutlineGenerationSourceRows,
    blueprint: &novel_blueprint::Model,
    version: String,
    change_description: &str,
    snapshot: Value,
    created_by: Option<String>,
    now: chrono::NaiveDateTime,
) -> Result<Option<BlueprintMutationStatusChange>, BlueprintDbError>
where
    C: ConnectionTrait,
{
    insert_blueprint_version_record(
        db,
        InsertBlueprintVersionInput {
            project_id: blueprint.project_id,
            version,
            change_description: Some(change_description.to_string()),
            blueprint_data: snapshot.clone(),
            now,
            created_by: created_by.clone(),
        },
    )
    .await?;

    let previous_status = ProjectStatus::parse(&rows.project.status).ok_or_else(|| {
        BlueprintDbError::InvalidArgument(format!("项目状态无效: {}", rows.project.status))
    })?;
    let next_status = project_status_after_blueprint_generation(previous_status);
    let total_chapters = blueprint
        .total_chapters
        .ok_or_else(|| BlueprintDbError::InvalidArgument("蓝图缺少 total_chapters".to_string()))?;
    sync_project_after_blueprint_record(
        db,
        SyncProjectAfterBlueprintInput {
            project_id: blueprint.project_id,
            title: blueprint.title.clone(),
            status: next_status,
            target_chapters: total_chapters,
            total_chapters: Some(total_chapters),
            chapters_per_part: blueprint.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&snapshot).map_err(|error| {
                BlueprintDbError::InvalidArgument(format!("序列化蓝图快照失败: {error}"))
            })?,
            now,
        },
    )
    .await?;

    if previous_status == next_status {
        return Ok(None);
    }
    insert_project_status_history(
        db,
        ProjectStatusHistoryInput {
            project_id: blueprint.project_id,
            from_status: previous_status,
            to_status: next_status,
            reason: Some(change_description.to_string()),
            changed_by: created_by,
            changed_at: now,
        },
    )
    .await
    .map_err(map_status_write_error)?;
    Ok(Some(BlueprintMutationStatusChange {
        from: previous_status,
        to: next_status,
        changed_at: now,
    }))
}

fn validate_project_id(project_id: i64) -> Result<(), BlueprintDbError> {
    if project_id <= 0 {
        return Err(BlueprintDbError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

fn required_text(value: String, label: &str, max_chars: usize) -> Result<String, BlueprintDbError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(BlueprintDbError::InvalidArgument(format!(
            "{label}不能为空"
        )));
    }
    validate_text_length(&value, label, max_chars)?;
    Ok(value)
}

fn optional_text(
    value: Option<String>,
    label: &str,
    max_chars: usize,
) -> Result<Option<String>, BlueprintDbError> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            validate_text_length(&value, label, max_chars)?;
            Ok(value)
        })
        .transpose()
}

fn validate_text_length(
    value: &str,
    label: &str,
    max_chars: usize,
) -> Result<(), BlueprintDbError> {
    let chars = value.chars().count();
    if chars > max_chars {
        return Err(BlueprintDbError::InvalidArgument(format!(
            "{label}不能超过 {max_chars} 个字符，当前为 {chars} 个字符"
        )));
    }
    Ok(())
}

fn map_status_write_error(error: ProjectStatusWriteError) -> BlueprintDbError {
    match error {
        ProjectStatusWriteError::InvalidArgument(message) => {
            BlueprintDbError::InvalidArgument(message)
        }
        ProjectStatusWriteError::ProjectNotFound { project_id } => {
            BlueprintDbError::NotFound(format!("项目不存在: {project_id}"))
        }
        ProjectStatusWriteError::StatusChanged { expected, actual } => {
            BlueprintDbError::InvalidArgument(format!(
                "项目状态已变化，请刷新后重试: expected={expected}, actual={actual}"
            ))
        }
        ProjectStatusWriteError::Database { source } => BlueprintDbError::Database(source),
    }
}

#[cfg(test)]
mod tests {
    use super::{max_part_end_chapter, normalize_manual_input};
    use crate::blueprint::SaveManualBlueprintInput;

    #[test]
    fn manual_input_trims_optional_fields_and_accepts_single_chapter() {
        let input = normalize_manual_input(SaveManualBlueprintInput {
            project_id: 1,
            genre: "  悬疑  ".to_string(),
            style: Some("  冷峻  ".to_string()),
            tone: Some("   ".to_string()),
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: None,
            total_chapters: 1,
            created_by: Some("user".to_string()),
        })
        .unwrap();

        assert_eq!(input.genre, "悬疑");
        assert_eq!(input.style.as_deref(), Some("冷峻"));
        assert_eq!(input.tone, None);
    }

    #[test]
    fn part_range_validation_accepts_empty_project() {
        assert_eq!(max_part_end_chapter(&[]).unwrap(), 0);
    }
}
