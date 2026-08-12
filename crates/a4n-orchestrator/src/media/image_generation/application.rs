use std::collections::{BTreeSet, HashMap};

use a4n_api_contract::manga::MangaReferenceBindingSummary;
use a4n_core::{
    error::AppError,
    models::{
        asset::object_profile,
        chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE, Model as MangaPagePrompt},
        scene::scene_profile,
    },
    source_spans::{
        ChapterSourceContext, metadata_source_content_sha256, source_metadata,
        source_text_hash as source_content_sha256,
    },
};
use a4n_domain::profile_material::ProfileMaterialKind;
use a4n_image::{ImageGenerationRequest, ImageReference};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    config::image_backend_supports_reference_images,
    manga::ensure_manga_source_can_write,
    profile_material::{
        ProfileMaterialStyleMetadata, object_profile_content, object_profile_image_prompt,
        profile_material_metadata, scene_profile_content, scene_profile_image_prompt,
    },
    project::ChapterMangaSourceSummary,
    video::{
        batch_item_reference_image_ids, video_frame_generation_label,
        video_frame_image_generation_context,
    },
};

use super::{
    super::{
        BatchImageExecutionItem, BatchImageGenerationApplicationInput,
        batch_image_rules::{
            attach_video_frame_reference_image_ids, batch_image_aspect_ratio,
            ensure_batch_image_item_shape, ensure_batch_reference_image_backend,
            image_dimensions_for_aspect_ratio, optional_json_i64_field, optional_json_string_field,
        },
    },
    contracts::{
        BatchImageGenerationInput, GenerateCanvasImageInput, GenerateGalgameImageInput,
        ImageGenerationExecutionInput, ImageGenerationHostPort, ImageGenerationPostCommitAction,
        ImageGenerationProgress, ImageGenerationWorkflowBinding, ObjectProfileImageInput,
        ObjectProfileImagePromptOutput, PageImageGenerationInput, PreparedImageGenerationRequest,
        SceneImageGenerationInput, SceneProfileImageInput, SceneProfileImagePromptOutput,
    },
};

const IMAGE_METADATA_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageChapterBindingAccess {
    ReadOnly,
    Writable,
}

struct DirectImagePlan {
    project_id: i64,
    prompt: String,
    image_type: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    metadata_json: Option<Value>,
    workflow_run_id: Option<i64>,
    command: String,
    file_extra: Option<String>,
    file_context: String,
}

struct PreparedDirectImage {
    plan: DirectImagePlan,
    preview: PreparedImageGenerationRequest,
    chapter_id: Option<i64>,
    metadata_json: Option<Value>,
}

struct PreparedPageImage {
    preview: PreparedImageGenerationRequest,
    chapter: ChapterMangaSourceSummary,
    page_prompt: MangaPagePrompt,
    panel_id: String,
    metadata_json: Value,
}

#[derive(Clone, Copy)]
struct ProfileImageSpec {
    profile_kind: ProfileMaterialKind,
    image_type: &'static str,
    command: &'static str,
}

const OBJECT_PROFILE_IMAGE_SPEC: ProfileImageSpec = ProfileImageSpec {
    profile_kind: ProfileMaterialKind::Object,
    image_type: "object_material",
    command: "image_object_profile",
};
const SCENE_PROFILE_IMAGE_SPEC: ProfileImageSpec = ProfileImageSpec {
    profile_kind: ProfileMaterialKind::Scene,
    image_type: "scene_material",
    command: "image_scene_profile",
};

struct ProfileImageMaterial {
    spec: ProfileImageSpec,
    profile_id: i64,
    project_id: i64,
    name: String,
    prompt: String,
    profile_content: String,
    source_material: String,
    style_metadata: ProfileMaterialStyleMetadata,
}

struct PreparedBatchImageJob {
    execution: BatchImageExecutionItem,
    preview_notes: Vec<String>,
}

impl OrchestratorContext {
    pub async fn prepare_scene_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: SceneImageGenerationInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        self.prepare_direct_image(
            host,
            scene_image_plan(input),
            ImageChapterBindingAccess::ReadOnly,
        )
        .await
        .map(|prepared| prepared.preview)
    }

    pub async fn generate_scene_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: SceneImageGenerationInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        self.generate_direct_image(host, scene_image_plan(input), progress)
            .await
    }

    pub async fn prepare_canvas_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateCanvasImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        self.prepare_direct_image(
            host,
            canvas_image_plan(input),
            ImageChapterBindingAccess::ReadOnly,
        )
        .await
        .map(|prepared| prepared.preview)
    }

    pub async fn generate_galgame_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateGalgameImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        if input.generation_spec_id <= 0 || input.target_entity_id <= 0 {
            return Err(AppError::InvalidArgument(
                "generation_spec_id 和 target_entity_id 必须是正整数".to_string(),
            ));
        }
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        self.generate_direct_image(host, galgame_image_plan(input), progress)
            .await
    }

    pub async fn generate_object_profile_image_prompt_validated(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ObjectProfileImagePromptOutput, AppError> {
        let material = self.object_profile_image_material(user_id, input).await?;
        Ok(ObjectProfileImagePromptOutput {
            object_profile_id: material.profile_id,
            object_name: material.name,
            prompt: material.prompt,
        })
    }

    pub async fn generate_scene_profile_image_prompt_validated(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<SceneProfileImagePromptOutput, AppError> {
        let material = self.scene_profile_image_material(user_id, input).await?;
        Ok(SceneProfileImagePromptOutput {
            scene_profile_id: material.profile_id,
            scene_name: material.name,
            prompt: material.prompt,
        })
    }

    pub async fn prepare_object_profile_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        let config_id = input.config_id;
        let chapter_id = input.chapter_id;
        let material = self.object_profile_image_material(user_id, &input).await?;
        self.prepare_profile_image_request(host, material, config_id, chapter_id)
            .await
    }

    pub async fn generate_object_profile_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        let config_id = input.config_id;
        let chapter_id = input.chapter_id;
        let material = self.object_profile_image_material(user_id, &input).await?;
        self.generate_profile_image(host, material, config_id, chapter_id, progress)
            .await
    }

    pub async fn generate_scene_profile_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: SceneProfileImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        let config_id = input.config_id;
        let chapter_id = input.chapter_id;
        let material = self.scene_profile_image_material(user_id, &input).await?;
        self.generate_profile_image(host, material, config_id, chapter_id, progress)
            .await
    }

    pub async fn prepare_page_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: PageImageGenerationInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.prepare_page_image(host, user_id, input)
            .await
            .map(|prepared| prepared.preview)
    }

    pub async fn generate_page_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: PageImageGenerationInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        progress.progress(0, 4, "正在读取整页漫画提示词");
        check_progress_cancelled(progress)?;
        let chapter_number = input.chapter_number;
        let page_number = input.page_number;
        let project_id = input.project_id;
        let prepared = self.prepare_page_image(host, user_id, input).await?;
        progress.progress(1, 4, "正在调用图片模型");
        let outcome = self
            .run_image_generation_validated(
                host,
                ImageGenerationExecutionInput {
                    project_id,
                    chapter_id: Some(prepared.chapter.chapter_id),
                    image_type: "manga_page".to_string(),
                    command: "image_page".to_string(),
                    file_extra: Some(format!("ch{chapter_number:02}_pg{page_number:02}")),
                    file_context: "漫画页图片".to_string(),
                    width: prepared.preview.config.default_width,
                    height: prepared.preview.config.default_height,
                    config: prepared.preview.config,
                    request: prepared.preview.request,
                    panel_id: Some(prepared.panel_id),
                    metadata_json: Some(prepared.metadata_json),
                    runtime_config_snapshot: host.runtime_config_snapshot(),
                    workflow: ImageGenerationWorkflowBinding::Owned,
                    post_commit: ImageGenerationPostCommitAction::MangaPage {
                        page_prompt_id: prepared.page_prompt.id,
                        page_number,
                    },
                },
                progress,
            )
            .await?;
        progress.progress(
            4,
            4,
            if outcome.restored {
                "整页漫画已从断点恢复"
            } else {
                "整页漫画已生成"
            },
        );
        Ok(outcome.summary)
    }

    pub async fn prepare_batch_images_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: BatchImageGenerationInput,
    ) -> Result<Vec<PreparedImageGenerationRequest>, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        let requests = input
            .requests_json
            .as_array()
            .ok_or_else(|| AppError::InvalidArgument("requests_json 必须是数组".to_string()))?;
        let jobs = self
            .build_batch_image_jobs(host, input.project_id, requests, false)
            .await?;
        Ok(jobs
            .into_iter()
            .map(|job| PreparedImageGenerationRequest {
                project_id: input.project_id,
                image_type: job.execution.image_type,
                prompt_name: None,
                subject_name: None,
                preview_notes: job.preview_notes,
                config: job.execution.config,
                request: job.execution.request,
            })
            .collect())
    }

    pub async fn generate_batch_images_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: BatchImageGenerationInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::super::BatchImageGenerationOutcome, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        let requests = input
            .requests_json
            .as_array()
            .ok_or_else(|| AppError::InvalidArgument("requests_json 必须是数组".to_string()))?;
        if requests.is_empty() {
            return Err(AppError::InvalidArgument(
                "批量图片请求不能为空".to_string(),
            ));
        }
        let total = u32::try_from(requests.len()).map_err(|_| {
            AppError::InvalidArgument(format!("图片生成请求数量超过 u32 上限：{}", requests.len()))
        })?;
        progress.progress(0, total.max(1), "正在准备批量图片请求");
        let items = self
            .build_batch_image_jobs(host, input.project_id, requests, true)
            .await?
            .into_iter()
            .map(|job| job.execution)
            .collect();
        self.run_batch_image_generation_validated(
            host,
            BatchImageGenerationApplicationInput {
                project_id: input.project_id,
                requests_json: input.requests_json,
                items,
                runtime_config_snapshot: host.runtime_config_snapshot(),
            },
            progress,
        )
        .await
    }

    async fn prepare_profile_image_request(
        &self,
        host: &dyn ImageGenerationHostPort,
        material: ProfileImageMaterial,
        config_id: Option<i64>,
        chapter_id: Option<i64>,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        let subject_name = material.name.clone();
        let prepared = self
            .prepare_direct_image(
                host,
                profile_image_plan(material, config_id, chapter_id),
                ImageChapterBindingAccess::ReadOnly,
            )
            .await?;
        Ok(PreparedImageGenerationRequest {
            subject_name: Some(subject_name),
            ..prepared.preview
        })
    }

    async fn generate_profile_image(
        &self,
        host: &dyn ImageGenerationHostPort,
        material: ProfileImageMaterial,
        config_id: Option<i64>,
        chapter_id: Option<i64>,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        self.generate_direct_image(
            host,
            profile_image_plan(material, config_id, chapter_id),
            progress,
        )
        .await
    }

    async fn generate_direct_image(
        &self,
        host: &dyn ImageGenerationHostPort,
        plan: DirectImagePlan,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::GeneratedImageSummary, AppError> {
        progress.progress(0, 3, "正在准备图片生成配置");
        check_progress_cancelled(progress)?;
        let prepared = self
            .prepare_direct_image(host, plan, ImageChapterBindingAccess::Writable)
            .await?;
        progress.progress(1, 3, "正在调用图片模型");
        let outcome = self
            .execute_prepared_direct_image(host, prepared, progress)
            .await?;
        progress.progress(
            3,
            3,
            if outcome.restored {
                "图片已从断点恢复"
            } else {
                "图片已保存"
            },
        );
        Ok(outcome.summary)
    }

    async fn prepare_direct_image(
        &self,
        host: &dyn ImageGenerationHostPort,
        mut plan: DirectImagePlan,
        access: ImageChapterBindingAccess,
    ) -> Result<PreparedDirectImage, AppError> {
        plan.image_type = required_text(&plan.image_type, "image_type")?.to_string();
        plan.prompt = required_text(&plan.prompt, "prompt")?.to_string();
        let (chapter_id, source_metadata) = self
            .resolve_image_chapter_binding(plan.project_id, plan.chapter_id, access)
            .await?;
        let config = host.resolve_image_config(plan.config_id).await?;
        let base_metadata = Some(json!({
            "config_id": config.id,
            "backend": config.backend,
            "image_type": plan.image_type,
        }));
        let metadata_json = merge_direct_image_metadata(base_metadata, plan.metadata_json.clone());
        let metadata_json = merge_source_metadata(metadata_json, source_metadata, true);
        let request = ImageGenerationRequest {
            prompt: plan.prompt.clone(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images: Vec::new(),
        };
        Ok(PreparedDirectImage {
            preview: PreparedImageGenerationRequest {
                project_id: plan.project_id,
                image_type: plan.image_type.clone(),
                prompt_name: None,
                subject_name: None,
                preview_notes: Vec::new(),
                config,
                request,
            },
            plan,
            chapter_id,
            metadata_json,
        })
    }

    async fn execute_prepared_direct_image(
        &self,
        host: &dyn ImageGenerationHostPort,
        prepared: PreparedDirectImage,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<super::ImageGenerationExecutionOutcome, AppError> {
        let workflow = prepared
            .plan
            .workflow_run_id
            .map_or(ImageGenerationWorkflowBinding::Owned, |workflow_run_id| {
                ImageGenerationWorkflowBinding::Attached { workflow_run_id }
            });
        self.run_image_generation_validated(
            host,
            ImageGenerationExecutionInput {
                project_id: prepared.plan.project_id,
                chapter_id: prepared.chapter_id,
                image_type: prepared.plan.image_type,
                command: prepared.plan.command,
                file_extra: prepared.plan.file_extra,
                file_context: prepared.plan.file_context,
                width: prepared.preview.config.default_width,
                height: prepared.preview.config.default_height,
                config: prepared.preview.config,
                request: prepared.preview.request,
                panel_id: prepared.plan.panel_id,
                metadata_json: prepared.metadata_json,
                runtime_config_snapshot: host.runtime_config_snapshot(),
                workflow,
                post_commit: ImageGenerationPostCommitAction::None,
            },
            progress,
        )
        .await
    }

    async fn prepare_page_image(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: PageImageGenerationInput,
    ) -> Result<PreparedPageImage, AppError> {
        self.require_image_project_editor(user_id, input.project_id)
            .await?;
        let chapter = self
            .chapter_manga_source_by_number(input.project_id, input.chapter_number, true)
            .await?;
        ensure_manga_source_can_write(&chapter, "生成漫画页图片")?;
        if chapter.content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "章节正文为空，无法生成漫画页图片".to_string(),
            ));
        }
        let page_prompt = self
            .manga_page_prompt_for_image_generation(
                input.project_id,
                chapter.chapter_id,
                input.page_number,
            )
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "第 {} 章第 {} 页没有整页漫画提示词，请先运行漫画流水线",
                    input.chapter_number, input.page_number
                ))
            })?;
        ensure_manga_page_panels_match_current_source(
            std::slice::from_ref(&page_prompt),
            &chapter.content,
        )?;
        let panel_id = format!("page{}", input.page_number);
        let reference_bindings = page_prompt
            .layout
            .as_ref()
            .and_then(|layout| layout.get("reference_bindings"))
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "漫画提示词 #{} 缺少 reference_bindings，请重新生成漫画提示词",
                    page_prompt.id
                ))
            })?;
        let reference_images = collect_bound_reference_images(host, &reference_bindings)?;
        let reference_image_count = reference_images.len();
        let config = host.resolve_image_config(None).await?;
        let reference_image_mode = if reference_images.is_empty() {
            "none".to_string()
        } else if image_backend_supports_reference_images(&config.backend) {
            "image_input".to_string()
        } else {
            return Err(AppError::InvalidArgument(format!(
                "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或解除本页参考图绑定",
                config.backend
            )));
        };
        let image_prompt = page_prompt.image_prompt.trim();
        if image_prompt.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 章第 {} 页整页漫画提示词为空，请先重新生成或补全提示词",
                input.chapter_number, input.page_number
            )));
        }
        let request = ImageGenerationRequest {
            prompt: image_prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images,
        };
        let metadata_json = json!({
            "config_id": config.id,
            "backend": config.backend,
            "chapter_number": input.chapter_number,
            "page_number": input.page_number,
            "panel_id": panel_id,
            "prompt_type": MANGA_PAGE_PROMPT_TYPE,
            "page_prompt_id": page_prompt.id,
            "generation_mode": "direct_page",
            "reference_bindings": reference_bindings,
            "reference_image_count": reference_image_count,
            "reference_image_mode": reference_image_mode,
            "source": image_source_metadata(&chapter.content),
        });
        Ok(PreparedPageImage {
            preview: PreparedImageGenerationRequest {
                project_id: input.project_id,
                image_type: "manga_page".to_string(),
                prompt_name: Some(MANGA_PAGE_PROMPT_TYPE.to_string()),
                subject_name: None,
                preview_notes: vec![format!(
                    "参考图数量：{reference_image_count}；参考图模式：{reference_image_mode}"
                )],
                config,
                request,
            },
            chapter,
            page_prompt,
            panel_id,
            metadata_json,
        })
    }

    async fn object_profile_image_material(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ProfileImageMaterial, AppError> {
        require_positive(input.object_profile_id, "object_profile_id")?;
        let profile = self.object_profile_by_id(input.object_profile_id).await?;
        self.require_image_project_editor(user_id, profile.project_id)
            .await?;
        Ok(object_profile_material(profile, input))
    }

    async fn scene_profile_image_material(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<ProfileImageMaterial, AppError> {
        require_positive(input.scene_profile_id, "scene_profile_id")?;
        let profile = self.scene_profile_by_id(input.scene_profile_id).await?;
        self.require_image_project_editor(user_id, profile.project_id)
            .await?;
        Ok(scene_profile_material(profile, input))
    }

    async fn require_image_project_editor(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        require_positive(project_id, "project_id")?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await
            .map(|_| ())
    }

    async fn resolve_image_chapter_binding(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
        access: ImageChapterBindingAccess,
    ) -> Result<(Option<i64>, Option<Value>), AppError> {
        require_positive(project_id, "project_id")?;
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let Some(chapter_id) = chapter_id else {
            return Ok((None, None));
        };
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节 #{chapter_id} 不存在")))?;
        let source_content = match project.project_kind.as_str() {
            "text" => {
                if chapter.project_id != project.id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{} 不属于文本项目 {project_id}",
                        chapter.id
                    )));
                }
                if chapter.status == "deleted" {
                    return Err(AppError::InvalidArgument(format!(
                        "第 {} 章已删除，不能绑定图片素材",
                        chapter.chapter_number
                    )));
                }
                if access == ImageChapterBindingAccess::Writable && chapter.status == "generating" {
                    return Err(AppError::InvalidArgument(format!(
                        "第 {} 章正在生成中，请等待完成后再绑定图片素材",
                        chapter.chapter_number
                    )));
                }
                chapter.content.clone()
            }
            "comic" | "video" | "galgame" => {
                if project.root_project_id != Some(chapter.project_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{} 不属于派生项目 {project_id} 绑定的根文本项目",
                        chapter.id
                    )));
                }
                let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
                    AppError::InvalidArgument(format!("派生项目缺少源文本快照: {project_id}"))
                })?;
                self.source_snapshot_chapter_by_snapshot_and_source(snapshot_id, chapter.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!(
                            "派生项目 {project_id} 的源快照不包含章节 {}",
                            chapter.id
                        ))
                    })?
                    .content
            }
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "图片链路不支持项目类型 {other}: project_id={project_id}"
                )));
            }
        };
        let source_metadata = if source_content.trim().is_empty() {
            None
        } else {
            Some(image_source_metadata(&source_content))
        };
        Ok((Some(chapter.id), source_metadata))
    }

    async fn build_batch_image_jobs(
        &self,
        host: &dyn ImageGenerationHostPort,
        project_id: i64,
        requests: &[Value],
        enforce_generation_locks: bool,
    ) -> Result<Vec<PreparedBatchImageJob>, AppError> {
        let mut jobs = Vec::with_capacity(requests.len());
        let mut video_frame_requests = BTreeSet::<(String, i64)>::new();
        for (index, item) in requests.iter().enumerate() {
            ensure_batch_image_item_shape(index, item).map_err(AppError::InvalidArgument)?;
            let prompt = item
                .get("prompt")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::InvalidArgument("每项必须含 prompt 字段".to_string()))?
                .trim();
            if prompt.is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "第 {} 项 prompt 不能为空",
                    index + 1
                )));
            }
            let image_type = item
                .get("image_type")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument("每项必须含非空 image_type 字段".to_string())
                })?
                .to_string();
            let chapter_id =
                optional_json_i64_field(item, "chapter_id").map_err(AppError::InvalidArgument)?;
            let panel_id = optional_json_string_field(item, "panel_id")
                .map_err(AppError::InvalidArgument)?
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string);
            let metadata_json = item
                .get("metadata_json")
                .filter(|value| !value.is_null())
                .cloned();
            let config_id =
                optional_json_i64_field(item, "config_id").map_err(AppError::InvalidArgument)?;
            let mut reference_image_ids =
                batch_item_reference_image_ids(item).map_err(AppError::InvalidArgument)?;
            let (chapter_id, source_metadata) = self
                .resolve_image_chapter_binding(
                    project_id,
                    chapter_id,
                    if enforce_generation_locks {
                        ImageChapterBindingAccess::Writable
                    } else {
                        ImageChapterBindingAccess::ReadOnly
                    },
                )
                .await?;
            let mut metadata_json = merge_source_metadata(metadata_json, source_metadata, false);
            let mut preview_notes = Vec::new();
            let video_frame_context = video_frame_image_generation_context(metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?
                .map(|context| {
                    (
                        context.source.to_string(),
                        context.prompt_id,
                        context.request_preview_note.map(ToString::to_string),
                    )
                });
            if let Some((source, prompt_id, request_preview_note)) = &video_frame_context {
                let key = (source.clone(), *prompt_id);
                if !video_frame_requests.insert(key) {
                    return Err(AppError::InvalidArgument(format!(
                        "批量成图包含重复的{}请求：视频帧提示词 #{}",
                        video_frame_generation_label(source),
                        prompt_id
                    )));
                }
                if enforce_generation_locks {
                    self.ensure_frame_prompt_image_generation_allowed(*prompt_id, source)
                        .await?;
                } else {
                    preview_notes.push(format!(
                        "{}绑定视频帧提示词 #{}；预览按当前有效参考图构造图片 provider 请求。",
                        video_frame_generation_label(source),
                        prompt_id
                    ));
                    if let Some(note) = request_preview_note {
                        preview_notes.push(note.clone());
                    }
                }
                let effective_ids = self
                    .effective_frame_prompt_image_reference_ids(
                        *prompt_id,
                        source,
                        &reference_image_ids,
                    )
                    .await?;
                if !enforce_generation_locks && effective_ids != reference_image_ids {
                    preview_notes
                        .push(format!("按边界帧连续性规则使用参考图：{:?}", effective_ids));
                }
                reference_image_ids = effective_ids;
                metadata_json =
                    attach_video_frame_reference_image_ids(metadata_json, &reference_image_ids)
                        .map_err(AppError::InvalidArgument)?;
            }
            let config = host.resolve_image_config(config_id).await?;
            let reference_images = self
                .collect_generated_reference_images_by_ids(
                    host,
                    project_id,
                    chapter_id,
                    video_frame_context.is_some(),
                    &reference_image_ids,
                )
                .await?;
            if !reference_images.is_empty() {
                ensure_batch_reference_image_backend(&config.backend)
                    .map_err(AppError::InvalidArgument)?;
                preview_notes.push(format!("参考图数量：{}", reference_images.len()));
            }
            let aspect_ratio = batch_image_aspect_ratio(item, metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
            let (width, height) = match aspect_ratio {
                Some(aspect_ratio) => image_dimensions_for_aspect_ratio(
                    config.default_width,
                    config.default_height,
                    &aspect_ratio,
                )
                .map_err(AppError::InvalidArgument)?,
                None => (config.default_width, config.default_height),
            };
            jobs.push(PreparedBatchImageJob {
                execution: BatchImageExecutionItem {
                    index,
                    request: ImageGenerationRequest {
                        prompt: prompt.to_string(),
                        width,
                        height,
                        model: config.model.clone(),
                        reference_images,
                    },
                    image_type,
                    chapter_id,
                    panel_id,
                    metadata_json,
                    config,
                },
                preview_notes,
            });
        }
        Ok(jobs)
    }

    async fn collect_generated_reference_images_by_ids(
        &self,
        host: &dyn ImageGenerationHostPort,
        project_id: i64,
        chapter_id: Option<i64>,
        video_frame_scope: bool,
        image_ids: &[i64],
    ) -> Result<Vec<ImageReference>, AppError> {
        if image_ids.is_empty() {
            return Ok(Vec::new());
        }
        let allowed_project_ids = if video_frame_scope {
            let chapter_id = chapter_id.ok_or_else(|| {
                AppError::InvalidArgument(
                    "视频帧参考图缺少章节上下文，无法校验引用范围".to_string(),
                )
            })?;
            self.ensure_video_reference_images_available(
                project_id,
                chapter_id,
                Some(&json!(image_ids)),
            )
            .await?;
            self.video_asset_project_ids(project_id).await?
        } else {
            vec![project_id]
        };
        let images_by_id: HashMap<_, _> = self
            .generated_images_by_ids(image_ids)
            .await?
            .into_iter()
            .map(|image| (image.id, image))
            .collect();
        let mut references = Vec::with_capacity(image_ids.len());
        for (index, image_id) in image_ids.iter().copied().enumerate() {
            let image = images_by_id
                .get(&image_id)
                .ok_or_else(|| AppError::NotFound(format!("参考图不存在：#{image_id}")))?;
            if !allowed_project_ids.contains(&image.project_id) {
                let scope = if video_frame_scope {
                    "当前视频项目或其根文本项目"
                } else {
                    "当前项目"
                };
                return Err(AppError::InvalidArgument(format!(
                    "参考图 #{} 不属于{scope}",
                    image_id
                )));
            }
            references.push(
                host.read_reference_image(&format!("参考图 {}", index + 1), &image.file_path)?,
            );
        }
        Ok(references)
    }
}

pub fn normalize_image_metadata(metadata_json: Option<Value>) -> Option<Value> {
    match metadata_json {
        Some(Value::Object(mut map)) => {
            map.entry("metadata_schema_version".to_string())
                .or_insert_with(|| json!(IMAGE_METADATA_SCHEMA_VERSION));
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
            "value": value,
        })),
        None => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
        })),
    }
}

fn scene_image_plan(input: SceneImageGenerationInput) -> DirectImagePlan {
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: "scene".to_string(),
        config_id: input.config_id,
        chapter_id: input.chapter_id,
        panel_id: None,
        metadata_json: None,
        workflow_run_id: None,
        command: "image_scene".to_string(),
        file_extra: None,
        file_context: "图片文件".to_string(),
    }
}

fn canvas_image_plan(input: GenerateCanvasImageInput) -> DirectImagePlan {
    let metadata_json = Some(json!({
        "canvas": {
            "graph_id": input.graph_id,
            "node_id": input.node_id,
            "workflow_run_id": input.workflow_run_id,
            "source_node_ids": input.source_node_ids,
        },
        "source": "video_canvas",
    }));
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: "canvas_image".to_string(),
        config_id: input.config_id,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command: "generate_canvas_image".to_string(),
        file_extra: None,
        file_context: "图片文件".to_string(),
    }
}

fn galgame_image_plan(input: GenerateGalgameImageInput) -> DirectImagePlan {
    let panel_id = format!(
        "galgame_{}_{}",
        input.target_entity_kind, input.target_entity_id
    );
    let metadata_json = Some(json!({
        "galgame": {
            "generation_spec_id": input.generation_spec_id,
            "target_kind": input.target_kind,
            "target_entity_kind": input.target_entity_kind,
            "target_entity_id": input.target_entity_id,
            "workflow_run_id": input.workflow_run_id,
        },
        "source": "galgame_asset_generation",
    }));
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: input.image_type,
        config_id: input.config_id,
        chapter_id: input.chapter_id,
        panel_id: Some(panel_id.clone()),
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command: "generate_galgame_image".to_string(),
        file_extra: Some(panel_id),
        file_context: "图片文件".to_string(),
    }
}

fn profile_image_plan(
    material: ProfileImageMaterial,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> DirectImagePlan {
    let panel_id = format!(
        "{}_profile_{}",
        material.spec.profile_kind.as_str(),
        material.profile_id
    );
    DirectImagePlan {
        project_id: material.project_id,
        prompt: material.prompt.clone(),
        image_type: material.spec.image_type.to_string(),
        config_id,
        chapter_id,
        panel_id: Some(panel_id.clone()),
        metadata_json: Some(profile_image_metadata(&material)),
        workflow_run_id: None,
        command: material.spec.command.to_string(),
        file_extra: Some(panel_id),
        file_context: "图片文件".to_string(),
    }
}

fn object_profile_material(
    profile: object_profile::Model,
    input: &ObjectProfileImageInput,
) -> ProfileImageMaterial {
    let prompt = object_profile_image_prompt(&profile, input.style.as_deref());
    let source_material = profile_source_material(
        &[
            profile.name.as_str(),
            profile.visual_description.as_str(),
            profile.material_and_state.as_str(),
            profile.story_usage.as_str(),
        ],
        &prompt,
    );
    let profile_content = object_profile_content(&profile);
    ProfileImageMaterial {
        spec: OBJECT_PROFILE_IMAGE_SPEC,
        profile_id: profile.id,
        project_id: profile.project_id,
        name: profile.name,
        prompt,
        profile_content,
        source_material,
        style_metadata: profile_style_metadata(
            input.style_template_id.clone(),
            input.style_template_name.clone(),
            input.style_template_version.clone(),
            input.custom_style.clone(),
        ),
    }
}

fn scene_profile_material(
    profile: scene_profile::Model,
    input: &SceneProfileImageInput,
) -> ProfileImageMaterial {
    let prompt = scene_profile_image_prompt(&profile, input.style.as_deref());
    let source_material = profile_source_material(
        &[
            profile.name.as_str(),
            profile.base_description.as_str(),
            profile.visual_description.as_str(),
            profile.spatial_layout.as_str(),
            profile.atmosphere.as_str(),
            profile.image_prompt_seed.as_str(),
        ],
        &prompt,
    );
    let profile_content = scene_profile_content(&profile);
    ProfileImageMaterial {
        spec: SCENE_PROFILE_IMAGE_SPEC,
        profile_id: profile.id,
        project_id: profile.project_id,
        name: profile.name,
        prompt,
        profile_content,
        source_material,
        style_metadata: profile_style_metadata(
            input.style_template_id.clone(),
            input.style_template_name.clone(),
            input.style_template_version.clone(),
            input.custom_style.clone(),
        ),
    }
}

fn profile_style_metadata(
    style_template_id: Option<String>,
    style_template_name: Option<String>,
    style_template_version: Option<String>,
    custom_style: Option<String>,
) -> ProfileMaterialStyleMetadata {
    ProfileMaterialStyleMetadata {
        style_template_id,
        style_template_name,
        style_template_version,
        custom_style,
    }
}

fn profile_source_material(parts: &[&str], prompt: &str) -> String {
    let mut source = parts.join("\n");
    source.push('\n');
    source.push_str(prompt);
    source
}

fn profile_image_metadata(material: &ProfileImageMaterial) -> Value {
    let mut metadata = profile_material_metadata(
        material.spec.profile_kind,
        material.profile_id,
        &material.name,
        &source_content_sha256(&material.profile_content),
        &material.style_metadata,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert(
            "source".to_string(),
            json!(format!("{}_profile", material.spec.profile_kind.as_str())),
        );
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(material.prompt));
        map.insert(
            "source_content_sha256".to_string(),
            json!(source_content_sha256(&material.source_material)),
        );
    }
    metadata
}

pub(super) fn collect_bound_reference_images(
    host: &dyn ImageGenerationHostPort,
    bindings: &Value,
) -> Result<Vec<ImageReference>, AppError> {
    let bindings = serde_json::from_value::<Vec<MangaReferenceBindingSummary>>(bindings.clone())
        .map_err(|error| AppError::InvalidArgument(format!("漫画参考图绑定合同无效: {error}")))?;
    let mut references = Vec::new();
    for (index, binding) in bindings.into_iter().enumerate() {
        let binding_context = format!("reference_bindings[{index}]");
        let asset = binding.bound_asset.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "参考图“{}”尚未绑定，无法生成漫画页图片",
                binding.character_name
            ))
        })?;
        let label = binding.character_name.trim();
        if label.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{binding_context}.character_name 不能为空"
            )));
        }
        let path = asset.image_path.trim();
        if path.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{binding_context}.bound_asset.image_path 不能为空"
            )));
        }
        references.push(host.read_reference_image(label, path)?);
    }
    Ok(references)
}

fn ensure_manga_page_panels_match_current_source(
    panels: &[chapter_manga_prompt::Model],
    current_content: &str,
) -> Result<(), AppError> {
    let current_hash = source_content_sha256(current_content);
    for panel in panels {
        let source = panel
            .layout
            .as_ref()
            .and_then(|layout| layout.get("source"));
        if source.is_none() {
            return Err(AppError::InvalidArgument(format!(
                "漫画提示词 #{} 缺少章节来源信息，请重新生成漫画提示词",
                panel.id
            )));
        }
        let Some(source_hash) = metadata_source_content_sha256(panel.layout.as_ref()) else {
            return Err(AppError::InvalidArgument(format!(
                "漫画提示词 #{} 缺少章节正文指纹，请重新生成漫画提示词",
                panel.id
            )));
        };
        if source_hash != current_hash {
            return Err(AppError::InvalidArgument(format!(
                "漫画提示词 #{} 基于旧章节正文，请重新生成漫画提示词",
                panel.id
            )));
        }
    }
    Ok(())
}

fn image_source_metadata(content: &str) -> Value {
    source_metadata(ChapterSourceContext { content })
}

fn merge_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
    preserve_existing_source: bool,
) -> Option<Value> {
    let Some(source_metadata) = source_metadata else {
        return metadata_json;
    };
    match metadata_json {
        Some(Value::Object(mut map)) => {
            let key = if preserve_existing_source && map.contains_key("source") {
                "chapter_source"
            } else {
                "source"
            };
            map.insert(key.into(), source_metadata);
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "source": source_metadata,
        })),
        None => Some(json!({
            "source": source_metadata,
        })),
    }
}

fn merge_direct_image_metadata(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            base.extend(extra);
            Some(Value::Object(base))
        }
        (Some(Value::Object(mut base)), Some(extra)) => {
            base.insert("extra".to_string(), extra);
            Some(Value::Object(base))
        }
        (Some(base), Some(Value::Object(mut extra))) => {
            extra.insert("base".to_string(), base);
            Some(Value::Object(extra))
        }
        (Some(base), Some(extra)) => Some(json!({
            "base": base,
            "extra": extra,
        })),
        (Some(base), None) => Some(base),
        (None, Some(extra)) => Some(extra),
        (None, None) => None,
    }
}

pub(super) fn check_progress_cancelled(
    progress: &ImageGenerationProgress<'_>,
) -> Result<(), AppError> {
    progress.check_cancelled().map_err(|_| AppError::Cancelled)
}

fn require_positive(value: i64, label: &str) -> Result<(), AppError> {
    if value <= 0 {
        Err(AppError::InvalidArgument(format!("{label} 必须大于 0")))
    } else {
        Ok(())
    }
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}
