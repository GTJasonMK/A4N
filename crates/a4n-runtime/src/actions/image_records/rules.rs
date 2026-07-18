use a4n_core::models::ai::generated_image;
use a4n_orchestrator::profile_material::{
    generated_image_profile_material_identity, is_project_reusable_video_material_image,
};
use std::collections::HashSet;

pub(super) fn resolve_video_reference_image_project_ids(
    requested_project_ids: Vec<i64>,
    allowed_project_ids: &[i64],
) -> Result<Vec<i64>, String> {
    let allowed = allowed_project_ids.iter().copied().collect::<HashSet<_>>();
    let mut project_ids = if requested_project_ids.is_empty() {
        allowed_project_ids.to_vec()
    } else {
        let mut resolved = Vec::with_capacity(requested_project_ids.len());
        for (index, project_id) in requested_project_ids.into_iter().enumerate() {
            if project_id <= 0 {
                return Err(format!("image_project_ids[{index}] 必须为正整数"));
            }
            if !allowed.contains(&project_id) {
                return Err(format!(
                    "image_project_ids[{index}] 不属于当前视频项目允许的素材项目：{project_id}"
                ));
            }
            resolved.push(project_id);
        }
        resolved
    };
    project_ids.sort_unstable();
    project_ids.dedup();
    Ok(project_ids)
}

pub(super) fn video_reference_scope_rank(image: &generated_image::Model, chapter_id: i64) -> i32 {
    if image.chapter_id == Some(chapter_id) {
        return 0;
    }
    if generated_image_profile_material_identity(image).is_some() || image.chapter_id.is_none() {
        return 1;
    }
    2
}

pub(super) fn is_video_reference_image_candidate(
    image: &generated_image::Model,
    chapter_id: i64,
) -> bool {
    image.chapter_id == Some(chapter_id)
        || image.chapter_id.is_none()
        || is_project_reusable_video_material_image(&image.image_type, image.metadata_json.as_ref())
}

pub(super) fn image_type_rank(image_type: &str) -> i32 {
    match image_type.trim() {
        "video_start_frame" | "video_end_frame" | "video_tail_frame" => 0,
        "manga_page" | "manga_panel" | "comic_panel" => 1,
        "scene_material" => 2,
        "object_material" => 3,
        "portrait" | "character_portrait" => 4,
        _ => 99,
    }
}

pub(super) fn required_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}

pub(super) fn append_secondary_error(primary: String, secondary: Option<String>) -> String {
    match secondary {
        Some(secondary) if !secondary.trim().is_empty() => {
            format!("{primary}；后续清理也失败：{secondary}")
        }
        _ => primary,
    }
}
