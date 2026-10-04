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
    ImportMediaPayload, VideoStudioProjectPackage, normalize_project_type, validate_canvas_project,
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
    required_text(&package.project.title, "Canvas 项目包项目标题")?;
    validate_canvas_project(&package.project, "Canvas 项目包")?;
    normalize_project_type(&package.project.project_type).map_err(|err| err.to_string())?;
    ProjectStatus::parse_storage(&package.project.status).map_err(|err| err.to_string())?;

    let ids = validate_and_collect_manifest_ids(package)?;
    crate::project::validate_provider_slots(&package.provider_slots)?;
    validate_canvas_records(package, &ids)?;
    validate_generation_records(package, &ids)?;
    validate_asset_records(package, media_payload, &ids)?;
    validate_assembly_records(package, &ids)?;
    validate_payload_entries(media_payload, &ids)
}
