use a4n_domain::project::ProjectStatus;
use chrono::Utc;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use serde_json::json;

use super::{apply_reverse_blueprint_record, resolve_reverse_blueprint_apply_record};
use crate::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseBlueprintApplyPlanRecord,
    ReverseBlueprintApplyResult,
};

pub async fn apply_reverse_blueprint_records(
    db: &DatabaseTransaction,
    project_id: i64,
    draft: ReverseBlueprintApplyDraft,
) -> Result<ReverseBlueprintApplyResult, ImportReverseDbError> {
    let locked_sources =
        crate::blueprint::locked_blueprint_sources(db, project_id, "归纳正式蓝图").await?;
    let next_version = crate::blueprint::next_blueprint_version_record(
        db,
        project_id,
        locked_sources
            .blueprint
            .as_ref()
            .map(|blueprint| blueprint.current_version.as_str()),
    )
    .await?;
    let apply_plan = ReverseBlueprintApplyPlanRecord {
        project_total_chapters: locked_sources
            .project
            .total_chapters
            .filter(|total| *total > 0),
        existing_blueprint_total_chapters: locked_sources
            .blueprint
            .as_ref()
            .and_then(|blueprint| blueprint.total_chapters.filter(|total| *total > 0)),
        next_version,
    };
    let now = Utc::now().naive_utc();
    let record = resolve_reverse_blueprint_apply_record(&draft, &apply_plan, now)?;
    if let Some(total_chapters) = record.total_chapters {
        crate::blueprint::validate_blueprint_chapter_plan(&locked_sources, total_chapters)?;
    }
    let blueprint = apply_reverse_blueprint_record(db, project_id, &record).await?;

    crate::project::merge_blueprint_characters(db, project_id, draft.characters, now).await?;
    crate::project::merge_blueprint_relationships(db, project_id, draft.relationships, now).await?;
    crate::character::sync_character_profiles_from_blueprint(db, project_id, now)
        .await
        .map_err(|error| match error {
            crate::character::CharacterProfileSeedError::Database(error) => {
                ImportReverseDbError::Database(error)
            }
            crate::character::CharacterProfileSeedError::Internal(message) => {
                ImportReverseDbError::Internal(message)
            }
        })?;
    let characters = crate::project::blueprint_characters_for_project(db, project_id).await?;
    let relationships = crate::project::blueprint_relationships_for_project(db, project_id).await?;
    let snapshot = json!({
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
        "characters": characters,
        "relationships": relationships,
    });
    crate::blueprint::insert_blueprint_version_record(
        db,
        crate::blueprint::InsertBlueprintVersionInput {
            project_id,
            version: record.current_version.clone(),
            change_description: "依据当前正式资料归纳蓝图".to_string(),
            blueprint_data: snapshot.clone(),
            now,
            created_by: "system".to_string(),
        },
    )
    .await?;
    let project_status = ProjectStatus::parse(&locked_sources.project.status).ok_or_else(|| {
        ImportReverseDbError::InvalidArgument(format!(
            "项目状态无效: {}",
            locked_sources.project.status
        ))
    })?;
    let total_chapters = record
        .total_chapters
        .or(locked_sources.project.total_chapters);
    crate::blueprint::sync_project_after_blueprint_record(
        db,
        crate::blueprint::SyncProjectAfterBlueprintInput {
            project_id,
            title: blueprint.title.clone(),
            status: project_status,
            target_chapters: total_chapters.unwrap_or(locked_sources.project.target_chapters),
            total_chapters,
            chapters_per_part: if record.total_chapters.is_some() {
                record.chapters_per_part
            } else {
                locked_sources.project.chapters_per_part
            },
            raw_blueprint: serde_json::to_string_pretty(&snapshot).map_err(|error| {
                ImportReverseDbError::InvalidArgument(format!("序列化蓝图快照失败: {error}"))
            })?,
            now,
        },
    )
    .await?;

    Ok(ReverseBlueprintApplyResult {
        character_count: characters.len(),
        relationship_count: relationships.len(),
        version: record.current_version,
    })
}

pub async fn apply_reverse_blueprint_records_transaction<T>(
    db: &T,
    project_id: i64,
    draft: ReverseBlueprintApplyDraft,
) -> Result<ReverseBlueprintApplyResult, ImportReverseDbError>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let result = apply_reverse_blueprint_records(&txn, project_id, draft).await?;
    txn.commit().await?;
    Ok(result)
}
