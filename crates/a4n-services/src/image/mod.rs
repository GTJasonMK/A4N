//! 图片管理服务：生成图片的存取与收藏管理
//!
//! 对应文档 16-图片生成系统

use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;

mod deletion;
mod provider_config;

use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::asset::profile_primary_material;
use a4n_core::models::chapter::chapter;
#[cfg(test)]
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::models::project::novel_project;

use self::deletion::clear_generated_image_references;
pub use self::provider_config::{
    configured_image_provider, image_backend_supports_reference_images,
};
use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunWithCheckpointInput,
    append_generation_artifact_target, delete_generation_run_targets_for,
    record_completed_generation_run_with_checkpoint,
};
use crate::profile_material::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};

#[derive(Clone)]
pub struct ImageService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
pub struct CreateImageRecord {
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<serde_json::Value>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

struct ValidatedCreateImageRecord {
    project_id: i64,
    image_type: String,
    file_path: String,
    prompt: String,
    model: String,
    width: i32,
    height: i32,
    file_size: Option<i64>,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    metadata_json: Option<serde_json::Value>,
    created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateImageRecord {
    pub prompt: Option<String>,
    pub image_type: Option<String>,
    pub chapter_id: Option<i64>,
    pub clear_chapter_id: bool,
    pub panel_id: Option<String>,
    pub clear_panel_id: bool,
    pub ai_score: Option<f32>,
    pub clear_ai_score: bool,
    pub is_favorite: Option<bool>,
}

impl ImageService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn get_project(&self, project_id: i64) -> Result<novel_project::Model, AppError> {
        validate_positive_i64(project_id, "project_id")?;
        novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }

    async fn ensure_project_exists(&self, project_id: i64) -> Result<(), AppError> {
        self.get_project(project_id).await.map(|_| ())
    }

    async fn ensure_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let project = self.get_project(project_id).await?;
        let chapter = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        match project.project_kind.as_str() {
            "text" => {
                if chapter.project_id != project.id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{chapter_id} 不属于文本项目 {project_id}"
                    )));
                }
            }
            "comic" | "video" | "galgame" => {
                if project.root_project_id != Some(chapter.project_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{chapter_id} 不属于派生项目 {project_id} 绑定的根文本项目"
                    )));
                }
            }
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "图片链路不支持项目类型 {other}: project_id={project_id}"
                )));
            }
        }
        Ok(())
    }

    pub async fn list_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.ensure_project_exists(project_id).await?;
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .order_by_desc(generated_image::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::ChapterId.eq(chapter_id))
            .order_by_desc(generated_image::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_video_reference_candidates(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_chapter_belongs_to_projects(project_ids, chapter_id)
            .await?;
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(
                Condition::any()
                    .add(generated_image::Column::ChapterId.eq(chapter_id))
                    .add(generated_image::Column::ChapterId.is_null())
                    .add(
                        generated_image::Column::ImageType
                            .is_in([IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_OBJECT_MATERIAL]),
                    ),
            )
            .order_by_desc(generated_image::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn create_record(
        &self,
        input: CreateImageRecord,
    ) -> Result<generated_image::Model, AppError> {
        let input = self.validate_create_record_input(input).await?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let image = insert_validated_image_record(&tx, input).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(image)
    }

    pub async fn create_record_with_generation_run_checkpoint(
        &self,
        input: CreateImageRecord,
        mut generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generated_image::Model, AppError> {
        let input = self.validate_create_record_input(input).await?;
        if generation_run.run.project_id != input.project_id {
            return Err(AppError::InvalidArgument(
                "图片生成运行记录 project_id 与图片资产不一致".to_string(),
            ));
        }
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let image = insert_validated_image_record(&tx, input).await?;
        generation_run.run.targets.push(GenerationRunTraceTarget {
            target_kind: image.image_type.clone(),
            target_table: "generated_image".to_string(),
            target_id: image.id,
        });
        generation_run.run.response_summary_json =
            image_generation_response_summary(generation_run.run.response_summary_json, &image)?;
        generation_run.checkpoint.parsed_output_json =
            image_generation_parsed_output(generation_run.checkpoint.parsed_output_json, &image)?;
        generation_run.checkpoint.artifact_refs_json =
            image_generation_artifact_refs(generation_run.checkpoint.artifact_refs_json, &image)?;
        record_completed_generation_run_with_checkpoint(
            &tx,
            generation_run.run,
            generation_run.checkpoint,
        )
        .await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(image)
    }

    async fn validate_create_record_input(
        &self,
        input: CreateImageRecord,
    ) -> Result<ValidatedCreateImageRecord, AppError> {
        validate_positive_i64(input.project_id, "project_id")?;
        let chapter_id = match input.chapter_id {
            Some(chapter_id) => Some(validate_positive_i64(chapter_id, "chapter_id")?),
            None => None,
        };
        let image_type = required_trimmed(&input.image_type, "image_type")?;
        let file_path = required_trimmed(&input.file_path, "file_path")?;
        let prompt = required_trimmed(&input.prompt, "prompt")?;
        let model = required_trimmed(&input.model, "model")?;
        let width = validate_positive_i32(input.width, "width")?;
        let height = validate_positive_i32(input.height, "height")?;
        let file_size = match input.file_size {
            Some(value) if value < 0 => {
                return Err(AppError::InvalidArgument(
                    "file_size 不能小于 0".to_string(),
                ));
            }
            value => value,
        };
        let panel_id = normalize_optional_text(input.panel_id.as_deref());
        match chapter_id {
            Some(chapter_id) => {
                self.ensure_chapter_belongs_to_project(input.project_id, chapter_id)
                    .await?;
            }
            None => self.ensure_project_exists(input.project_id).await?,
        }

        let now = input
            .created_at
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());
        Ok(ValidatedCreateImageRecord {
            project_id: input.project_id,
            image_type,
            file_path,
            prompt,
            model,
            width,
            height,
            file_size,
            chapter_id,
            panel_id,
            metadata_json: input.metadata_json,
            created_at: now,
        })
    }

    async fn ensure_chapter_belongs_to_projects(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let chapter_id = validate_positive_i64(chapter_id, "chapter_id")?;
        let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
        for project_id in project_ids {
            normalized_project_ids.push(validate_positive_i64(*project_id, "project_id")?);
        }
        normalized_project_ids.sort_unstable();
        normalized_project_ids.dedup();

        let projects = novel_project::Entity::find()
            .filter(novel_project::Column::Id.is_in(normalized_project_ids.iter().copied()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let projects_by_id = projects
            .into_iter()
            .map(|project| (project.id, project))
            .collect::<HashMap<_, _>>();
        for project_id in &normalized_project_ids {
            if !projects_by_id.contains_key(project_id) {
                return Err(AppError::NotFound(format!("项目不存在: {project_id}")));
            }
        }

        let chapter = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;

        for project_id in normalized_project_ids {
            let project = projects_by_id
                .get(&project_id)
                .expect("missing projects checked above");
            match project.project_kind.as_str() {
                "text" => {
                    if chapter.project_id != project.id {
                        return Err(AppError::InvalidArgument(format!(
                            "章节 #{chapter_id} 不属于文本项目 {project_id}"
                        )));
                    }
                }
                "comic" | "video" | "galgame" => {
                    if project.root_project_id != Some(chapter.project_id) {
                        return Err(AppError::InvalidArgument(format!(
                            "章节 #{chapter_id} 不属于派生项目 {project_id} 绑定的根文本项目"
                        )));
                    }
                }
                other => {
                    return Err(AppError::InvalidArgument(format!(
                        "图片链路不支持项目类型 {other}: project_id={project_id}"
                    )));
                }
            }
        }
        Ok(())
    }

    pub async fn update(
        &self,
        image_id: i64,
        input: UpdateImageRecord,
    ) -> Result<generated_image::Model, AppError> {
        validate_positive_i64(image_id, "image_id")?;
        validate_update_record_clear_flags(&input)?;
        let image = generated_image::Entity::find_by_id(image_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("图片不存在".into()))?;

        let mut active: generated_image::ActiveModel = image.clone().into();
        if let Some(prompt) = input.prompt {
            active.prompt = Set(required_trimmed(&prompt, "prompt")?);
        }
        if let Some(image_type) = input.image_type {
            let image_type = required_trimmed(&image_type, "image_type")?;
            validate_update_record_image_type(&image.image_type, &image_type)?;
            active.image_type = Set(image_type);
        }
        if input.clear_chapter_id {
            validate_update_record_chapter_binding(&image, None)?;
            active.chapter_id = Set(None);
        } else if let Some(chapter_id) = input.chapter_id {
            let chapter_id = validate_positive_i64(chapter_id, "chapter_id")?;
            validate_update_record_chapter_binding(&image, Some(chapter_id))?;
            self.ensure_chapter_belongs_to_project(image.project_id, chapter_id)
                .await?;
            active.chapter_id = Set(Some(chapter_id));
        }
        if input.clear_panel_id {
            validate_update_record_panel_binding(&image, None)?;
            active.panel_id = Set(None);
        } else if input.panel_id.is_some() {
            let panel_id = normalize_optional_text(input.panel_id.as_deref());
            validate_update_record_panel_binding(&image, panel_id.as_deref())?;
            active.panel_id = Set(panel_id);
        }
        if input.clear_ai_score {
            active.ai_score = Set(None);
        } else if let Some(score) = input.ai_score {
            active.ai_score = Set(Some(validate_finite_f32(score, "ai_score")?));
        }
        if let Some(is_favorite) = input.is_favorite {
            active.is_favorite = Set(is_favorite);
        }
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn toggle_favorite(&self, image_id: i64) -> Result<generated_image::Model, AppError> {
        let img = generated_image::Entity::find_by_id(image_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("图片不存在".into()))?;

        let mut active: generated_image::ActiveModel = img.into();
        active.is_favorite = Set(!active.is_favorite.unwrap());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn get_image(&self, image_id: i64) -> Result<generated_image::Model, AppError> {
        generated_image::Entity::find_by_id(image_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("图片不存在".into()))
    }

    pub async fn delete(&self, image_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        generated_image::Entity::find_by_id(image_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("图片不存在".into()))?;

        clear_generated_image_references(&txn, &[image_id]).await?;
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::PrimaryImageId.eq(image_id))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "generated_image", &[image_id]).await?;
        generated_image::Entity::delete_by_id(image_id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn delete_manga_page_images(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        let panel_id = format!("page{page_number}");
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::ChapterId.eq(chapter_id))
            .all(&txn)
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .filter(|image| {
                should_delete_manga_page_image(image, &panel_id, page_number, preserve_image_id)
            })
            .collect::<Vec<_>>();
        let file_paths = images
            .iter()
            .map(|image| image.file_path.clone())
            .collect::<Vec<_>>();
        let image_ids = images.iter().map(|image| image.id).collect::<Vec<_>>();
        if !image_ids.is_empty() {
            clear_generated_image_references(&txn, &image_ids).await?;
            delete_generation_run_targets_for(&txn, "generated_image", &image_ids).await?;
            generated_image::Entity::delete_many()
                .filter(generated_image::Column::Id.is_in(image_ids))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(file_paths)
    }
}

async fn insert_validated_image_record<C>(
    db: &C,
    input: ValidatedCreateImageRecord,
) -> Result<generated_image::Model, AppError>
where
    C: ConnectionTrait,
{
    let image = generated_image::ActiveModel {
        project_id: Set(input.project_id),
        image_type: Set(input.image_type),
        file_path: Set(input.file_path),
        prompt: Set(input.prompt),
        model: Set(input.model),
        width: Set(input.width),
        height: Set(input.height),
        file_size: Set(input.file_size),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(input.chapter_id),
        panel_id: Set(input.panel_id),
        metadata_json: Set(input.metadata_json),
        created_at: Set(input.created_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(image)
}

fn image_generation_response_summary(
    summary: Option<serde_json::Value>,
    image: &generated_image::Model,
) -> Result<Option<serde_json::Value>, AppError> {
    let generated = serde_json::json!({
        "generated_image_id": image.id,
        "file_path": &image.file_path,
        "model": &image.model,
        "file_size": image.file_size,
    });
    match summary {
        Some(serde_json::Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Ok(Some(serde_json::Value::Object(map)))
        }
        Some(_) => Err(AppError::InvalidArgument(
            "图片生成运行记录 response_summary_json 必须是对象".to_string(),
        )),
        None => Ok(Some(generated)),
    }
}

fn image_generation_parsed_output(
    parsed_output: Option<serde_json::Value>,
    image: &generated_image::Model,
) -> Result<Option<serde_json::Value>, AppError> {
    let generated = serde_json::json!({
        "generated_image_id": image.id,
        "project_id": image.project_id,
        "image_type": &image.image_type,
        "file_path": &image.file_path,
        "prompt": &image.prompt,
        "model": &image.model,
        "width": image.width,
        "height": image.height,
        "file_size": image.file_size,
        "chapter_id": image.chapter_id,
        "panel_id": &image.panel_id,
        "metadata": &image.metadata_json,
    });
    match parsed_output {
        Some(serde_json::Value::Object(mut map)) => {
            map.insert("generated_image".to_string(), generated);
            Ok(Some(serde_json::Value::Object(map)))
        }
        Some(_) => Err(AppError::InvalidArgument(
            "图片生成运行记录 parsed_output_json 必须是对象".to_string(),
        )),
        None => Ok(Some(generated)),
    }
}

fn image_generation_artifact_refs(
    artifact_refs: Option<serde_json::Value>,
    image: &generated_image::Model,
) -> Result<Option<serde_json::Value>, AppError> {
    Ok(Some(append_generation_artifact_target(
        artifact_refs,
        GenerationRunTraceTarget {
            target_kind: image.image_type.clone(),
            target_table: "generated_image".to_string(),
            target_id: image.id,
        },
    )?))
}

fn required_trimmed(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn validate_positive_i32(value: i32, field: &str) -> Result<i32, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

fn validate_positive_i64(value: i64, field: &str) -> Result<i64, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

fn validate_finite_f32(value: f32, field: &str) -> Result<f32, AppError> {
    if !value.is_finite() {
        return Err(AppError::InvalidArgument(format!("{field} 必须是有限数")));
    }
    Ok(value)
}

fn validate_update_record_clear_flags(input: &UpdateImageRecord) -> Result<(), AppError> {
    if input.clear_chapter_id && input.chapter_id.is_some() {
        return Err(AppError::InvalidArgument(
            "clear_chapter_id 不能与 chapter_id 同时使用".to_string(),
        ));
    }
    if input.clear_panel_id && input.panel_id.is_some() {
        return Err(AppError::InvalidArgument(
            "clear_panel_id 不能与 panel_id 同时使用".to_string(),
        ));
    }
    if input.clear_ai_score && input.ai_score.is_some() {
        return Err(AppError::InvalidArgument(
            "clear_ai_score 不能与 ai_score 同时使用".to_string(),
        ));
    }
    Ok(())
}

fn validate_update_record_image_type(current: &str, next: &str) -> Result<(), AppError> {
    let current = current.trim();
    let next = next.trim();
    if current == next {
        return Ok(());
    }
    if is_system_managed_image_type(current) || is_system_managed_image_type(next) {
        return Err(AppError::InvalidArgument(format!(
            "系统管理图片类型不能通过通用 update_image 修改：{current} -> {next}"
        )));
    }
    Ok(())
}

fn validate_update_record_chapter_binding(
    image: &generated_image::Model,
    next: Option<i64>,
) -> Result<(), AppError> {
    if image.chapter_id == next {
        return Ok(());
    }
    if is_system_managed_image_type(&image.image_type) {
        return Err(AppError::InvalidArgument(format!(
            "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id={} {:?} -> {:?}",
            image.id, image.chapter_id, next
        )));
    }
    Ok(())
}

fn validate_update_record_panel_binding(
    image: &generated_image::Model,
    next: Option<&str>,
) -> Result<(), AppError> {
    if image.panel_id.as_deref() == next {
        return Ok(());
    }
    if is_system_managed_image_type(&image.image_type) {
        return Err(AppError::InvalidArgument(format!(
            "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id={} {:?} -> {:?}",
            image.id,
            image.panel_id.as_deref(),
            next
        )));
    }
    Ok(())
}

fn is_system_managed_image_type(image_type: &str) -> bool {
    matches!(
        image_type.trim(),
        "scene"
            | "manga_page"
            | "scene_material"
            | "object_material"
            | "video_key_frame"
            | "video_start_frame"
            | "video_storyboard_frame"
            | "video_end_frame"
            | "video_tail_frame"
            | "canvas_image"
            | "canvas_frame"
            | "portrait"
            | "portrait_reference"
            | "chapter_reference"
            | "project_reference"
    )
}

fn generated_image_page_number(metadata: Option<&serde_json::Value>) -> Option<i32> {
    metadata
        .and_then(|value| value.get("page_number"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn should_delete_manga_page_image(
    image: &generated_image::Model,
    panel_id: &str,
    page_number: i32,
    preserve_image_id: Option<i64>,
) -> bool {
    if Some(image.id) == preserve_image_id || image.image_type != "manga_page" {
        return false;
    }
    image.panel_id.as_deref() == Some(panel_id)
        || generated_image_page_number(image.metadata_json.as_ref()) == Some(page_number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::chapter::chapter_manga_prompt;
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: DatabaseConnection) -> ImageService {
        ImageService::new(Arc::new(db))
    }

    fn image_model() -> generated_image::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        generated_image::Model {
            id: 9,
            project_id: 1,
            image_type: "scene".to_string(),
            file_path: "images/scene.png".to_string(),
            prompt: "draw scene".to_string(),
            model: "image-model".to_string(),
            width: 1024,
            height: 576,
            file_size: Some(1234),
            ai_score: None,
            is_favorite: false,
            chapter_id: Some(2),
            panel_id: Some("panel-1".to_string()),
            metadata_json: Some(json!({"source": "test"})),
            created_at: now,
        }
    }

    #[test]
    fn validate_finite_f32_rejects_non_finite_scores() {
        assert_eq!(validate_finite_f32(0.8, "ai_score").unwrap(), 0.8);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let err = validate_finite_f32(value, "ai_score")
                .expect_err("non-finite score must be rejected");
            assert!(matches!(
                err,
                AppError::InvalidArgument(message) if message == "ai_score 必须是有限数"
            ));
        }
    }

    #[test]
    fn update_record_clear_flags_reject_field_value_conflicts() {
        let cases = [
            (
                UpdateImageRecord {
                    chapter_id: Some(1),
                    clear_chapter_id: true,
                    ..Default::default()
                },
                "clear_chapter_id 不能与 chapter_id 同时使用",
            ),
            (
                UpdateImageRecord {
                    panel_id: Some("panel-1".to_string()),
                    clear_panel_id: true,
                    ..Default::default()
                },
                "clear_panel_id 不能与 panel_id 同时使用",
            ),
            (
                UpdateImageRecord {
                    ai_score: Some(0.8),
                    clear_ai_score: true,
                    ..Default::default()
                },
                "clear_ai_score 不能与 ai_score 同时使用",
            ),
        ];

        for (input, expected) in cases {
            let err = validate_update_record_clear_flags(&input)
                .expect_err("conflicting update input must be rejected");
            assert!(matches!(
                err,
                AppError::InvalidArgument(message) if message == expected
            ));
        }
    }

    #[test]
    fn update_record_image_type_rejects_system_managed_type_transitions() {
        validate_update_record_image_type("manga_page", " manga_page ")
            .expect("same managed image type should be allowed");
        validate_update_record_image_type("custom_reference", "custom_board")
            .expect("custom image type edits should be allowed");

        let cases = [
            ("manga_page", "custom_reference"),
            ("custom_reference", "manga_page"),
            ("scene_material", "object_material"),
            ("video_tail_frame", "project_reference"),
        ];
        for (current, next) in cases {
            let err = validate_update_record_image_type(current, next)
                .expect_err("managed image type transition must be rejected");
            assert!(matches!(
                err,
                AppError::InvalidArgument(message)
                    if message
                        == format!(
                            "系统管理图片类型不能通过通用 update_image 修改：{current} -> {next}"
                        )
            ));
        }
    }

    #[test]
    fn update_record_binding_rejects_system_managed_field_changes() {
        let image = image_model();
        validate_update_record_chapter_binding(&image, Some(2))
            .expect("same managed chapter binding should be allowed");
        validate_update_record_panel_binding(&image, Some("panel-1"))
            .expect("same managed panel binding should be allowed");

        let chapter_err = validate_update_record_chapter_binding(&image, Some(3))
            .expect_err("managed chapter binding change must be rejected");
        assert!(matches!(
            chapter_err,
            AppError::InvalidArgument(message)
                if message
                    == "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id=9 Some(2) -> Some(3)"
        ));

        let clear_chapter_err = validate_update_record_chapter_binding(&image, None)
            .expect_err("managed chapter binding clear must be rejected");
        assert!(matches!(
            clear_chapter_err,
            AppError::InvalidArgument(message)
                if message
                    == "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id=9 Some(2) -> None"
        ));

        let panel_err = validate_update_record_panel_binding(&image, Some("panel-2"))
            .expect_err("managed panel binding change must be rejected");
        assert!(matches!(
            panel_err,
            AppError::InvalidArgument(message)
                if message
                    == "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id=9 Some(\"panel-1\") -> Some(\"panel-2\")"
        ));

        let clear_panel_err = validate_update_record_panel_binding(&image, None)
            .expect_err("managed panel binding clear must be rejected");
        assert!(matches!(
            clear_panel_err,
            AppError::InvalidArgument(message)
                if message
                    == "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id=9 Some(\"panel-1\") -> None"
        ));
    }

    #[test]
    fn update_record_binding_allows_custom_field_changes() {
        let mut image = image_model();
        image.image_type = "custom_reference".to_string();

        validate_update_record_chapter_binding(&image, Some(3))
            .expect("custom chapter binding change should be allowed");
        validate_update_record_chapter_binding(&image, None)
            .expect("custom chapter binding clear should be allowed");
        validate_update_record_panel_binding(&image, Some("panel-2"))
            .expect("custom panel binding change should be allowed");
        validate_update_record_panel_binding(&image, None)
            .expect("custom panel binding clear should be allowed");
    }

    async fn insert_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        novel_project::ActiveModel {
            user_id: Set(1),
            title: Set(title.to_string()),
            description: Set("图片测试项目".to_string()),
            project_type: Set("novel_long".to_string()),
            project_kind: Set("text".to_string()),
            source_sync_status: Set("current".to_string()),
            status: Set("writing".to_string()),
            target_chapters: Set(10),
            current_chapter: Set(0),
            total_chapters: Set(Some(10)),
            chapters_per_part: Set(Some(5)),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn insert_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("图片章节".to_string())),
            content: Set("章节正文".to_string()),
            word_count: Set(4),
            status: Set("draft".to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn insert_manga_prompt(
        db: &DatabaseConnection,
        generated_image_id: Option<i64>,
    ) -> chapter_manga_prompt::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        chapter_manga_prompt::ActiveModel {
            chapter_id: Set(1),
            project_id: Set(1),
            page_number: Set(1),
            panel_number: Set(1),
            prompt_type: Set("page_generation".to_string()),
            image_prompt: Set("draw page".to_string()),
            dialogue_text: Set(None),
            characters_in_frame: Set(None),
            layout: Set(None),
            generated_image_id: Set(generated_image_id),
            camera_angle: Set(None),
            dialogue_position: Set(None),
            status: Set("completed".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn insert_frame_prompt(
        db: &DatabaseConnection,
        project_id: i64,
        chapter_id: i64,
        shot_number: i32,
        reference_image_ids: Vec<i64>,
    ) -> chapter_video_prompt::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        chapter_video_prompt::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            style: Set(Some("测试风格".to_string())),
            prompt_type: Set("frame_prompt".to_string()),
            workflow_mode: Set("boundary_frames".to_string()),
            shot_number: Set(shot_number),
            title: Set(Some(format!("边界帧 {shot_number}"))),
            scene_summary: Set(Some("边界帧摘要".to_string())),
            visual_prompt: Set("边界帧提示词".to_string()),
            motion_prompt: Set(None),
            negative_prompt: Set(Some("低质量".to_string())),
            duration_secs: Set(5),
            aspect_ratio: Set(Some("16:9".to_string())),
            reference_image_ids_json: Set(Some(json!(reference_image_ids.clone()))),
            status: Set("pending".to_string()),
            generated_video_id: Set(None),
            metadata_json: Set(Some(json!({
                "boundary_frame": {
                    "frame_number": shot_number,
                    "reference_image_ids": reference_image_ids
                }
            }))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn generated_image_count(db: &DatabaseConnection) -> u64 {
        generated_image::Entity::find().count(db).await.unwrap()
    }

    fn image_record(project_id: i64) -> CreateImageRecord {
        CreateImageRecord {
            project_id,
            image_type: "scene".to_string(),
            file_path: "images/scene.png".to_string(),
            prompt: "draw scene".to_string(),
            model: "test-model".to_string(),
            width: 512,
            height: 512,
            file_size: None,
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        }
    }

    async fn insert_image(
        db: &DatabaseConnection,
        service: &ImageService,
    ) -> generated_image::Model {
        let project = insert_project(db, "图片项目").await;
        service
            .create_record(image_record(project.id))
            .await
            .unwrap()
    }

    #[test]
    fn image_generation_run_json_requires_object_shape() {
        let image = image_model();
        let summary = image_generation_response_summary(Some(json!({"format": "png"})), &image)
            .expect("object response summary should accept")
            .expect("response summary should be present");
        assert_eq!(summary["generated_image_id"], image.id);
        assert_eq!(summary["format"], "png");

        let parsed = image_generation_parsed_output(Some(json!({"provider": "test"})), &image)
            .expect("object parsed output should accept")
            .expect("parsed output should be present");
        assert_eq!(parsed["provider"], "test");
        assert_eq!(parsed["generated_image"]["generated_image_id"], image.id);

        let err = image_generation_response_summary(Some(json!(["legacy-summary"])), &image)
            .expect_err("non-object response summary must fail explicitly");
        assert!(err.to_string().contains("response_summary_json 必须是对象"));

        let err = image_generation_parsed_output(Some(json!(["legacy-output"])), &image)
            .expect_err("non-object parsed output must fail explicitly");
        assert!(err.to_string().contains("parsed_output_json 必须是对象"));
    }

    #[tokio::test]
    async fn create_rejects_non_positive_dimensions_before_insert() {
        let db = setup_db().await;
        let service = service(db.clone());

        let err = service
            .create_record(CreateImageRecord {
                width: 0,
                ..image_record(1)
            })
            .await
            .expect_err("zero-width image must be rejected");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message.contains("width 必须大于 0")
        ));
        assert_eq!(generated_image_count(&db).await, 0);
    }

    #[tokio::test]
    async fn create_normalizes_required_text_and_blank_panel_id() {
        let db = setup_db().await;
        let project = insert_project(&db, "图片项目").await;
        let chapter = insert_chapter(&db, project.id).await;
        let service = service(db);

        let image = service
            .create_record(CreateImageRecord {
                image_type: " scene ".to_string(),
                file_path: " images/scene.png ".to_string(),
                prompt: " draw scene ".to_string(),
                model: " test-model ".to_string(),
                chapter_id: Some(chapter.id),
                panel_id: Some("   ".to_string()),
                metadata_json: Some(json!({"source": "test"})),
                ..image_record(project.id)
            })
            .await
            .expect("valid image should be inserted");

        assert_eq!(image.image_type, "scene");
        assert_eq!(image.file_path, "images/scene.png");
        assert_eq!(image.prompt, "draw scene");
        assert_eq!(image.model, "test-model");
        assert_eq!(image.panel_id, None);
    }

    #[tokio::test]
    async fn create_boundary_frame_image_does_not_update_next_frame_reference() {
        let db = setup_db().await;
        let project = insert_project(&db, "图片项目").await;
        let chapter = insert_chapter(&db, project.id).await;
        let first_prompt = insert_frame_prompt(&db, project.id, chapter.id, 1, vec![3, 9]).await;
        let next_prompt =
            insert_frame_prompt(&db, project.id, chapter.id, 2, vec![3, 9, 999_999]).await;
        let service = service(db.clone());

        service
            .create_record(CreateImageRecord {
                project_id: project.id,
                image_type: "video_key_frame".to_string(),
                file_path: "images/frame-1.png".to_string(),
                prompt: "边界帧提示词".to_string(),
                model: "image-model".to_string(),
                width: 1280,
                height: 720,
                file_size: Some(1024),
                chapter_id: Some(chapter.id),
                panel_id: None,
                metadata_json: Some(json!({
                    "generation_context": {
                        "source": "video_boundary_frame_prompt",
                        "prompt_id": first_prompt.id
                    }
                })),
                created_at: None,
            })
            .await
            .unwrap();

        let updated = chapter_video_prompt::Entity::find_by_id(next_prompt.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated.reference_image_ids_json,
            Some(json!([3, 9, 999_999]))
        );
        assert_eq!(
            updated
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("boundary_frame"))
                .and_then(|frame| frame.get("reference_image_ids")),
            Some(&json!([3, 9, 999_999]))
        );
    }

    #[tokio::test]
    async fn create_rejects_missing_project_without_insert() {
        let db = setup_db().await;
        let service = service(db.clone());

        let err = service
            .create_record(image_record(404))
            .await
            .expect_err("missing project must not create orphan image");

        assert!(matches!(
            err,
            AppError::NotFound(message) if message == "项目不存在: 404"
        ));
        assert_eq!(generated_image_count(&db).await, 0);
    }

    #[tokio::test]
    async fn create_rejects_chapter_from_another_project_without_insert() {
        let db = setup_db().await;
        let first_project = insert_project(&db, "项目一").await;
        let second_project = insert_project(&db, "项目二").await;
        let other_chapter = insert_chapter(&db, second_project.id).await;
        let service = service(db.clone());

        let err = service
            .create_record(CreateImageRecord {
                chapter_id: Some(other_chapter.id),
                ..image_record(first_project.id)
            })
            .await
            .expect_err("cross-project chapter binding must be rejected");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message == format!("章节 #{} 不属于项目 {}", other_chapter.id, first_project.id)
        ));
        assert_eq!(generated_image_count(&db).await, 0);
    }

    #[tokio::test]
    async fn update_rejects_non_finite_ai_score_without_update() {
        let db = setup_db().await;
        let service = service(db.clone());
        let image = insert_image(&db, &service).await;

        let err = service
            .update(
                image.id,
                UpdateImageRecord {
                    ai_score: Some(f32::NAN),
                    ..Default::default()
                },
            )
            .await
            .expect_err("NaN score must be rejected");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message.contains("ai_score 必须是有限数")
        ));
        let stored = service.get_image(image.id).await.unwrap();
        assert_eq!(stored.ai_score, None);
    }

    #[tokio::test]
    async fn update_sets_ai_score_with_finite_value() {
        let db = setup_db().await;
        let service = service(db.clone());
        let image = insert_image(&db, &service).await;

        let updated = service
            .update(
                image.id,
                UpdateImageRecord {
                    ai_score: Some(0.8),
                    ..Default::default()
                },
            )
            .await
            .expect("finite score should update");

        assert_eq!(updated.ai_score, Some(0.8));
        let stored = service.get_image(image.id).await.unwrap();
        assert_eq!(stored.ai_score, Some(0.8));
    }

    #[tokio::test]
    async fn delete_missing_image_returns_not_found_without_touching_orphan_references() {
        let db = setup_db().await;
        let prompt = insert_manga_prompt(&db, Some(999)).await;
        let service = service(db.clone());

        let err = service
            .delete(999)
            .await
            .expect_err("missing image deletion must be visible");

        assert!(matches!(err, AppError::NotFound(message) if message.contains("图片不存在")));
        let stored = chapter_manga_prompt::Entity::find_by_id(prompt.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.generated_image_id, Some(999));
        assert_eq!(stored.status, "completed");
    }

    #[tokio::test]
    async fn delete_existing_image_clears_manga_prompt_reference() {
        let db = setup_db().await;
        let project = insert_project(&db, "图片项目").await;
        let chapter = insert_chapter(&db, project.id).await;
        let service = service(db.clone());
        let image = service
            .create_record(CreateImageRecord {
                image_type: "manga_page".to_string(),
                file_path: "images/page1.png".to_string(),
                prompt: "draw page".to_string(),
                chapter_id: Some(chapter.id),
                panel_id: Some("page1".to_string()),
                ..image_record(project.id)
            })
            .await
            .unwrap();
        let prompt = insert_manga_prompt(&db, Some(image.id)).await;

        service
            .delete(image.id)
            .await
            .expect("existing image should be deleted");

        assert_eq!(generated_image_count(&db).await, 0);
        let stored = chapter_manga_prompt::Entity::find_by_id(prompt.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.generated_image_id, None);
        assert_eq!(stored.status, "pending");
    }

    #[test]
    fn delete_manga_page_image_filter_preserves_replacement_image() {
        let mut old_image = image_model();
        old_image.id = 10;
        old_image.image_type = "manga_page".to_string();
        old_image.panel_id = Some("page1".to_string());

        let mut replacement = old_image.clone();
        replacement.id = 11;
        replacement.file_path = "images/page-new.png".to_string();

        assert!(should_delete_manga_page_image(
            &old_image,
            "page1",
            1,
            Some(replacement.id)
        ));
        assert!(!should_delete_manga_page_image(
            &replacement,
            "page1",
            1,
            Some(replacement.id)
        ));
    }
}
