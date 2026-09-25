use super::contracts::ManifestIds;
use a4n_db_models::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_domain::CanvasNodeRefKind;
use a4n_domain::canvas_transfer_runtime_identity_key;

use crate::media::{
    MEDIA_STATUS_COMPLETED, is_valid_media_kind, is_valid_media_origin, is_valid_media_status,
};
use crate::project::{ImportMediaPayload, VideoStudioProjectPackage};

use super::super::media::{require_media_payload, validate_optional_file_size};
use super::super::provenance::{
    MediaProvenanceSnapshotRefs, validate_media_provenance_snapshot,
    validate_media_provenance_source_external_path,
};

pub(super) fn validate_asset_records(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
    ids: &ManifestIds,
) -> Result<(), String> {
    for image in &package.generated_images {
        if image.project_id != package.project.id || image.chapter_id.is_some() {
            return Err(format!(
                "生成图片不是项目级画布资产: generated_image_id={}",
                image.id
            ));
        }
        if let Some(key) = image
            .metadata_json
            .as_ref()
            .and_then(canvas_transfer_runtime_identity_key)
        {
            return Err(format!(
                "视频画布项目包生成图片 metadata_json 不能携带运行身份 {key}: generated_image_id={}",
                image.id
            ));
        }
        validate_optional_file_size(image.file_size, "generated_image", image.id)?;
        require_media_payload(
            media_payload,
            CanvasNodeRefKind::GeneratedImage.as_str(),
            image.id,
            FILE_PATH_FIELD,
        )?;
    }
    for video in &package.generated_videos {
        if video.project_id != package.project.id || video.chapter_id.is_some() {
            return Err(format!(
                "生成视频不是项目级画布资产: generated_video_id={}",
                video.id
            ));
        }
        if !a4n_domain::video_workflow::is_terminal_video_status(&video.status) {
            return Err(format!(
                "视频画布项目包只能传输终态生成视频: generated_video_id={}, status={}",
                video.id, video.status
            ));
        }
        if video.provider_job_id.is_some() {
            return Err(format!(
                "视频画布项目包生成视频不能携带 provider_job_id: generated_video_id={}",
                video.id
            ));
        }
        if let Some(key) = video
            .metadata_json
            .as_ref()
            .and_then(canvas_transfer_runtime_identity_key)
        {
            return Err(format!(
                "视频画布项目包生成视频 metadata_json 不能携带运行身份 {key}: generated_video_id={}",
                video.id
            ));
        }
        validate_optional_file_size(video.file_size, "generated_video", video.id)?;
        if let Some(source_video_id) = video.source_video_id
            && !ids.video_ids.contains(&source_video_id)
        {
            return Err(format!(
                "生成视频引用了不存在的源视频: generated_video_id={}, source_video_id={source_video_id}",
                video.id
            ));
        }
        if let Some(source_image_id) = video.source_image_id
            && !ids.image_ids.contains(&source_image_id)
        {
            return Err(format!(
                "生成视频引用了不存在的源图片: generated_video_id={}, source_image_id={source_image_id}",
                video.id
            ));
        }
        if video.file_path.is_some() {
            require_media_payload(
                media_payload,
                CanvasNodeRefKind::GeneratedVideo.as_str(),
                video.id,
                FILE_PATH_FIELD,
            )?;
        }
        if video.thumbnail_path.is_some() {
            require_media_payload(
                media_payload,
                CanvasNodeRefKind::GeneratedVideo.as_str(),
                video.id,
                THUMBNAIL_PATH_FIELD,
            )?;
        }
    }
    for asset in &package.media_assets {
        validate_canvas_media_asset_owner(asset, package.project.id)?;
        validate_optional_file_size(asset.file_size, "media_asset", asset.id)?;
        if !is_valid_media_kind(&asset.media_kind) {
            return Err(format!(
                "媒体资产类型无效: media_asset_id={}, media_kind={}",
                asset.id, asset.media_kind
            ));
        }
        if !is_valid_media_origin(&asset.origin) {
            return Err(format!(
                "媒体资产来源无效: media_asset_id={}, origin={}",
                asset.id, asset.origin
            ));
        }
        if !is_valid_media_status(&asset.status) {
            return Err(format!(
                "媒体资产状态无效: media_asset_id={}, status={}",
                asset.id, asset.status
            ));
        }
        if asset.status == MEDIA_STATUS_COMPLETED && asset.file_path.is_none() {
            return Err(format!("completed 媒体资产缺少 file_path: {}", asset.id));
        }
        if asset.file_path.is_some() {
            require_media_payload(
                media_payload,
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                FILE_PATH_FIELD,
            )?;
        }
        if asset.thumbnail_path.is_some() {
            require_media_payload(
                media_payload,
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                THUMBNAIL_PATH_FIELD,
            )?;
        }
    }
    for provenance in &package.media_provenance {
        if !ids.media_asset_ids.contains(&provenance.media_asset_id) {
            return Err(format!(
                "媒体来源引用了不存在的媒体资产: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        if provenance.provider_job_id.is_some() {
            return Err(format!(
                "视频画布项目包媒体来源不能携带 provider_job_id: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        if let Some(key) = provenance
            .request_snapshot_json
            .as_ref()
            .and_then(canvas_transfer_runtime_identity_key)
        {
            return Err(format!(
                "视频画布项目包媒体来源 request_snapshot_json 不能携带运行身份 {key}: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        if let Some(spec_id) = provenance.generation_spec_id
            && !ids.generation_spec_ids.contains(&spec_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的生成意图: media_asset_id={}, generation_spec_id={spec_id}",
                provenance.media_asset_id
            ));
        }
        if let Some(artifact_id) = provenance.prompt_artifact_id
            && !ids.prompt_artifact_ids.contains(&artifact_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的提示词产物: media_asset_id={}, prompt_artifact_id={artifact_id}",
                provenance.media_asset_id
            ));
        }
        if let Some(source_asset_id) = provenance.source_media_asset_id
            && !ids.media_asset_ids.contains(&source_asset_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的源媒体资产: media_asset_id={}, source_media_asset_id={source_asset_id}",
                provenance.media_asset_id
            ));
        }
        validate_media_provenance_source_external_path(provenance, &ids.image_ids, &ids.video_ids)?;
        validate_media_provenance_snapshot(
            provenance,
            &MediaProvenanceSnapshotRefs {
                graph_ids: &ids.graph_ids,
                node_ids: &ids.node_ids,
                text_asset_ids: &ids.text_asset_ids,
                generation_spec_ids: &ids.generation_spec_ids,
                prompt_artifact_ids: &ids.prompt_artifact_ids,
                image_ids: &ids.image_ids,
                video_ids: &ids.video_ids,
                media_asset_ids: &ids.media_asset_ids,
                video_segment_ids: &ids.video_segment_ids,
                assembly_timeline_ids: &ids.assembly_timeline_ids,
                assembly_clip_ids: &ids.assembly_clip_ids,
            },
        )?;
    }
    Ok(())
}

pub(crate) fn validate_canvas_media_asset_owner(
    asset: &a4n_db_models::models::media::media_asset::Model,
    project_id: i64,
) -> Result<(), String> {
    if asset.project_id != project_id || asset.root_project_id != project_id {
        return Err(format!(
            "媒体资产不属于导入包 freeform 项目: media_asset_id={}",
            asset.id
        ));
    }
    if asset.source_snapshot_id.is_some()
        || asset.source_chapter_id.is_some()
        || asset.source_snapshot_chapter_id.is_some()
    {
        return Err(format!(
            "视频画布媒体资产不能绑定文本快照或章节: media_asset_id={}",
            asset.id
        ));
    }
    Ok(())
}
