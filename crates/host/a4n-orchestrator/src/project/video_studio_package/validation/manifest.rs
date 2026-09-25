mod assembly;
mod assets;
pub(crate) use assets::validate_canvas_media_asset_owner;
mod canvas_records;
mod contracts;
mod generation;
mod ids;
mod payload;

use self::assembly::validate_assembly_records;
use self::assets::validate_asset_records;
use self::canvas_records::validate_canvas_records;
use self::generation::validate_generation_records;
use self::ids::validate_and_collect_manifest_ids;
use self::payload::validate_payload_entries;
use a4n_domain::project::{ProjectStatus, required_text};

use crate::project::video_studio_package::{PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION};
use crate::project::{
    ImportMediaPayload, VideoStudioProjectPackage, normalize_project_type,
    validate_freeform_video_project,
};

pub(crate) fn validate_family_canvas_package(
    canvas: &VideoStudioProjectPackage,
    family: &crate::project::ProjectFamilyPackage,
) -> Result<std::collections::HashSet<i64>, String> {
    let project = family
        .projects
        .iter()
        .find(|project| project.id == canvas.project.id)
        .ok_or_else(|| "画布不属于项目族".to_string())?;
    if project.project_kind != "video"
        || project != &canvas.project
        || canvas.version != VIDEO_STUDIO_PACKAGE_VERSION
        || canvas.mode != super::super::FAMILY_CANVAS_MODE
    {
        return Err("项目族画布的项目身份、版本或模式不一致".to_string());
    }
    if [
        canvas.generation_specs.len(),
        canvas.prompt_artifacts.len(),
        canvas.generated_images.len(),
        canvas.generated_videos.len(),
        canvas.media_assets.len(),
        canvas.media_provenance.len(),
        canvas.media_bindings.len(),
        canvas.video_segments.len(),
        canvas.assembly_timelines.len(),
        canvas.assembly_clips.len(),
        canvas.media_files.len(),
    ]
    .into_iter()
    .any(|count| count != 0)
    {
        return Err("项目族画布只能携带图和文本资产；共享媒体及生成记录由项目族保存".to_string());
    }
    let mut ids = validate_and_collect_manifest_ids(canvas)?;
    ids.generation_spec_ids.extend(
        family
            .generation_specs
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.prompt_artifact_ids.extend(
        family
            .prompt_artifacts
            .iter()
            .filter(|row| ids.generation_spec_ids.contains(&row.generation_spec_id))
            .map(|row| row.id),
    );
    ids.image_ids.extend(
        family
            .generated_images
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.video_ids.extend(
        family
            .generated_videos
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.media_asset_ids.extend(
        family
            .media_assets
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.video_segment_ids.extend(
        family
            .video_segments
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.assembly_timeline_ids.extend(
        family
            .assembly_timelines
            .iter()
            .filter(|row| row.project_id == project.id)
            .map(|row| row.id),
    );
    ids.assembly_clip_ids.extend(
        family
            .assembly_clips
            .iter()
            .filter(|row| ids.assembly_timeline_ids.contains(&row.timeline_id))
            .map(|row| row.id),
    );
    validate_canvas_records(canvas, &ids)?;
    // Validate the family's canvas-owned generation rows with the existing
    // freeform contract. The serialized family remains their only owner.
    let mut generation_view = canvas.clone();
    generation_view.generation_specs = family
        .generation_specs
        .iter()
        .filter(|spec| {
            spec.project_id == project.id
                && spec.source_context_kind.as_deref()
                    == Some(
                        a4n_db_models::models::generation::generation_spec::SOURCE_CONTEXT_FREEFORM,
                    )
        })
        .cloned()
        .collect();
    validate_generation_records(&generation_view, &ids)?;
    Ok(generation_view
        .generation_specs
        .iter()
        .map(|spec| spec.id)
        .collect())
}

pub(super) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    if package.version != VIDEO_STUDIO_PACKAGE_VERSION {
        return Err(format!("视频画布项目包版本无效: {}", package.version));
    }
    if package.mode != PACKAGE_MODE {
        return Err(format!("视频画布项目包模式无效: {}", package.mode));
    }
    required_text(&package.project.title, "视频画布项目包项目标题")?;
    validate_freeform_video_project(&package.project, "视频画布项目包")?;
    normalize_project_type(&package.project.project_type).map_err(|err| err.to_string())?;
    ProjectStatus::parse_storage(&package.project.status).map_err(|err| err.to_string())?;

    let ids = validate_and_collect_manifest_ids(package)?;
    validate_canvas_records(package, &ids)?;
    validate_generation_records(package, &ids)?;
    validate_asset_records(package, media_payload, &ids)?;
    validate_assembly_records(package, &ids)?;
    validate_payload_entries(media_payload, &ids)
}
