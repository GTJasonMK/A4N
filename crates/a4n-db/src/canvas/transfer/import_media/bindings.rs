use super::super::remap::remap_video_studio_required_id;
use super::super::types::{VideoStudioProjectImportError, VideoStudioProjectImportIdMaps};
use a4n_core::models::media::media_binding;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_media_bindings<C>(
    db: &C,
    bindings: &[media_binding::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for binding in bindings {
        let target_id = match binding.target_kind.as_str() {
            "generated_video" => remap_video_studio_required_id(
                &maps.video_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "assembly_timeline" => remap_video_studio_required_id(
                &maps.assembly_timeline_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "video_segment" => remap_video_studio_required_id(
                &maps.video_segment_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            other => {
                return Err(VideoStudioProjectImportError::InvalidPackage(format!(
                    "视频画布项目包暂不支持媒体绑定目标类型: target_kind={other}"
                )));
            }
        };
        let row = media_binding::ActiveModel {
            media_asset_id: Set(remap_video_studio_required_id(
                &maps.media_asset_ids,
                binding.media_asset_id,
                "media_binding.media_asset_id",
            )?),
            target_kind: Set(binding.target_kind.clone()),
            target_id: Set(target_id),
            role: Set(binding.role.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.media_binding_ids.insert(binding.id, row.id);
    }
    Ok(())
}
