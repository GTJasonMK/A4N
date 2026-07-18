mod assembly;
mod assets;
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
use super::*;

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
    validate_freeform_video_project(&package.project, "视频画布项目包")?;
    normalize_project_type(&package.project.project_type).map_err(|err| err.to_string())?;
    StateValidator::parse_status(&package.project.status).map_err(|err| err.to_string())?;

    let ids = validate_and_collect_manifest_ids(package)?;
    validate_canvas_records(package, &ids)?;
    validate_generation_records(package, &ids)?;
    validate_asset_records(package, media_payload, &ids)?;
    validate_assembly_records(package, &ids)?;
    validate_payload_entries(media_payload, &ids)
}
