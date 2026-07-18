use crate::galgame::{
    GalgameMediaRecordError, GalgameTargetContext, MEDIA_KIND_IMAGE, MEDIA_STATUS_COMPLETED,
    PROJECT_KIND_GALGAME,
};
use a4n_core::models::{generation::generation_spec, media::media_asset, project::novel_project};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};

pub fn validate_galgame_media_asset_source_context(
    asset: &media_asset::Model,
    context: &GalgameTargetContext,
) -> Result<(), GalgameMediaRecordError> {
    match asset.source_snapshot_id {
        Some(source_snapshot_id) if source_snapshot_id == context.source_snapshot_id => {}
        Some(source_snapshot_id) => {
            return Err(GalgameMediaRecordError::InvalidArgument(format!(
                "媒体资产 {} 的源快照与目标不一致: asset_source_snapshot_id={}, target_source_snapshot_id={}",
                asset.id, source_snapshot_id, context.source_snapshot_id
            )));
        }
        None => {
            return Err(GalgameMediaRecordError::InvalidArgument(format!(
                "媒体资产 {} 缺少 source_snapshot_id，不能绑定到 Galgame snapshot-backed 目标",
                asset.id
            )));
        }
    }
    if let Some(source_chapter_id) = asset.source_chapter_id
        && source_chapter_id != context.source_chapter_id
    {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 的源章节与目标不一致: asset_source_chapter_id={}, target_source_chapter_id={}",
            asset.id, source_chapter_id, context.source_chapter_id
        )));
    }
    if asset.source_chapter_id.is_some() != asset.source_snapshot_chapter_id.is_some() {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 的源章节上下文不完整: source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空",
            asset.id
        )));
    }
    if let Some(snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && snapshot_chapter_id != context.source_snapshot_chapter_id
    {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 的源快照章节与目标不一致: asset_source_snapshot_chapter_id={}, target_source_snapshot_chapter_id={}",
            asset.id, snapshot_chapter_id, context.source_snapshot_chapter_id
        )));
    }
    Ok(())
}

pub(in crate::galgame::media) async fn galgame_bindable_media_asset<C>(
    db: &C,
    project_id: i64,
    media_asset_id: i64,
    required_media_kind: &str,
    target_context: Option<&GalgameTargetContext>,
) -> Result<media_asset::Model, GalgameMediaRecordError>
where
    C: ConnectionTrait,
{
    let use_row_lock = db.get_database_backend() != DbBackend::Sqlite;
    let mut project_query = novel_project::Entity::find_by_id(project_id);
    if use_row_lock {
        project_query = project_query.lock_exclusive();
    }
    let project = project_query
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
        .ok_or_else(|| GalgameMediaRecordError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "项目不是 Galgame 派生项目: {}",
            project.id
        )));
    }
    let root_project_id = project.root_project_id.ok_or_else(|| {
        GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 项目缺少根文本项目: {}",
            project.id
        ))
    })?;
    let mut asset_query = media_asset::Entity::find_by_id(media_asset_id);
    if use_row_lock {
        asset_query = asset_query.lock_exclusive();
    }
    let asset = asset_query
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
        .ok_or_else(|| {
            GalgameMediaRecordError::NotFound(format!("媒体资产不存在: {media_asset_id}"))
        })?;
    if asset.project_id != project.id || asset.root_project_id != root_project_id {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 不属于当前 Galgame 项目族",
            asset.id
        )));
    }
    if asset.media_kind != required_media_kind {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 类型不匹配: 需要 {}, 实际 {}",
            asset.id, required_media_kind, asset.media_kind
        )));
    }
    if asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none() {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "媒体资产 {} 不是可绑定的已完成媒体",
            asset.id
        )));
    }
    if let Some(context) = target_context {
        validate_galgame_media_asset_source_context(&asset, context)?;
    }
    Ok(asset)
}

pub(in crate::galgame) fn validate_completed_visual_asset(
    spec: &generation_spec::Model,
    asset: &media_asset::Model,
) -> Result<(), GalgameMediaRecordError> {
    if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 视觉生成意图已完成但绑定媒体项目族不一致: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        )));
    }
    if asset.media_kind != MEDIA_KIND_IMAGE {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 视觉生成意图已完成但绑定媒体类型不是 image: spec_id={}, media_asset_id={}, media_kind={}",
            spec.id, asset.id, asset.media_kind
        )));
    }
    if asset.status != MEDIA_STATUS_COMPLETED {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 视觉生成意图已完成但绑定媒体未完成: spec_id={}, media_asset_id={}, status={}",
            spec.id, asset.id, asset.status
        )));
    }
    Ok(())
}
