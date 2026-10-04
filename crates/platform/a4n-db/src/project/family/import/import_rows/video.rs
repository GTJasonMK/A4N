use super::*;
use a4n_db_models::models::{
    ai::generated_video,
    chapter::chapter_video_prompt,
    media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD},
};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter,
    sea_query::{Alias, CaseStatement, Expr},
};

pub(super) async fn insert_video_generation_rows<Importer>(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    insert_video_prompts(txn, package, maps).await?;
    insert_generated_videos(txn, package, maps, media_importer).await?;
    for prompt in package.chapter_video_prompts.iter().filter(|prompt| {
        prompt.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
            && prompt.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
    }) {
        let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("共享片段规划缺少 metadata_json")
        })?;
        super::super::remap::collect_video_plan_fingerprints(metadata, maps)?;
    }
    Ok(())
}

pub(super) async fn update_video_generation_refs(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    update_video_prompt_refs(txn, package, maps).await?;
    update_generated_video_refs(txn, package, maps).await
}

async fn insert_video_prompts(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let prompts = package
        .chapter_video_prompts
        .iter()
        .cloned()
        .map(crate::video::chapter_video_prompt_model_from_record)
        .collect::<Vec<_>>();
    insert_mapped_import_rows!(
        txn,
        &prompts,
        prompt,
        chapter_video_prompt::ActiveModel,
        maps.chapter_video_prompt_ids,
        "导入章节视频提示词失败",
        |active| {
            if prompt.status == a4n_domain::video_prompt::STATUS_RUNNING {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "不能导入运行中的章节视频提示词: {}",
                    prompt.id
                )));
            }
            active.project_id = Set(map_required(
                &maps.project_ids,
                prompt.project_id,
                "章节视频提示词 project_id",
            )?);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                prompt.chapter_id,
                "章节视频提示词 chapter_id",
            )?);
            active.reference_image_ids_json = Set(remap_chapter_video_reference_image_ids(
                prompt.reference_image_ids_json.as_ref(),
                maps,
            )?);
            active.generated_video_id = Set(None);
            active.timeline_item_id = Set(None);
            // Keep required language metadata; remap embedded IDs before this transaction commits.
        }
    );
    Ok(())
}

async fn insert_generated_videos<Importer>(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    if package.generated_videos.is_empty() {
        return Ok(());
    }
    let rows = package
        .generated_videos
        .iter()
        .map(|video| {
            if !a4n_domain::video_workflow::is_terminal_video_status(&video.status) {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "只能导入终态生成视频: generated_video_id={}, status={}",
                    video.id, video.status
                )));
            }
            let mut active: generated_video::ActiveModel = video.clone().into();
            active.id = sea_orm::NotSet;
            active.project_id = Set(map_required(
                &maps.project_ids,
                video.project_id,
                "生成视频 project_id",
            )?);
            active.chapter_id = Set(map_optional(
                &maps.chapter_ids,
                video.chapter_id,
                "生成视频 chapter_id",
            )?);
            active.file_path = Set(None);
            active.thumbnail_path = Set(None);
            active.source_video_id = Set(None);
            active.source_image_id = Set(None);
            active.provider_job_id = Set(None);
            active.timeline_item_id = Set(None);
            active.metadata_json = Set(None);
            Ok::<_, ProjectFamilyImportError>(active)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = generated_video::Entity::insert_many(rows)
        .exec_with_returning_many(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "导入生成视频失败",
            source,
        })?;
    if inserted.len() != package.generated_videos.len() {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "导入生成视频批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            package.generated_videos.len()
        )));
    }
    for (video, inserted) in package.generated_videos.iter().zip(inserted) {
        maps.generated_video_ids.insert(video.id, inserted.id);
        if let Some(path) = import_video_file_field(
            media_importer,
            inserted.project_id,
            video.id,
            inserted.id,
            FILE_PATH_FIELD,
            video.file_path.as_deref(),
        )? {
            maps.generated_video_paths.insert(video.id, path);
        }
        if let Some(path) = import_video_file_field(
            media_importer,
            inserted.project_id,
            video.id,
            inserted.id,
            THUMBNAIL_PATH_FIELD,
            video.thumbnail_path.as_deref(),
        )? {
            maps.generated_video_thumbnail_paths.insert(video.id, path);
        }
    }
    Ok(())
}

fn import_video_file_field<Importer>(
    media_importer: &mut Importer,
    project_id: i64,
    old_video_id: i64,
    new_video_id: i64,
    field: &str,
    original_path: Option<&str>,
) -> Result<Option<String>, ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let imported = media_importer.write_imported_media_field(
        project_id,
        CanvasNodeRefKind::GeneratedVideo.as_str(),
        old_video_id,
        new_video_id,
        field,
    )?;
    if original_path.is_some() != imported.is_some() {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "生成视频媒体文件与字段不一致: generated_video_id={old_video_id}, field={field}"
        )));
    }
    Ok(imported)
}

async fn update_video_prompt_refs(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    if package.chapter_video_prompts.is_empty() {
        return Ok(());
    }
    let mut generated_video_id_case = CaseStatement::new();
    let mut metadata_case = CaseStatement::new();
    let mut visual_prompt_case = CaseStatement::new();
    let mut prompt_ids = Vec::with_capacity(package.chapter_video_prompts.len());
    for prompt in &package.chapter_video_prompts {
        let new_id = map_required(
            &maps.chapter_video_prompt_ids,
            prompt.id,
            "章节视频提示词 ID",
        )?;
        generated_video_id_case = generated_video_id_case.case(
            chapter_video_prompt::Column::Id.eq(new_id),
            Expr::value(map_optional(
                &maps.generated_video_ids,
                prompt.generated_video_id,
                "章节视频提示词 generated_video_id",
            )?),
        );
        let metadata = remap_video_metadata(prompt.metadata_json.as_ref(), maps)?;
        let visual_prompt =
            if prompt.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN {
                let segments = metadata
                    .as_ref()
                    .and_then(|value| value.get("segments"))
                    .ok_or_else(|| {
                        ProjectFamilyImportError::invalid_package("视频分段规划缺少 segments")
                    })?;
                serde_json::json!({"segments": segments}).to_string()
            } else {
                prompt.visual_prompt.clone()
            };
        visual_prompt_case = visual_prompt_case.case(
            chapter_video_prompt::Column::Id.eq(new_id),
            Expr::value(visual_prompt),
        );
        metadata_case = metadata_case.case(
            chapter_video_prompt::Column::Id.eq(new_id),
            Expr::value(metadata).cast_as(Alias::new("json")),
        );
        prompt_ids.push(new_id);
    }
    let updated = chapter_video_prompt::Entity::update_many()
        .col_expr(
            chapter_video_prompt::Column::VisualPrompt,
            visual_prompt_case
                .finally(Expr::col(chapter_video_prompt::Column::VisualPrompt))
                .into(),
        )
        .col_expr(
            chapter_video_prompt::Column::GeneratedVideoId,
            generated_video_id_case
                .finally(Expr::col(chapter_video_prompt::Column::GeneratedVideoId))
                .into(),
        )
        .col_expr(
            chapter_video_prompt::Column::TimelineItemId,
            Expr::value(Option::<i64>::None),
        )
        .col_expr(
            chapter_video_prompt::Column::MetadataJson,
            metadata_case
                .finally(Expr::col(chapter_video_prompt::Column::MetadataJson))
                .into(),
        )
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新章节视频提示词引用失败",
            source,
        })?;
    if updated.rows_affected != prompt_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新章节视频提示词引用数量不一致: updated={}, expected={}",
            updated.rows_affected,
            prompt_ids.len()
        )));
    }
    Ok(())
}

async fn update_generated_video_refs(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    if package.generated_videos.is_empty() {
        return Ok(());
    }
    let mut file_path_case = CaseStatement::new();
    let mut thumbnail_path_case = CaseStatement::new();
    let mut source_video_id_case = CaseStatement::new();
    let mut source_image_id_case = CaseStatement::new();
    let mut metadata_case = CaseStatement::new();
    let mut video_ids = Vec::with_capacity(package.generated_videos.len());
    for video in &package.generated_videos {
        let new_id = map_required(&maps.generated_video_ids, video.id, "生成视频 ID")?;
        file_path_case = file_path_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(maps.generated_video_paths.get(&video.id).cloned()),
        );
        thumbnail_path_case = thumbnail_path_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(maps.generated_video_thumbnail_paths.get(&video.id).cloned()),
        );
        source_video_id_case = source_video_id_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(map_optional(
                &maps.generated_video_ids,
                video.source_video_id,
                "生成视频 source_video_id",
            )?),
        );
        source_image_id_case = source_image_id_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(map_optional(
                &maps.generated_image_ids,
                video.source_image_id,
                "生成视频 source_image_id",
            )?),
        );
        metadata_case = metadata_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(remap_video_metadata(video.metadata_json.as_ref(), maps)?)
                .cast_as(Alias::new("json")),
        );
        video_ids.push(new_id);
    }
    let updated = generated_video::Entity::update_many()
        .col_expr(
            generated_video::Column::FilePath,
            file_path_case
                .finally(Expr::col(generated_video::Column::FilePath))
                .into(),
        )
        .col_expr(
            generated_video::Column::ThumbnailPath,
            thumbnail_path_case
                .finally(Expr::col(generated_video::Column::ThumbnailPath))
                .into(),
        )
        .col_expr(
            generated_video::Column::SourceVideoId,
            source_video_id_case
                .finally(Expr::col(generated_video::Column::SourceVideoId))
                .into(),
        )
        .col_expr(
            generated_video::Column::SourceImageId,
            source_image_id_case
                .finally(Expr::col(generated_video::Column::SourceImageId))
                .into(),
        )
        .col_expr(
            generated_video::Column::ProviderJobId,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            generated_video::Column::TimelineItemId,
            Expr::value(Option::<i64>::None),
        )
        .col_expr(
            generated_video::Column::MetadataJson,
            metadata_case
                .finally(Expr::col(generated_video::Column::MetadataJson))
                .into(),
        )
        .filter(generated_video::Column::Id.is_in(video_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新生成视频引用失败",
            source,
        })?;
    if updated.rows_affected != video_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新生成视频引用数量不一致: updated={}, expected={}",
            updated.rows_affected,
            video_ids.len()
        )));
    }
    Ok(())
}

pub(super) async fn insert_video_segments(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.video_segments,
        segment,
        video_segment::ActiveModel,
        maps.video_segment_ids,
        "导入视频片段失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                segment.project_id,
                "视频片段 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                segment.root_project_id,
                "视频片段 root_project_id",
            )?);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                segment.source_chapter_id,
                "视频片段 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                segment.source_snapshot_chapter_id,
                "视频片段 source_snapshot_chapter_id",
            )?);
            active.source_generation_spec_id = Set(map_optional(
                &maps.generation_spec_ids,
                segment.source_generation_spec_id,
                "视频片段 source_generation_spec_id",
            )?);
            active.result_media_asset_id = Set(None);
            active.previous_segment_id = Set(None);
            active.next_segment_id = Set(None);
        }
    );
    if package.video_segments.is_empty() {
        return Ok(());
    }
    let mut result_media_asset_id_case = CaseStatement::new();
    let mut previous_segment_id_case = CaseStatement::new();
    let mut next_segment_id_case = CaseStatement::new();
    let mut segment_ids = Vec::with_capacity(package.video_segments.len());
    for segment in &package.video_segments {
        let new_segment_id = map_required(&maps.video_segment_ids, segment.id, "视频片段 ID")?;
        result_media_asset_id_case = result_media_asset_id_case.case(
            video_segment::Column::Id.eq(new_segment_id),
            Expr::value(map_optional(
                &maps.media_asset_ids,
                segment.result_media_asset_id,
                "视频片段 result_media_asset_id",
            )?),
        );
        previous_segment_id_case = previous_segment_id_case.case(
            video_segment::Column::Id.eq(new_segment_id),
            Expr::value(map_optional(
                &maps.video_segment_ids,
                segment.previous_segment_id,
                "视频片段 previous_segment_id",
            )?),
        );
        next_segment_id_case = next_segment_id_case.case(
            video_segment::Column::Id.eq(new_segment_id),
            Expr::value(map_optional(
                &maps.video_segment_ids,
                segment.next_segment_id,
                "视频片段 next_segment_id",
            )?),
        );
        segment_ids.push(new_segment_id);
    }
    let updated = video_segment::Entity::update_many()
        .col_expr(
            video_segment::Column::ResultMediaAssetId,
            result_media_asset_id_case
                .finally(Expr::col(video_segment::Column::ResultMediaAssetId))
                .into(),
        )
        .col_expr(
            video_segment::Column::PreviousSegmentId,
            previous_segment_id_case
                .finally(Expr::col(video_segment::Column::PreviousSegmentId))
                .into(),
        )
        .col_expr(
            video_segment::Column::NextSegmentId,
            next_segment_id_case
                .finally(Expr::col(video_segment::Column::NextSegmentId))
                .into(),
        )
        .filter(video_segment::Column::Id.is_in(segment_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新视频片段引用失败",
            source,
        })?;
    if updated.rows_affected != segment_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新视频片段引用数量不一致: updated={}, expected={}",
            updated.rows_affected,
            segment_ids.len()
        )));
    }
    Ok(())
}

pub(super) async fn insert_assembly_rows(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.assembly_timelines,
        timeline,
        assembly_timeline::ActiveModel,
        maps.assembly_timeline_ids,
        "导入成片时间线失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                timeline.project_id,
                "成片时间线 project_id",
            )?);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                timeline.source_chapter_id,
                "成片时间线 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                timeline.source_snapshot_chapter_id,
                "成片时间线 source_snapshot_chapter_id",
            )?);
            active.preview_media_asset_id = Set(map_optional(
                &maps.media_asset_ids,
                timeline.preview_media_asset_id,
                "成片时间线 preview_media_asset_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.assembly_clips,
        clip,
        assembly_clip::ActiveModel,
        maps.assembly_clip_ids,
        "导入成片 clip 失败",
        |active| {
            active.timeline_id = Set(map_required(
                &maps.assembly_timeline_ids,
                clip.timeline_id,
                "成片 clip timeline_id",
            )?);
            active.media_asset_id = Set(map_required(
                &maps.media_asset_ids,
                clip.media_asset_id,
                "成片 clip media_asset_id",
            )?);
            active.source_segment_id = Set(map_optional(
                &maps.video_segment_ids,
                clip.source_segment_id,
                "成片 clip source_segment_id",
            )?);
        }
    );
    Ok(())
}

pub(super) async fn update_generation_spec_deferred_targets(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let mut target_entity_id_case = CaseStatement::new();
    let mut spec_ids = Vec::new();
    for spec in &package.generation_specs {
        if spec.source_context_kind.as_deref() == Some(generation_spec::SOURCE_CONTEXT_FREEFORM)
            || !matches!(spec.target_entity_kind.as_str(), "segment" | "assembly")
            || spec.target_entity_id.is_none()
        {
            continue;
        }
        let new_spec_id = map_required(&maps.generation_spec_ids, spec.id, "生成意图 ID")?;
        let new_target_id = remap_generation_target_id(spec, maps, true)?;
        target_entity_id_case = target_entity_id_case.case(
            generation_spec::Column::Id.eq(new_spec_id),
            Expr::value(new_target_id),
        );
        spec_ids.push(new_spec_id);
    }
    if spec_ids.is_empty() {
        return Ok(());
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::TargetEntityId,
            target_entity_id_case
                .finally(Expr::col(generation_spec::Column::TargetEntityId))
                .into(),
        )
        .filter(generation_spec::Column::Id.is_in(spec_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新生成意图目标引用失败",
            source,
        })?;
    if updated.rows_affected != spec_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新生成意图目标引用数量不一致: updated={}, expected={}",
            updated.rows_affected,
            spec_ids.len()
        )));
    }
    Ok(())
}
