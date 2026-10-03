use crate::media::{
    MEDIA_STATUS_COMPLETED, is_valid_media_kind, is_valid_media_origin, is_valid_media_status,
};
use a4n_db_models::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};

use super::super::assets::validate_media_asset_source_context;
use super::context::MediaManifestContext;
use crate::project::{
    ImportMediaPayload, family_package::ProjectFamilyPackage, media_asset_path, media_file_key,
};

const MEDIA_ASSET_OWNER: &str = "media_asset";

pub(super) fn validate_media_assets(
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
    context: &MediaManifestContext<'_>,
) -> Result<(), String> {
    for asset in &package.media_assets {
        if asset.project_id == package.root_project_id {
            return Err(format!(
                "项目族导入包媒体资产不能属于根文本项目: media_asset_id={}",
                asset.id
            ));
        }
        if !is_valid_media_status(&asset.status) {
            return Err(format!(
                "项目族导入包媒体资产状态无效: media_asset_id={}, status={}",
                asset.id, asset.status
            ));
        }
        if !is_valid_media_kind(&asset.media_kind) {
            return Err(format!(
                "项目族导入包媒体资产类型无效: media_asset_id={}, media_kind={}",
                asset.id, asset.media_kind
            ));
        }
        if !is_valid_media_origin(&asset.origin) {
            return Err(format!(
                "项目族导入包媒体资产来源无效: media_asset_id={}, origin={}",
                asset.id, asset.origin
            ));
        }
        if asset.root_project_id == asset.project_id {
            if !package
                .projects
                .iter()
                .any(|project| project.id == asset.project_id && project.project_kind == "video")
            {
                return Err(format!(
                    "freeform 媒体资产必须属于项目族的视频项目: {}",
                    asset.id
                ));
            }
            if asset.source_snapshot_id.is_some()
                || asset.source_chapter_id.is_some()
                || asset.source_snapshot_chapter_id.is_some()
            {
                return Err(format!(
                    "freeform 媒体资产不能绑定文本快照或章节: media_asset_id={}",
                    asset.id
                ));
            }
        } else {
            if asset.root_project_id != package.root_project_id {
                return Err(format!(
                    "媒体资产 root_project_id 与导入包根项目不一致: media_asset_id={}",
                    asset.id
                ));
            }
            validate_media_asset_source_context(
                asset,
                package.root_project_id,
                &context.source_snapshots,
                &context.source_snapshot_chapters,
            )?;
        }
        if media_asset_path(asset, FILE_PATH_FIELD)?.is_some()
            && !media_payload.entries.contains_key(&media_file_key(
                MEDIA_ASSET_OWNER,
                asset.id,
                FILE_PATH_FIELD,
            ))
        {
            return Err(format!("媒体资产缺少文件包条目: {}", asset.id));
        }
        if media_asset_path(asset, THUMBNAIL_PATH_FIELD)?.is_some()
            && !media_payload.entries.contains_key(&media_file_key(
                MEDIA_ASSET_OWNER,
                asset.id,
                THUMBNAIL_PATH_FIELD,
            ))
        {
            return Err(format!("媒体资产缺少缩略图包条目: {}", asset.id));
        }
        if asset.status == MEDIA_STATUS_COMPLETED && asset.file_path.is_none() {
            return Err(format!("completed 媒体资产缺少 file_path: {}", asset.id));
        }
    }
    Ok(())
}
