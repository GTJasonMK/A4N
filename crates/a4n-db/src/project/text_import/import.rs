use super::*;

use super::blueprint::{
    insert_import_text_project_blueprint, insert_import_text_project_blueprint_versions,
    insert_import_text_project_characters, insert_import_text_project_relationships,
};
use super::outlines::{
    insert_import_text_project_chapter_outlines, insert_import_text_project_part_outlines,
};
use super::validation::{
    validate_import_text_project_chapter_bounds, validate_import_text_project_payload,
};

pub async fn import_text_project_record(
    db: &DatabaseConnection,
    input: ImportTextProjectRecordInput,
) -> Result<ImportTextProjectRecordResult, ImportTextProjectError> {
    let ImportTextProjectRecordInput {
        user_id,
        title,
        description,
        project_type,
        status,
        target_chapters,
        current_chapter,
        total_chapters,
        chapters_per_part,
        avatar_svg,
        avatar_animal,
        raw_blueprint,
        import_analysis_progress,
        export_config,
        payload,
    } = input;

    let project_type = normalize_project_type(&project_type)
        .map_err(|error| ImportTextProjectError::InvalidPackage(error.to_string()))?
        .to_string();
    StateValidator::parse_status(&status).map_err(|error| {
        ImportTextProjectError::InvalidPackage(format!("导入项目状态无效: {error}"))
    })?;
    validate_import_text_project_payload(&payload)?;
    let (max_chapter_number, max_available_content_chapter_number, available_content_chapter_count) =
        payload.chapter_bound_counts();
    validate_import_text_project_chapter_bounds(
        target_chapters,
        current_chapter,
        total_chapters,
        chapters_per_part,
        max_chapter_number,
        max_available_content_chapter_number,
        available_content_chapter_count,
    )?;

    let tx = db
        .begin()
        .await
        .map_err(|error| ImportTextProjectError::database("启动文本项目导入事务失败", error))?;
    let now = Utc::now().naive_utc();
    let project = novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(title),
        description: Set(description),
        project_type: Set(project_type),
        project_kind: Set(PROJECT_KIND_TEXT.to_string()),
        source_mode: Set(SOURCE_MODE_SELF.to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        derived_project_status: Set(None),
        status: Set(status),
        target_chapters: Set(target_chapters),
        current_chapter: Set(current_chapter),
        total_chapters: Set(total_chapters),
        chapters_per_part: Set(chapters_per_part),
        avatar_svg: Set(avatar_svg),
        avatar_animal: Set(avatar_animal),
        raw_blueprint: Set(raw_blueprint),
        import_analysis_progress: Set(import_analysis_progress),
        export_config: Set(export_config),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(|error| ImportTextProjectError::database("导入项目失败", error))?;

    let part_id_map =
        insert_import_text_project_part_outlines(&tx, project.id, &payload.part_outlines, now)
            .await?;
    let outline_id_map = insert_import_text_project_chapter_outlines(
        &tx,
        project.id,
        &payload.chapter_outlines,
        &part_id_map,
        now,
    )
    .await?;
    if let Some(blueprint) = &payload.blueprint {
        insert_import_text_project_blueprint(&tx, project.id, blueprint, now).await?;
    }
    insert_import_text_project_blueprint_versions(
        &tx,
        project.id,
        &payload.blueprint_versions,
        now,
    )
    .await?;
    insert_import_text_project_characters(&tx, project.id, &payload.characters, now).await?;
    insert_import_text_project_relationships(&tx, project.id, &payload.relationships, now).await?;

    let mut postprocess_chapter_ids = Vec::new();
    for import_chapter in &payload.chapters {
        let chapter_label = format!("第{}章", import_chapter.chapter_number);
        let outline_id = import_chapter
            .old_outline_id
            .map(|old_id| {
                outline_id_map.get(&old_id).copied().ok_or_else(|| {
                    ImportTextProjectError::InvalidPackage(format!(
                        "{chapter_label}引用的章节大纲不存在: outline_id={old_id}"
                    ))
                })
            })
            .transpose()?;
        let chapter = chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(outline_id),
            chapter_number: Set(import_chapter.chapter_number),
            title: Set(import_chapter.title.clone()),
            content: Set(import_chapter.content.clone()),
            word_count: Set(import_chapter.word_count),
            status: Set(import_chapter.status.clone()),
            summary: Set(import_chapter.summary.clone()),
            style_tags: Set(import_chapter.style_tags.clone()),
            analysis_data: Set(import_chapter.analysis_data.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|error| ImportTextProjectError::database("导入章节失败", error))?;
        postprocess_chapter_ids.push(chapter.id);
    }

    tx.commit()
        .await
        .map_err(|error| ImportTextProjectError::database("提交文本项目导入事务失败", error))?;
    let has_preserved_chapter_tags = payload.has_preserved_chapter_tags();
    Ok(ImportTextProjectRecordResult {
        project,
        postprocess_chapter_ids,
        has_preserved_chapter_tags,
    })
}
