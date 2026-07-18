use crate::AppState;
use crate::file_utils::canonicalize_data_file_path;
use a4n_image::ImageReference;
use base64::Engine as _;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;

pub async fn collect_generated_reference_images_by_ids(
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
            .orchestrator
            .ensure_video_reference_images_available(
                project_id,
                chapter_id,
                Some(&json!(image_ids)),
            )
            .await
            .map_err(|error| error.to_string())?;
        state
            .orchestrator
            .video_asset_project_ids(project_id)
            .await
            .map_err(|error| error.to_string())?
    } else {
        vec![project_id]
    };
    let images_by_id: HashMap<_, _> = state
        .orchestrator
        .generated_images_by_ids(image_ids)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|image| (image.id, image))
        .collect();
    let mut out = Vec::with_capacity(image_ids.len());
    for (index, image_id) in image_ids.iter().copied().enumerate() {
        let image = images_by_id
            .get(&image_id)
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

pub fn collect_bound_reference_images(
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
        let binding_context = format!("reference_bindings[{index}]");
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
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("读取参考图 {} 失败：{error}", path.display()))?;
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
