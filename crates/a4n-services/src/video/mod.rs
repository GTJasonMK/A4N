//! 视频生成服务：章节视频提示词与视频产物管理

mod config;
mod library;
mod prompt_context;
mod prompt_generation;
mod prompt_json;
mod prompt_preview;
mod prompt_records;
mod prompt_store;
mod records;

use a4n_engine::llm::provider::LlmProvider;
use a4n_engine::prompt::engine::PromptEngine;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;

use crate::profile_material;
use crate::project::SOURCE_MODE_FREEFORM;
use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::novel_project;

pub use self::config::{
    UpdateVideoConfigParams, VideoConfigParams, duration_capability_for_config,
    pick_provider_for_config, validate_video_config_model, validate_video_config_params,
    validate_video_duration_for_config_secs, validate_video_max_concurrent,
};
pub use self::prompt_context::validate_prompt_options_with_reference_limit;
pub use self::prompt_json::{
    validate_direct_prompt_text, video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
pub use self::prompt_preview::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};
pub use self::prompt_records::{VideoPromptDownstreamDependencies, VideoPromptUpdateInput};
pub use self::records::{UpdateVideoRecord, mark_canvas_generated_video_refs_missing};
pub use crate::prompt_language::PromptOutputLanguage;

pub const SHOT_ANALYSIS_CONTRACT_VERSION: &str = "shot_language_analysis_v3";
pub const FRAME_SEGMENT_PLAN_CONTRACT_VERSION: &str = "director_continuity_v4";
pub const BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION: &str = "boundary_frame_unit_anchors_v2";
pub const FIRST_FRAME_CHAIN_CONTRACT_VERSION: &str = "first_frame_chain_units_v3";
pub const CHAT_VIDEO_CONTRACT_VERSION: &str = "chat_video_units_v3";
pub const STORYBOARD_PROMPT_CONTRACT_VERSION: &str = "storyboard_panel_sheet_v1";
pub const STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION: &str = "storyboard_panel_video_v1";
const FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT: usize = 1;

pub fn is_project_reusable_video_material_image(
    image_type: &str,
    metadata: Option<&Value>,
) -> bool {
    profile_material::is_project_reusable_video_material_image(image_type, metadata)
}

fn is_video_frame_input_metadata(metadata: &serde_json::Map<String, Value>) -> bool {
    profile_material::is_video_frame_input_metadata(Some(&Value::Object(metadata.clone())))
}

type VideoPromptProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVideoPromptOptions {
    pub style: String,
    #[serde(default)]
    pub prompt_output_language: Option<String>,
    pub prompt_batch_size: i32,
    pub duration_secs: i32,
    #[serde(default)]
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub video_library_image_ids: Vec<i64>,
    pub duration_capability: Option<VideoDurationCapability>,
    pub regenerate_existing: bool,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
}

pub struct ChapterVideoPromptStageInput<'a, 'source> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub source: ChapterVideoSourceContext<'source>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoDurationCapability {
    pub model: Option<String>,
    pub duration_mode: String,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub supported_durations_secs: Vec<i32>,
}

impl VideoDurationCapability {
    pub fn validate_duration(&self, duration_secs: i32) -> Result<(), AppError> {
        let fixed_mode = self.duration_mode == "fixed";
        if duration_secs < self.min_duration_secs || duration_secs > self.max_duration_secs {
            return Err(AppError::InvalidArgument(format!(
                "视频时长 {duration_secs}s 不在当前模型支持范围 {}-{}s 内",
                self.min_duration_secs, self.max_duration_secs
            )));
        }
        if fixed_mode && !self.supported_durations_secs.contains(&duration_secs) {
            return Err(AppError::InvalidArgument(format!(
                "视频时长 {duration_secs}s 不在当前模型固定时长档位 {:?} 内",
                self.supported_durations_secs
            )));
        }
        Ok(())
    }

    pub fn validate_duration_options(&self, duration_secs_options: &[i32]) -> Result<(), AppError> {
        for duration_secs in duration_secs_options {
            self.validate_duration(*duration_secs)?;
        }
        Ok(())
    }

    pub fn to_prompt_json(&self) -> Value {
        let fixed_mode = self.duration_mode == "fixed";
        json!({
            "model": self.model,
            "duration_mode": self.duration_mode,
            "default_duration_secs": self.default_duration_secs,
            "min_duration_secs": self.min_duration_secs,
            "max_duration_secs": self.max_duration_secs,
            "supported_durations_secs": self.supported_durations_secs,
            "rule": if fixed_mode {
                format!(
                    "每个镜头 duration_secs 必须从 supported_durations_secs 中选择，优先使用 default_duration_secs={}",
                    self.default_duration_secs
                )
            } else {
                format!(
                    "每个镜头 duration_secs 可以使用 {}-{} 秒范围内的任意整数，优先使用 default_duration_secs",
                    self.min_duration_secs, self.max_duration_secs
                )
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterVideoSourceContext<'a> {
    pub content: &'a str,
    pub summary: Option<&'a str>,
    pub analysis: Option<&'a Value>,
}

#[derive(Debug, Clone)]
pub struct CreateVideoRecord {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Clone)]
pub struct VideoService {
    db: Arc<DatabaseConnection>,
}

impl VideoService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn get_project(
        &self,
        project_id: i64,
    ) -> Result<a4n_core::models::project::novel_project::Model, AppError> {
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

    async fn get_chapter(&self, chapter_id: i64) -> Result<chapter::Model, AppError> {
        validate_positive_i64(chapter_id, "chapter_id")?;
        chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))
    }

    async fn ensure_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let project = self.get_project(project_id).await?;
        let chapter = self.get_chapter(chapter_id).await?;
        match project.project_kind.as_str() {
            "text" => {
                if chapter.project_id != project.id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{chapter_id} 不属于文本项目 {project_id}"
                    )));
                }
            }
            "video" => {
                if project.root_project_id != Some(chapter.project_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{chapter_id} 不属于视频项目 {project_id} 绑定的根文本项目"
                    )));
                }
            }
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "视频链路不支持项目类型 {other}: project_id={project_id}"
                )));
            }
        }
        Ok(())
    }

    pub async fn video_asset_project_ids(&self, project_id: i64) -> Result<Vec<i64>, AppError> {
        let project = self.get_project(project_id).await?;
        match project.project_kind.as_str() {
            "text" => Ok(vec![project.id]),
            "video" if project.source_mode == SOURCE_MODE_FREEFORM => Ok(vec![project.id]),
            "video" => Ok(vec![
                project.id,
                project.root_project_id.ok_or_else(|| {
                    AppError::InvalidArgument(format!("视频项目缺少 root_project_id: {project_id}"))
                })?,
            ]),
            other => Err(AppError::InvalidArgument(format!(
                "视频链路不支持项目类型 {other}: project_id={project_id}"
            ))),
        }
    }

    pub async fn ensure_video_reference_images_available(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        self.validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
            .await
            .map(|_| ())
    }

    pub async fn validated_video_reference_images(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        let ids = prompt_reference_image_ids(reference_image_ids_json)?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let allowed_project_ids = self.video_asset_project_ids(project_id).await?;
        let rows = generated_image::Entity::find()
            .filter(generated_image::Column::Id.is_in(ids.iter().copied()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut images_by_id = rows
            .into_iter()
            .map(|image| (image.id, image))
            .collect::<HashMap<_, _>>();

        let mut images = Vec::with_capacity(ids.len());
        for id in ids {
            let image = images_by_id
                .remove(&id)
                .ok_or_else(|| AppError::NotFound(format!("参考图片不存在: {id}")))?;
            if !allowed_project_ids.contains(&image.project_id) {
                return Err(AppError::InvalidArgument(format!(
                    "参考图片 #{id} 不属于视频项目 {project_id} 或其根文本项目"
                )));
            }
            if is_project_reusable_video_material_image(
                &image.image_type,
                image.metadata_json.as_ref(),
            ) {
                images.push(image);
                continue;
            }
            if image.chapter_id.is_some() && image.chapter_id != Some(chapter_id) {
                return Err(AppError::InvalidArgument(format!(
                    "参考图片 #{id} 不属于提示词章节 {chapter_id}"
                )));
            }
            images.push(image);
        }
        Ok(images)
    }

    async fn ensure_prompt_generation_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        options: &ChapterVideoPromptOptions,
    ) -> Result<(), AppError> {
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        let reference_ids_json = json!(options.reference_image_ids);
        self.ensure_video_reference_images_available(
            project_id,
            chapter_id,
            Some(&reference_ids_json),
        )
        .await
    }
}

fn validate_positive_i64(value: i64, field: &str) -> Result<i64, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

fn prompt_reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("reference_image_ids 必须是数组".to_string()))?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
            AppError::InvalidArgument(format!("reference_image_ids[{index}] 必须是正整数"))
        })?;
        if ids.contains(&id) {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids 包含重复 ID：{id}"
            )));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn video_llm_step_error(prompt_name: &str, err: AppError) -> AppError {
    match err {
        AppError::Timeout(message) => {
            AppError::Timeout(format!("视频提示词步骤 {prompt_name} 超时：{message}"))
        }
        AppError::Internal(error) => internal_video_error(format!(
            "视频提示词步骤 {prompt_name} 调用 LLM 失败：{error}"
        )),
        AppError::LlmError(message) => AppError::LlmError(format!(
            "视频提示词步骤 {prompt_name} 调用 LLM 失败：{message}"
        )),
        other => other,
    }
}

fn internal_video_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

#[cfg(test)]
mod tests;
