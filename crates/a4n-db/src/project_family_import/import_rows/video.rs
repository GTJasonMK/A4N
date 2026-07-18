use super::*;

pub(super) async fn insert_video_segments(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
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
    for segment in &package.video_segments {
        let new_segment_id = map_required(&maps.video_segment_ids, segment.id, "视频片段 ID")?;
        let stored = video_segment::Entity::find_by_id(new_segment_id)
            .one(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "读取导入视频片段失败",
                source,
            })?
            .ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(format!(
                    "导入视频片段不存在: {new_segment_id}"
                ))
            })?;
        let mut active: video_segment::ActiveModel = stored.into();
        active.result_media_asset_id = Set(map_optional(
            &maps.media_asset_ids,
            segment.result_media_asset_id,
            "视频片段 result_media_asset_id",
        )?);
        active.previous_segment_id = Set(map_optional(
            &maps.video_segment_ids,
            segment.previous_segment_id,
            "视频片段 previous_segment_id",
        )?);
        active.next_segment_id = Set(map_optional(
            &maps.video_segment_ids,
            segment.next_segment_id,
            "视频片段 next_segment_id",
        )?);
        active
            .update(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新视频片段引用失败",
                source,
            })?;
    }
    Ok(())
}

pub(super) async fn insert_assembly_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
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
    package: &ProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    for spec in &package.generation_specs {
        if !matches!(spec.target_entity_kind.as_str(), "segment" | "assembly")
            || spec.target_entity_id.is_none()
        {
            continue;
        }
        let new_spec_id = map_required(&maps.generation_spec_ids, spec.id, "生成意图 ID")?;
        let new_target_id = remap_generation_target_id(spec, maps, true)?;
        let stored = generation_spec::Entity::find_by_id(new_spec_id)
            .one(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "读取导入生成意图失败",
                source,
            })?
            .ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(format!(
                    "导入生成意图不存在: {new_spec_id}"
                ))
            })?;
        let mut active: generation_spec::ActiveModel = stored.into();
        active.target_entity_id = Set(new_target_id);
        active
            .update(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新生成意图目标引用失败",
                source,
            })?;
    }
    Ok(())
}
