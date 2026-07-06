use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{
    GeneratedImageSummary, normalize_image_metadata, to_summary,
};
use crate::actions::uploaded_image;
use crate::file_utils::{data_dir_relative_file_path, deletable_data_file_path};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::asset::profile_primary_material;
use a4n_services::image::{CreateImageRecord, UpdateImageRecord};
use a4n_services::profile_material::{
    generated_image_profile_material_identity, is_project_reusable_video_material_image,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_IMPORT_IMAGE_BYTES: usize = 50 * 1024 * 1024;
pub(crate) const IMPORTED_IMAGE_MODEL: &str = "external-import";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImagesByChapterInput {
    pub project_id: i64,
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListVideoReferenceImagesInput {
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(default)]
    pub image_project_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedImageInput {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub image_type: Option<String>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub clear_chapter_id: bool,
    #[serde(default)]
    pub panel_id: Option<String>,
    #[serde(default)]
    pub clear_panel_id: bool,
    #[serde(default)]
    pub ai_score: Option<f32>,
    #[serde(default)]
    pub clear_ai_score: bool,
    #[serde(default)]
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadImageInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub image_type: String,
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub panel_id: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<Value>,
}

pub async fn list_images_by_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let rows = state
        .image
        .list_by_project(project_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_images_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let rows = state
        .image
        .list_by_chapter(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_video_reference_images(
    state: &AppState,
    user_id: i64,
    input: ListVideoReferenceImagesInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let allowed_project_ids = state
        .video
        .video_asset_project_ids(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let image_project_ids =
        resolve_video_reference_image_project_ids(input.image_project_ids, &allowed_project_ids)?;
    access::verify_projects_editor_string(state, user_id, &image_project_ids).await?;

    let primary_rows = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.is_in(image_project_ids.clone()))
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let primary_image_ids = primary_rows
        .into_iter()
        .map(|row| row.primary_image_id)
        .collect::<HashSet<_>>();

    let project_images = state
        .image
        .list_video_reference_candidates(&image_project_ids, input.chapter_id)
        .await
        .map_err(|err| err.to_string())?;
    let mut by_id = HashMap::new();
    for image in project_images {
        if is_video_reference_image_candidate(&image, input.chapter_id) {
            by_id.insert(image.id, image);
        }
    }

    let mut images = by_id.into_values().collect::<Vec<_>>();
    images.sort_by(|a, b| {
        video_reference_scope_rank(a, input.chapter_id)
            .cmp(&video_reference_scope_rank(b, input.chapter_id))
            .then_with(|| {
                primary_image_ids
                    .contains(&b.id)
                    .cmp(&primary_image_ids.contains(&a.id))
            })
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| image_type_rank(&a.image_type).cmp(&image_type_rank(&b.image_type)))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(images.iter().map(to_summary).collect())
}

fn resolve_video_reference_image_project_ids(
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

pub async fn update_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
    input: UpdateGeneratedImageInput,
) -> Result<GeneratedImageSummary, String> {
    let existing = state
        .image
        .get_image(image_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let row = state
        .image
        .update(
            image_id,
            UpdateImageRecord {
                prompt: input.prompt,
                image_type: input.image_type,
                chapter_id: input.chapter_id,
                clear_chapter_id: input.clear_chapter_id,
                panel_id: input.panel_id,
                clear_panel_id: input.clear_panel_id,
                ai_score: input.ai_score,
                clear_ai_score: input.clear_ai_score,
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&row))
}

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let image_type = required_text(&input.image_type, "图片类型")?;
    let prompt = required_text(&input.prompt, "图片提示词")?;
    let model = match input.model.as_deref() {
        Some(value) => required_text(value, "图片模型")?,
        None => IMPORTED_IMAGE_MODEL.to_string(),
    };
    let imported = read_uploaded_image(&input)?;
    create_imported_image_record(
        state,
        project_id,
        ImportedImageRecordInput {
            image_type,
            prompt,
            model,
            chapter_id: input.chapter_id,
            panel_id: input.panel_id,
            metadata_json: input.metadata_json,
            imported,
        },
    )
    .await
}

struct ImportedImageRecordInput {
    image_type: String,
    prompt: String,
    model: String,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    metadata_json: Option<Value>,
    imported: ImportedImageFile,
}

async fn create_imported_image_record(
    state: &AppState,
    project_id: i64,
    input: ImportedImageRecordInput,
) -> Result<GeneratedImageSummary, String> {
    let ImportedImageRecordInput {
        image_type,
        prompt,
        model,
        chapter_id,
        panel_id,
        metadata_json,
        imported,
    } = input;
    let output_path = imported_image_output_path(
        &state.config.data_dir,
        project_id,
        chapter_id,
        imported.extension,
    )?;
    write_import_image(&output_path, &imported.bytes)?;
    let relative_path =
        data_dir_relative_file_path(&state.config.data_dir, &output_path, "导入图片")?;
    let metadata_json = imported_image_metadata(
        metadata_json,
        imported.source_file_name,
        imported.extension,
        imported.mime_type,
    );
    let record = match state
        .image
        .create_record(CreateImageRecord {
            project_id,
            image_type,
            file_path: relative_path,
            prompt,
            model,
            width: imported.width,
            height: imported.height,
            file_size: Some(imported.file_size),
            chapter_id,
            panel_id,
            metadata_json,
            created_at: None,
        })
        .await
    {
        Ok(record) => record,
        Err(err) => {
            let cleanup_error = remove_image_delete_file(&output_path).err();
            return Err(append_secondary_error(
                format!("导入图片记录创建失败：{err}"),
                cleanup_error,
            ));
        }
    };
    Ok(to_summary(&record))
}

pub async fn toggle_image_favorite(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<GeneratedImageSummary, String> {
    image_project_id_for_user(state, user_id, image_id).await?;
    let row = state
        .image
        .toggle_favorite(image_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&row))
}

pub async fn score_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
    score: f32,
) -> Result<GeneratedImageSummary, String> {
    image_project_id_for_user(state, user_id, image_id).await?;
    let row = state
        .image
        .update(
            image_id,
            UpdateImageRecord {
                ai_score: Some(score),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_summary(&row))
}

pub async fn delete_image(state: &AppState, user_id: i64, image_id: i64) -> Result<(), String> {
    let existing = state
        .image
        .get_image(image_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let image_path =
        deletable_data_file_path(&state.config.data_dir, &existing.file_path, "图片文件")?;
    let staged_file = match image_path.as_ref() {
        Some(path) => Some(stage_image_delete_file(path)?),
        None => None,
    };
    let delete_result = state
        .image
        .delete(image_id)
        .await
        .map_err(|e| e.to_string());
    match delete_result {
        Ok(()) => {
            if let Some(staged_file) = staged_file {
                staged_file.remove()?;
            }
            Ok(())
        }
        Err(err) => {
            let restore_error = staged_file.and_then(|staged_file| staged_file.restore().err());
            Err(append_secondary_error(
                format!("图片记录删除失败：{err}"),
                restore_error,
            ))
        }
    }
}

async fn image_project_id_for_user(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<i64, String> {
    let image = state
        .image
        .get_image(image_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, image.project_id).await?;
    Ok(image.project_id)
}

fn video_reference_scope_rank(
    image: &a4n_core::models::ai::generated_image::Model,
    chapter_id: i64,
) -> i32 {
    if image.chapter_id == Some(chapter_id) {
        return 0;
    }
    if generated_image_profile_material_identity(image).is_some() || image.chapter_id.is_none() {
        return 1;
    }
    2
}

fn is_video_reference_image_candidate(
    image: &a4n_core::models::ai::generated_image::Model,
    chapter_id: i64,
) -> bool {
    image.chapter_id == Some(chapter_id)
        || image.chapter_id.is_none()
        || is_project_reusable_video_material_image(&image.image_type, image.metadata_json.as_ref())
}

fn image_type_rank(image_type: &str) -> i32 {
    match image_type.trim() {
        "video_start_frame" | "video_end_frame" | "video_tail_frame" => 0,
        "manga_page" | "manga_panel" | "comic_panel" => 1,
        "scene_material" => 2,
        "object_material" => 3,
        "portrait" | "character_portrait" => 4,
        _ => 99,
    }
}

fn remove_image_delete_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("图片文件删除失败：{}：{err}", path.display())),
    }
}

struct StagedImageDeleteFile {
    original_path: PathBuf,
    staged_path: PathBuf,
    staging_dir: PathBuf,
}

impl StagedImageDeleteFile {
    fn remove(self) -> Result<(), String> {
        let remove_file = remove_image_delete_file(&self.staged_path);
        let remove_dir = remove_image_delete_staging_dir(&self.staging_dir);
        match (remove_file, remove_dir) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(file_err), Ok(())) => Err(file_err),
            (Ok(()), Err(dir_err)) => Err(dir_err),
            (Err(file_err), Err(dir_err)) => Err(format!("{file_err}；{dir_err}")),
        }
    }

    fn restore(self) -> Result<(), String> {
        let restore = std::fs::rename(&self.staged_path, &self.original_path).map_err(|err| {
            format!(
                "图片记录删除失败后恢复文件失败：{} -> {}：{err}",
                self.staged_path.display(),
                self.original_path.display()
            )
        });
        let remove_dir = remove_image_delete_staging_dir(&self.staging_dir);
        match (restore, remove_dir) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(restore_err), Ok(())) => Err(restore_err),
            (Ok(()), Err(dir_err)) => Err(dir_err),
            (Err(restore_err), Err(dir_err)) => Err(format!("{restore_err}；{dir_err}")),
        }
    }
}

fn stage_image_delete_file(path: &Path) -> Result<StagedImageDeleteFile, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("图片文件缺少父目录：{}", path.display()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("图片文件缺少文件名：{}", path.display()))?;
    let staging_dir = create_image_delete_staging_dir(parent)?;
    let staged_path = staging_dir.join(file_name);
    match std::fs::rename(path, &staged_path) {
        Ok(()) => Ok(StagedImageDeleteFile {
            original_path: path.to_path_buf(),
            staged_path,
            staging_dir,
        }),
        Err(err) => {
            let _ = std::fs::remove_dir(&staging_dir);
            Err(format!(
                "图片文件移入删除暂存区失败：{} -> {}：{err}",
                path.display(),
                staged_path.display()
            ))
        }
    }
}

fn create_image_delete_staging_dir(parent: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..100 {
        let candidate = parent.join(format!(
            ".a4n-delete-image-{}-{stamp}-{attempt}",
            std::process::id()
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "创建图片删除暂存目录失败：{}：{err}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "创建图片删除暂存目录失败：{} 下可用名称已耗尽",
        parent.display()
    ))
}

fn remove_image_delete_staging_dir(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "图片删除暂存目录删除失败：{}：{err}",
            path.display()
        )),
    }
}

pub(crate) type ImportedImageFile = uploaded_image::DecodedUploadedImage;

fn read_uploaded_image(input: &UploadImageInput) -> Result<ImportedImageFile, String> {
    read_uploaded_image_data(&input.file_name, &input.mime_type, &input.data_base64)
}

pub(crate) fn read_uploaded_image_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
) -> Result<ImportedImageFile, String> {
    uploaded_image::decode_uploaded_image_data(
        file_name,
        mime_type,
        data_base64,
        MAX_IMPORT_IMAGE_BYTES,
    )
}

fn imported_image_output_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = match chapter_id {
        Some(chapter_id) if chapter_id > 0 => {
            ProjectPaths::new(data_dir, project_id).chapter_images_dir(chapter_id)
        }
        Some(chapter_id) => {
            return Err(format!(
                "导入章节图片需要有效 chapter_id，收到 {chapter_id}"
            ));
        }
        None => ProjectPaths::new(data_dir, project_id).images_dir(),
    };
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建导入图片目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("imported_image_{stamp}{suffix}.{extension}"));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err("创建导入图片失败：文件名冲突过多".to_string())
}

fn write_import_image(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(bytes)
                .map_err(|err| format!("写入导入图片失败：{err}"))
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            Err("写入导入图片失败：文件名已存在".to_string())
        }
        Err(err) => Err(format!("创建导入图片失败：{err}")),
    }
}

fn imported_image_metadata(
    metadata_json: Option<Value>,
    source_file_name: String,
    source_extension: &'static str,
    mime_type: &'static str,
) -> Option<Value> {
    let mut metadata = match metadata_json {
        Some(Value::Object(map)) => Value::Object(map),
        Some(value) => json!({ "metadata": value }),
        None => json!({}),
    };
    if let Some(map) = metadata.as_object_mut() {
        map.insert(
            "import".to_string(),
            json!({
                "external": true,
                "source_file_name": source_file_name,
                "source_extension": source_extension,
                "mime_type": mime_type,
                "imported_at": Utc::now().to_rfc3339(),
            }),
        );
    }
    normalize_image_metadata(Some(metadata))
}

fn required_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}

fn append_secondary_error(primary: String, secondary: Option<String>) -> String {
    match secondary {
        Some(secondary) if !secondary.trim().is_empty() => {
            format!("{primary}；后续清理也失败：{secondary}")
        }
        _ => primary,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_video_reference_image_candidate, resolve_video_reference_image_project_ids,
        stage_image_delete_file,
    };
    use crate::file_utils::deletable_data_file_path;
    use a4n_core::models::ai::generated_image;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .expect("system clock must be after UNIX_EPOCH");
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
    }

    fn image_candidate(chapter_id: Option<i64>, image_type: &str) -> generated_image::Model {
        generated_image::Model {
            id: 1,
            project_id: 10,
            image_type: image_type.to_string(),
            file_path: "images/ref.png".to_string(),
            prompt: "prompt".to_string(),
            model: "model".to_string(),
            width: 512,
            height: 512,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            chapter_id,
            panel_id: None,
            metadata_json: None,
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn image_delete_file_path_accepts_existing_relative_file() {
        let data_dir = unique_temp_dir("a4n_image_delete_ok");
        let file_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");

        let resolved = deletable_data_file_path(&data_dir, "projects/1/images/ref.png", "图片文件")
            .expect("relative image path should resolve")
            .expect("existing image path should be returned");
        assert_eq!(
            resolved,
            file_path.canonicalize().expect("canonicalize file")
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn image_delete_file_path_skips_missing_file() {
        let data_dir = unique_temp_dir("a4n_image_delete_missing");
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        let resolved = deletable_data_file_path(&data_dir, "missing.png", "图片文件")
            .expect("missing relative image should not block record cleanup");
        assert_eq!(resolved, None);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn image_delete_file_path_rejects_absolute_path() {
        let data_dir = unique_temp_dir("a4n_image_delete_absolute");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let absolute = data_dir.join("image.png");

        let err = deletable_data_file_path(&data_dir, &absolute.to_string_lossy(), "图片文件")
            .expect_err("absolute image path should be rejected");
        assert!(err.contains("相对路径"));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn image_delete_file_path_rejects_directory() {
        let data_dir = unique_temp_dir("a4n_image_delete_directory");
        std::fs::create_dir_all(data_dir.join("images/ref.png")).expect("create directory");

        let err = deletable_data_file_path(&data_dir, "images/ref.png", "图片文件")
            .expect_err("directory image path should be rejected");
        assert_eq!(err, "图片文件不是文件");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn staged_image_delete_file_can_restore_original_file() {
        let data_dir = unique_temp_dir("a4n_image_delete_restore");
        let file_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");
        let canonical = file_path.canonicalize().expect("canonical image path");

        let staged = stage_image_delete_file(&canonical).expect("stage image file");
        assert!(!canonical.exists());
        assert!(staged.staged_path.exists());

        staged.restore().expect("restore staged image file");
        assert_eq!(
            std::fs::read(&canonical).expect("read restored file"),
            b"demo"
        );
        assert!(
            data_dir
                .join("projects/1/images")
                .read_dir()
                .expect("read image dir")
                .all(|entry| {
                    !entry
                        .expect("dir entry")
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".a4n-delete-image-")
                })
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn staged_image_delete_file_remove_deletes_staged_file_and_dir() {
        let data_dir = unique_temp_dir("a4n_image_delete_remove");
        let file_path = data_dir.join("projects/1/images/ref.png");
        std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
        std::fs::write(&file_path, b"demo").expect("write file");
        let canonical = file_path.canonicalize().expect("canonical image path");

        let staged = stage_image_delete_file(&canonical).expect("stage image file");
        let staging_dir = staged.staging_dir.clone();
        staged.remove().expect("remove staged image file");

        assert!(!canonical.exists());
        assert!(!staging_dir.exists());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn video_reference_candidate_keeps_original_scope_rules() {
        assert!(is_video_reference_image_candidate(
            &image_candidate(Some(7), "scene"),
            7,
        ));
        assert!(is_video_reference_image_candidate(
            &image_candidate(None, "scene"),
            7,
        ));

        let mut reusable = image_candidate(Some(9), "scene_material");
        reusable.metadata_json = Some(json!({
            "profile_kind": "scene",
            "profile_id": 3
        }));
        assert!(is_video_reference_image_candidate(&reusable, 7));

        let other_chapter = image_candidate(Some(9), "scene");
        assert!(!is_video_reference_image_candidate(&other_chapter, 7));
    }

    #[test]
    fn video_reference_image_project_ids_default_to_allowed_scope() {
        let ids = resolve_video_reference_image_project_ids(Vec::new(), &[20, 10])
            .expect("empty request should use allowed scope");

        assert_eq!(ids, vec![10, 20]);
    }

    #[test]
    fn video_reference_image_project_ids_deduplicate_requested_scope() {
        let ids = resolve_video_reference_image_project_ids(vec![20, 10, 20], &[20, 10])
            .expect("allowed duplicates should be normalized");

        assert_eq!(ids, vec![10, 20]);
    }

    #[test]
    fn video_reference_image_project_ids_reject_non_positive_ids() {
        let err = resolve_video_reference_image_project_ids(vec![10, 0], &[10])
            .expect_err("non-positive image project id should fail");

        assert!(err.contains("image_project_ids[1] 必须为正整数"));
    }

    #[test]
    fn video_reference_image_project_ids_reject_projects_outside_video_scope() {
        let err = resolve_video_reference_image_project_ids(vec![10, 99], &[10, 20])
            .expect_err("unrelated owned projects should not enter video references");

        assert!(err.contains("不属于当前视频项目允许的素材项目：99"));
    }
}
