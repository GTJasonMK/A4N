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
use a4n_core::state::validator::parse_project_status;

use crate::project::{normalize_project_type, required_text, validate_freeform_video_project};
use crate::project::video_studio_package::{
    ImportMediaPayload, PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION, VideoStudioProjectPackage,
};

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
    parse_project_status(&package.project.status).map_err(|err| err.to_string())?;

    let ids = validate_and_collect_manifest_ids(package)?;
    validate_canvas_records(package, &ids)?;
    validate_generation_records(package, &ids)?;
    validate_asset_records(package, media_payload, &ids)?;
    validate_assembly_records(package, &ids)?;
    validate_payload_entries(media_payload, &ids)
}
