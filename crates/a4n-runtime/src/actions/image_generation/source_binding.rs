use crate::AppState;
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::{chapter, chapter_manga_prompt};
use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_engine::image::ImageReference;
use a4n_services::source::{
    ChapterSourceContext, metadata_source_content_sha256, source_content_sha256, source_metadata,
};
use base64::Engine as _;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImageChapterBindingAccess {
    ReadOnly,
    Writable,
}

pub(super) async fn resolve_image_chapter_binding(
    state: &AppState,
    project_id: i64,
    chapter_id: Option<i64>,
    access: ImageChapterBindingAccess,
) -> Result<(Option<i64>, Option<Value>), String> {
    if project_id <= 0 {
        return Err("project_id 必须大于 0".to_string());
    }
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;

    let Some(id) = chapter_id else {
        return Ok((None, None));
    };
    let chapter = chapter::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("章节 #{id} 不存在"))?;
    let source_content = match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(format!("章节 #{} 不属于文本项目 {project_id}", chapter.id));
            }
            if chapter.status == "deleted" {
                return Err(format!(
                    "第 {} 章已删除，不能绑定图片素材",
                    chapter.chapter_number
                ));
            }
            if access == ImageChapterBindingAccess::Writable && chapter.status == "generating" {
                return Err(format!(
                    "第 {} 章正在生成中，请等待完成后再绑定图片素材",
                    chapter.chapter_number
                ));
            }
            chapter.content.clone()
        }
        "comic" | "video" | "galgame" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(format!(
                    "章节 #{} 不属于派生项目 {project_id} 绑定的根文本项目",
                    chapter.id
                ));
            }
            let snapshot_id = project
                .source_snapshot_id
                .ok_or_else(|| format!("派生项目缺少源文本快照: {project_id}"))?;
            source_snapshot_chapter::Entity::find()
                .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
                .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter.id))
                .one(state.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("派生项目 {project_id} 的源快照不包含章节 {}", chapter.id))?
                .content
        }
        other => {
            return Err(format!(
                "图片链路不支持项目类型 {other}: project_id={project_id}"
            ));
        }
    };
    let source_metadata = if source_content.trim().is_empty() {
        None
    } else {
        Some(image_source_metadata(&source_content))
    };

    Ok((Some(chapter.id), source_metadata))
}

pub(super) async fn collect_generated_reference_images_by_ids(
    state: &AppState,
    project_id: i64,
    chapter_id: Option<i64>,
    video_frame_scope: bool,
    image_ids: &[i64],
) -> Result<Vec<ImageReference>, String> {
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    let allowed_project_ids = if video_frame_scope {
        let chapter_id =
            chapter_id.ok_or_else(|| "视频帧参考图缺少章节上下文，无法校验引用范围".to_string())?;
        state
            .video
            .ensure_video_reference_images_available(
                project_id,
                chapter_id,
                Some(&json!(image_ids)),
            )
            .await
            .map_err(|e| e.to_string())?;
        state
            .video
            .video_asset_project_ids(project_id)
            .await
            .map_err(|e| e.to_string())?
    } else {
        vec![project_id]
    };
    let mut out = Vec::with_capacity(image_ids.len());
    for (index, image_id) in image_ids.iter().copied().enumerate() {
        let image = generated_image::Entity::find_by_id(image_id)
            .one(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("参考图不存在：#{image_id}"))?;
        if !allowed_project_ids.contains(&image.project_id) {
            let scope = if video_frame_scope {
                "当前视频项目或其根文本项目"
            } else {
                "当前项目"
            };
            return Err(format!("参考图 #{} 不属于{scope}", image_id));
        }
        let label = format!("参考图 {}", index + 1);
        out.push(read_reference_image(state, &label, &image.file_path)?);
    }
    Ok(out)
}

pub(super) fn collect_bound_reference_images(
    state: &AppState,
    layout: Option<&Value>,
) -> Result<Vec<ImageReference>, String> {
    let Some(bindings) = layout
        .and_then(|layout| layout.get("reference_bindings"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (index, binding) in bindings.iter().enumerate() {
        let binding_context = format!("reference_bindings[{}]", index);
        let binding_object = binding
            .as_object()
            .ok_or_else(|| format!("{binding_context} 必须是 JSON 对象"))?;
        let status = binding_object
            .get("status")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|status| !status.is_empty())
            .unwrap_or("missing");
        let Some(asset) = binding_object.get("bound_asset").and_then(Value::as_object) else {
            if status == "bound" {
                return Err(format!(
                    "{binding_context} 已标记 bound，但缺少 bound_asset"
                ));
            }
            continue;
        };
        let Some(path) = asset
            .get("image_path")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
        else {
            return Err(format!("{binding_context}.bound_asset 缺少 image_path"));
        };
        let label = binding_object
            .get("label")
            .and_then(Value::as_str)
            .or_else(|| asset.get("label").and_then(Value::as_str))
            .unwrap_or("参考图")
            .trim()
            .to_string();
        out.push(read_reference_image(state, &label, path)?);
    }
    Ok(out)
}

fn read_reference_image(
    state: &AppState,
    label: &str,
    path: &str,
) -> Result<ImageReference, String> {
    let path =
        canonicalize_data_file_path(&state.config.data_dir, path, &format!("参考图 {label}"))?;
    let bytes =
        std::fs::read(&path).map_err(|e| format!("读取参考图 {} 失败：{e}", path.display()))?;
    let mime = reference_image_mime(&path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(ImageReference {
        label: label.to_string(),
        data_uri: format!("data:{mime};base64,{encoded}"),
    })
}

fn reference_image_mime(path: &Path) -> Result<&'static str, String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        other => Err(format!("参考图格式不支持: {other}，仅支持 png/jpg/webp")),
    }
}

pub(super) fn image_source_metadata(content: &str) -> Value {
    source_metadata(ChapterSourceContext { content })
}

pub(super) fn ensure_manga_page_panels_match_current_source(
    panels: &[chapter_manga_prompt::Model],
    current_content: &str,
) -> Result<(), String> {
    let current_hash = source_content_sha256(current_content);
    for panel in panels {
        let source = panel
            .layout
            .as_ref()
            .and_then(|layout| layout.get("source"));
        if source.is_none() {
            return Err(format!(
                "漫画提示词 #{} 缺少章节来源信息，请重新生成漫画提示词",
                panel.id
            ));
        }
        let Some(source_hash) = metadata_source_content_sha256(panel.layout.as_ref()) else {
            return Err(format!(
                "漫画提示词 #{} 缺少章节正文指纹，请重新生成漫画提示词",
                panel.id
            ));
        };
        if source_hash != current_hash {
            return Err(format!(
                "漫画提示词 #{} 基于旧章节正文，请重新生成漫画提示词",
                panel.id
            ));
        }
    }
    Ok(())
}
