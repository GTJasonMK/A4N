use a4n_core::{
    error::AppError,
    models::generation::{generation_spec, prompt_artifact},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

use super::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE,
    GALGAME_EXPRESSIONS, GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG,
    GALGAME_TARGET_PORTRAIT_EXPRESSION, GalgameAssetGenerationItem, GalgameAssetGenerationSummary,
    GalgameCgImportance, GalgameGenerationTargetKind, GalgameMediaBindingSummary, GalgameMediaRole,
    GalgameMediaTargetKind, GalgamePortraitSpecTargetInput, GalgamePromptArtifactUpsertInput,
    GalgameTargetContext, GalgameTargetEntityKind, RunGalgameAssetGenerationInput,
};
use crate::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING};
use crate::{
    OrchestratorContext,
    media::{GenerateGalgameImageInput, GeneratedImageSummary},
    portrait::{GeneratePortraitInput, PortraitSummary},
};

const GALGAME_PORTRAIT_VARIANT_STYLE: &str =
    "Galgame 视觉小说角色立绘表情差分；必须保持参考图中的角色身份、服装、体型比例和美术风格一致。";

#[async_trait]
pub trait GalgameAssetGenerationHostPort: Send + Sync {
    async fn render_image_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: i64,
    ) -> Result<String, AppError>;

    async fn generate_visual_image(
        &self,
        user_id: i64,
        input: GenerateGalgameImageInput,
    ) -> Result<GeneratedImageSummary, AppError>;

    async fn generate_portrait_image(
        &self,
        user_id: i64,
        input: GeneratePortraitInput,
    ) -> Result<PortraitSummary, AppError>;

    fn validate_data_file(&self, relative_path: &str, label: &str) -> Result<(), AppError>;
}

impl OrchestratorContext {
    pub async fn run_galgame_asset_generation_validated(
        &self,
        host_port: &dyn GalgameAssetGenerationHostPort,
        user_id: i64,
        input: RunGalgameAssetGenerationInput,
    ) -> Result<GalgameAssetGenerationSummary, AppError> {
        validate_input(&input)?;
        let specs = self
            .galgame_asset_generation_specs(
                input.project_id,
                &[
                    GALGAME_TARGET_BACKGROUND,
                    GALGAME_TARGET_CG,
                    GALGAME_TARGET_PORTRAIT_EXPRESSION,
                ],
                &input.generation_spec_ids,
                input.limit,
            )
            .await?;
        let mut summary = GalgameAssetGenerationSummary {
            project_id: input.project_id,
            generated_count: 0,
            skipped_completed_count: 0,
            items: Vec::with_capacity(specs.len()),
        };
        for spec in specs {
            let (item, generated) = match spec.target_kind.as_str() {
                GALGAME_TARGET_BACKGROUND | GALGAME_TARGET_CG => {
                    run_visual_spec(self, host_port, user_id, &input, &spec).await?
                }
                GALGAME_TARGET_PORTRAIT_EXPRESSION => {
                    run_portrait_spec(self, host_port, user_id, &input, &spec).await?
                }
                other => {
                    return Err(AppError::InvalidArgument(format!(
                        "Galgame 资产生成不支持 target_kind: spec_id={}, target_kind={other}",
                        spec.id
                    )));
                }
            };
            if generated {
                summary.generated_count += 1;
            } else {
                summary.skipped_completed_count += 1;
            }
            summary.items.push(item);
        }
        Ok(summary)
    }
}

struct GalgameVisualSpecPlan {
    target_entity_id: i64,
    target_kind: GalgameGenerationTargetKind,
    target: GalgameMediaTargetKind,
    role: GalgameMediaRole,
    prompt_kind: &'static str,
    image_type: &'static str,
    template_name: &'static str,
}

impl GalgameVisualSpecPlan {
    fn from_spec(spec: &generation_spec::Model) -> Result<Self, AppError> {
        let target_entity_id = spec.target_entity_id.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Galgame 视觉生成意图缺少 target_entity_id: {}",
                spec.id
            ))
        })?;
        if target_entity_id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 视觉生成意图 target_entity_id 无效: {}",
                spec.id
            )));
        }
        match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
            (GALGAME_TARGET_BACKGROUND, GALGAME_ENTITY_SCENE) => Ok(Self {
                target_entity_id,
                target_kind: GalgameGenerationTargetKind::GalgameBackground,
                target: GalgameMediaTargetKind::GalgameScene,
                role: GalgameMediaRole::Background,
                prompt_kind: "background_image_prompt",
                image_type: "galgame_background",
                template_name: "galgame_background_image_prompt",
            }),
            (GALGAME_TARGET_CG, GALGAME_ENTITY_LINE) => Ok(Self {
                target_entity_id,
                target_kind: GalgameGenerationTargetKind::GalgameCg,
                target: GalgameMediaTargetKind::GalgameLine,
                role: GalgameMediaRole::Cg,
                prompt_kind: "event_cg_prompt",
                image_type: "galgame_cg",
                template_name: "galgame_event_cg_prompt",
            }),
            _ => Err(AppError::InvalidArgument(format!(
                "Galgame 视觉生成只支持背景/CG spec: spec_id={}, target={}/{}",
                spec.id, spec.target_kind, spec.target_entity_kind
            ))),
        }
    }

    fn target_entity_kind(&self) -> GalgameTargetEntityKind {
        match self.target {
            GalgameMediaTargetKind::GalgameScene => GalgameTargetEntityKind::GalgameScene,
            GalgameMediaTargetKind::GalgameLine => GalgameTargetEntityKind::GalgameLine,
        }
    }
}

async fn run_visual_spec(
    context: &OrchestratorContext,
    host: &dyn GalgameAssetGenerationHostPort,
    user_id: i64,
    input: &RunGalgameAssetGenerationInput,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), AppError> {
    let plan = GalgameVisualSpecPlan::from_spec(spec)?;
    let target_context = context
        .galgame_media_target_context_validated(
            input.project_id,
            plan.target,
            plan.target_entity_id,
        )
        .await?;
    validate_visual_spec(spec, &plan, &target_context)?;
    if let Some(item) = completed_visual(context, host, spec, &plan).await? {
        return Ok((item, false));
    }
    context
        .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_RUNNING)
        .await?;
    let prompt = upsert_prompt(context, host, spec, &plan).await?;
    let image = match host
        .generate_visual_image(
            user_id,
            GenerateGalgameImageInput {
                project_id: input.project_id,
                prompt: prompt.prompt_text.clone(),
                image_type: plan.image_type.to_string(),
                config_id: input.config_id,
                width: input.width,
                height: input.height,
                chapter_id: spec.source_chapter_id,
                generation_spec_id: spec.id,
                target_kind: plan.target_kind.to_string(),
                target_entity_kind: plan.target_entity_kind().to_string(),
                target_entity_id: plan.target_entity_id,
                workflow_run_id: None,
            },
        )
        .await
    {
        Ok(image) => image,
        Err(error) => {
            context
                .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_FAILED)
                .await?;
            return Err(error);
        }
    };
    let binding = match persist_visual_asset(context, spec, &plan, &prompt, &image).await {
        Ok(binding) => binding,
        Err(error) => {
            context
                .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_FAILED)
                .await?;
            return Err(error);
        }
    };
    context
        .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_COMPLETED)
        .await?;
    Ok((
        GalgameAssetGenerationItem {
            generation_spec_id: spec.id,
            target_kind: plan.target_kind,
            target_entity_kind: plan.target_entity_kind(),
            target_entity_id: plan.target_entity_id,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            prompt_artifact_id: Some(prompt.id),
            generated_image_id: Some(image.id),
            portrait_id: None,
            media_asset_id: Some(binding.asset.id),
            binding_id: Some(binding.binding.id),
        },
        true,
    ))
}

async fn run_portrait_spec(
    context: &OrchestratorContext,
    host: &dyn GalgameAssetGenerationHostPort,
    user_id: i64,
    input: &RunGalgameAssetGenerationInput,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), AppError> {
    let plan = GalgamePortraitSpecPlan::from_spec(spec)?;
    let profile = context
        .galgame_portrait_spec_target_profile(
            spec,
            &GalgamePortraitSpecTargetInput {
                profile_id: plan.profile_id,
                expression: plan.expression.clone(),
                pose: plan.pose.clone(),
                position_samples: plan.position_samples.clone(),
                source_line_ids: plan.source_line_ids.clone(),
            },
        )
        .await?;
    if let Some(item) = completed_portrait(context, host, spec, &plan).await? {
        return Ok((item, false));
    }
    context
        .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_RUNNING)
        .await?;
    let reference_image_ids = match context
        .primary_galgame_portrait_reference_image_ids(&profile)
        .await
    {
        Ok(ids) => ids,
        Err(error) => {
            context
                .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_FAILED)
                .await?;
            return Err(error);
        }
    };
    let portrait = match host
        .generate_portrait_image(
            user_id,
            GeneratePortraitInput {
                project_id: spec.root_project_id,
                character_name: profile.name.clone(),
                style: Some(GALGAME_PORTRAIT_VARIANT_STYLE.to_string()),
                expression: Some(plan.expression.clone()),
                pose: Some(plan.pose.clone()),
                reference_image_ids,
                width: input.width,
                height: input.height,
            },
        )
        .await
    {
        Ok(portrait) => portrait,
        Err(error) => {
            context
                .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_FAILED)
                .await?;
            return Err(error);
        }
    };
    context
        .mark_galgame_generation_spec_status_record(spec.id, WORKFLOW_STATUS_COMPLETED)
        .await?;
    Ok((portrait_item(spec, &plan, portrait.id), true))
}

async fn upsert_prompt(
    context: &OrchestratorContext,
    host: &dyn GalgameAssetGenerationHostPort,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<prompt_artifact::Model, AppError> {
    if let Some(existing) = context
        .galgame_prompt_artifact_by_generation_spec(spec.id)
        .await?
        && existing.edited_by_user
    {
        return Ok(existing);
    }
    let variables = visual_prompt_variables(spec, plan)?;
    let prompt_text = host
        .render_image_prompt(plan.template_name, &variables, spec.project_id)
        .await?
        .trim()
        .to_string();
    if prompt_text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 图像提示词模板渲染为空: spec_id={}, template={}",
            spec.id, plan.template_name
        )));
    }
    context
        .upsert_galgame_prompt_artifact_record(GalgamePromptArtifactUpsertInput {
            generation_spec_id: spec.id,
            prompt_kind: plan.prompt_kind.to_string(),
            prompt_text,
            renderer_version: "galgame_asset_generation_v1".to_string(),
        })
        .await
}

fn visual_prompt_variables(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<HashMap<String, Value>, AppError> {
    let scene = required_object(&spec.params_json, "scene", spec.id)?;
    required_string(scene, "title", "scene.title", spec.id)?;
    let mut variables = HashMap::new();
    variables.insert("params".to_string(), spec.params_json.clone());
    variables.insert("scene".to_string(), scene.clone());
    variables.insert(
        "source".to_string(),
        spec.params_json
            .get("source")
            .cloned()
            .unwrap_or(Value::Null),
    );
    variables.insert(
        "canonical_key".to_string(),
        spec.params_json
            .get("canonical_key")
            .cloned()
            .unwrap_or(Value::Null),
    );
    if plan.target_kind == GalgameGenerationTargetKind::GalgameCg {
        let line = required_object(&spec.params_json, "line", spec.id)?;
        required_string(line, "text", "line.text", spec.id)?;
        let cg_note = required_string(&spec.params_json, "cg_note", "cg_note", spec.id)?;
        let cg_importance = required_cg_importance(&spec.params_json, spec.id)?;
        variables.insert("line".to_string(), line.clone());
        variables.insert("cg_note".to_string(), Value::String(cg_note.to_string()));
        variables.insert(
            "cg_importance".to_string(),
            Value::String(cg_importance.to_string()),
        );
    }
    Ok(variables)
}

async fn completed_visual(
    context: &OrchestratorContext,
    host: &dyn GalgameAssetGenerationHostPort,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, AppError> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let records = context
        .completed_galgame_visual_binding_records(
            spec,
            plan.target_entity_kind().as_str(),
            plan.target_entity_id,
            plan.role.as_str(),
        )
        .await?;
    let path = records
        .asset
        .file_path
        .as_deref()
        .ok_or_else(|| AppError::InvalidState {
            current: "galgame_asset_generation".to_string(),
            operation: format!("已完成 spec 缺少文件路径: {}", spec.id),
        })?;
    host.validate_data_file(path, "Galgame 视觉媒体文件")?;
    validate_request_snapshot(
        spec,
        plan.target_entity_id,
        records.asset.id,
        records.provenance.request_snapshot_json.as_ref(),
    )?;
    Ok(Some(GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: plan.target_kind,
        target_entity_kind: plan.target_entity_kind(),
        target_entity_id: plan.target_entity_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: records.provenance.prompt_artifact_id,
        generated_image_id: None,
        portrait_id: None,
        media_asset_id: Some(records.asset.id),
        binding_id: Some(records.binding.id),
    }))
}

async fn completed_portrait(
    context: &OrchestratorContext,
    host: &dyn GalgameAssetGenerationHostPort,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, AppError> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let portrait = context
        .completed_galgame_portrait_record(spec, plan.profile_id, &plan.expression, &plan.pose)
        .await?;
    host.validate_data_file(&portrait.image_path, "Galgame 立绘差分")?;
    Ok(Some(portrait_item(spec, plan, portrait.id)))
}

fn portrait_item(
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
    portrait_id: i64,
) -> GalgameAssetGenerationItem {
    GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: GalgameGenerationTargetKind::PortraitExpression,
        target_entity_kind: GalgameTargetEntityKind::CharacterProfile,
        target_entity_id: plan.profile_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: None,
        generated_image_id: None,
        portrait_id: Some(portrait_id),
        media_asset_id: None,
        binding_id: None,
    }
}

async fn persist_visual_asset(
    context: &OrchestratorContext,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    prompt: &prompt_artifact::Model,
    image: &GeneratedImageSummary,
) -> Result<GalgameMediaBindingSummary, AppError> {
    let generated = context
        .galgame_generated_image_by_id(image.id, spec.project_id)
        .await?;
    let generated_file_path = generated.file_path.trim();
    if generated_file_path.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 生成图片缺少文件路径: generated_image_id={}",
            generated.id
        )));
    }
    let project = context
        .project_by_id(spec.project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", spec.project_id)))?;
    let source_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        AppError::InvalidArgument(format!("Galgame 项目缺少源文本快照: {}", project.id))
    })?;
    let asset = context.create_media_asset(crate::media::CreateMediaAssetInput {
        project_id: spec.project_id, root_project_id: spec.root_project_id, source_snapshot_id: Some(source_snapshot_id), source_chapter_id: spec.source_chapter_id, source_snapshot_chapter_id: spec.source_snapshot_chapter_id,
        media_kind: crate::media::MEDIA_KIND_IMAGE.to_string(), origin: crate::media::MEDIA_ORIGIN_GENERATED.to_string(), file_path: Some(generated_file_path.to_string()), thumbnail_path: None, width: Some(generated.width), height: Some(generated.height), duration_secs: None, fps: None, status: crate::media::MEDIA_STATUS_COMPLETED.to_string(), error_message: None,
        provenance: crate::media::CreateMediaProvenanceInput { generation_spec_id: Some(spec.id), prompt_artifact_id: Some(prompt.id), provider_job_id: None, source_media_asset_id: None, source_external_path: Some(a4n_domain::GeneratedMediaReference::Image(generated.id).to_string()), imported_from_user: false, request_snapshot_json: Some(json!({"source":"galgame_asset_generation","generated_image_id":generated.id,"image_type":generated.image_type,"model":generated.model,"prompt":generated.prompt,"metadata_json":generated.metadata_json,"target_kind":spec.target_kind,"target_entity_kind":spec.target_entity_kind,"target_entity_id":plan.target_entity_id})) },
    }).await?;
    match context
        .bind_galgame_media_validated(super::BindGalgameMediaInput {
            project_id: spec.project_id,
            media_asset_id: asset.id,
            target_kind: plan.target,
            target_id: plan.target_entity_id,
            role: plan.role,
        })
        .await
    {
        Ok(binding) => Ok(binding),
        Err(error) => {
            let cleanup = context
                .delete_media_asset_with_provenance_records(asset.id)
                .await
                .err();
            Err(match cleanup {
                Some(cleanup) => crate::errors::app_error_with_secondary(
                    error,
                    format!("后续清理也失败：{cleanup}"),
                ),
                None => error,
            })
        }
    }
}

struct GalgamePortraitSpecPlan {
    profile_id: i64,
    expression: String,
    pose: String,
    position_samples: Vec<String>,
    source_line_ids: Vec<i64>,
}
impl GalgamePortraitSpecPlan {
    fn from_spec(spec: &generation_spec::Model) -> Result<Self, AppError> {
        let profile_id = spec.target_entity_id.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Galgame 立绘差分生成意图缺少 target_entity_id: {}",
                spec.id
            ))
        })?;
        if profile_id <= 0
            || spec.target_kind != GALGAME_TARGET_PORTRAIT_EXPRESSION
            || spec.target_entity_kind != GALGAME_ENTITY_CHARACTER_PROFILE
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 立绘差分目标无效: spec_id={}",
                spec.id
            )));
        }
        if let Some(param_profile_id) = spec
            .params_json
            .get("character_profile_id")
            .and_then(Value::as_i64)
            && param_profile_id != profile_id
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 立绘差分参数 character_profile_id 与 target_entity_id 不一致: spec_id={}",
                spec.id
            )));
        }
        let expression = owned_string(&spec.params_json, "expression").ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 立绘差分缺少 expression: {}", spec.id))
        })?;
        if !GALGAME_EXPRESSIONS.contains(&expression.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 立绘差分 expression 无效: {}",
                spec.id
            )));
        }
        let pose = owned_string(&spec.params_json, "pose").ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 立绘差分缺少 pose: {}", spec.id))
        })?;
        let variant = format!("expression:{expression}:pose:{pose}");
        let expected = format!(
            "galgame:{}:portrait:{}:{}",
            spec.project_id, profile_id, variant
        );
        if required_string(&spec.params_json, "canonical_key", "canonical_key", spec.id)?
            != expected
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 立绘差分 canonical_key 无效: {}",
                spec.id
            )));
        }
        Ok(Self {
            profile_id,
            expression,
            pose,
            position_samples: required_string_array(
                &spec.params_json,
                "position_samples",
                spec.id,
            )?,
            source_line_ids: required_i64_array(&spec.params_json, "source_line_ids", spec.id)?,
        })
    }
}

fn validate_visual_spec(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    target: &GalgameTargetContext,
) -> Result<(), AppError> {
    if spec.project_id != target.project_id
        || spec.root_project_id != target.root_project_id
        || spec.source_snapshot_id != Some(target.source_snapshot_id)
        || spec.source_chapter_id != Some(target.source_chapter_id)
        || spec.source_snapshot_chapter_id != Some(target.source_snapshot_chapter_id)
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 视觉生成意图上下文与目标不一致: {}",
            spec.id
        )));
    }
    match plan.target_kind {
        GalgameGenerationTargetKind::GalgameBackground => {
            let expected = format!(
                "galgame:{}:background:scene:{}",
                spec.project_id, plan.target_entity_id
            );
            validate_string_equals(spec, &spec.params_json, "canonical_key", &expected)?;
            let scene = required_object(&spec.params_json, "scene", spec.id)?;
            validate_i64_equals(spec, scene, "id", plan.target_entity_id)?;
            let source = required_object(&spec.params_json, "source", spec.id)?;
            validate_i64_equals(
                spec,
                source,
                "source_snapshot_chapter_id",
                target.source_snapshot_chapter_id,
            )?;
            validate_i64_equals(spec, source, "source_chapter_id", target.source_chapter_id)?;
        }
        GalgameGenerationTargetKind::GalgameCg => {
            let line_id = target.line_id.ok_or_else(|| {
                AppError::InvalidArgument(format!("CG 目标缺少 line_id: {}", spec.id))
            })?;
            let expected = format!("galgame:{}:cg:line:{line_id}", spec.project_id);
            validate_string_equals(spec, &spec.params_json, "canonical_key", &expected)?;
            let line = required_object(&spec.params_json, "line", spec.id)?;
            validate_i64_equals(spec, line, "id", line_id)?;
            let scene = required_object(&spec.params_json, "scene", spec.id)?;
            validate_i64_equals(spec, scene, "id", target.scene_id)?;
            required_string(&spec.params_json, "cg_note", "cg_note", spec.id)?;
            required_cg_importance(&spec.params_json, spec.id)?;
        }
        GalgameGenerationTargetKind::PortraitExpression => {
            return Err(AppError::InvalidArgument(format!(
                "不支持的 Galgame 视觉 target_kind: spec_id={}, target_kind={}",
                spec.id, plan.target_kind
            )));
        }
    }
    Ok(())
}

fn validate_string_equals(
    spec: &generation_spec::Model,
    value: &Value,
    field: &str,
    expected: &str,
) -> Result<(), AppError> {
    let actual = required_string(value, field, field, spec.id)?;
    if actual == expected {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 视觉生成意图 {field} 与目标不一致: spec_id={}, actual={}, expected={expected}",
            spec.id, actual
        )))
    }
}

fn validate_i64_equals(
    spec: &generation_spec::Model,
    value: &Value,
    field: &str,
    expected: i64,
) -> Result<(), AppError> {
    let actual = required_i64(value, field, spec.id)?;
    if actual == expected {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 视觉生成意图参数 {field} 与目标不一致: spec_id={}, actual={}, expected={expected}",
            spec.id, actual
        )))
    }
}

fn validate_request_snapshot(
    spec: &generation_spec::Model,
    target_id: i64,
    media_asset_id: i64,
    snapshot: Option<&Value>,
) -> Result<(), AppError> {
    let request = snapshot
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: "galgame_asset_generation".to_string(),
            operation: format!(
                "完成媒体缺少 request_snapshot_json: spec_id={}, media_asset_id={}",
                spec.id, media_asset_id
            ),
        })?;
    if request.get("source").and_then(Value::as_str) != Some("galgame_asset_generation")
        || request.get("target_kind").and_then(Value::as_str) != Some(spec.target_kind.as_str())
        || request.get("target_entity_kind").and_then(Value::as_str)
            != Some(spec.target_entity_kind.as_str())
        || request.get("target_entity_id").and_then(Value::as_i64) != Some(target_id)
    {
        return Err(AppError::InvalidState {
            current: "galgame_asset_generation".to_string(),
            operation: format!("完成媒体来源与 spec 不一致: {}", spec.id),
        });
    }
    Ok(())
}

fn required_object<'a>(value: &'a Value, field: &str, id: i64) -> Result<&'a Value, AppError> {
    value
        .get(field)
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 参数 {} 必须是对象: spec_id={id}", field))
        })
}
fn required_string<'a>(
    value: &'a Value,
    field: &str,
    path: &str,
    id: i64,
) -> Result<&'a str, AppError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 参数 {} 无效: spec_id={id}", path))
        })
}
fn owned_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}
fn required_i64(value: &Value, field: &str, id: i64) -> Result<i64, AppError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .filter(|v| *v > 0)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 参数 {} 无效: spec_id={id}", field))
        })
}
fn required_i64_array(value: &Value, field: &str, id: i64) -> Result<Vec<i64>, AppError> {
    let a = value
        .get(field)
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Galgame 参数 {} 必须是非空数组: spec_id={id}",
                field
            ))
        })?;
    let mut seen = HashSet::new();
    a.iter()
        .map(|v| {
            let n = v.as_i64().filter(|n| *n > 0).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "Galgame 参数 {} 必须是正整数数组: spec_id={id}",
                    field
                ))
            })?;
            if !seen.insert(n) {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame 参数 {} 不能重复: spec_id={id}",
                    field
                )));
            }
            Ok(n)
        })
        .collect()
}
fn required_string_array(value: &Value, field: &str, id: i64) -> Result<Vec<String>, AppError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame 参数 {} 必须是数组: spec_id={id}", field))
        })?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Galgame 参数 {} 必须是字符串数组: spec_id={id}",
                        field
                    ))
                })
        })
        .collect()
}
fn required_cg_importance(value: &Value, id: i64) -> Result<GalgameCgImportance, AppError> {
    let importance = required_string(value, "cg_importance", "cg_importance", id)?;
    GalgameCgImportance::parse(importance)
        .ok_or_else(|| AppError::InvalidArgument(format!("cg_importance 无效: spec_id={id}")))
}

fn validate_input(input: &RunGalgameAssetGenerationInput) -> Result<(), AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input.width <= 0 || input.height <= 0 {
        return Err(AppError::InvalidArgument(
            "图片 width 和 height 必须大于 0".to_string(),
        ));
    }
    if input.limit.is_some_and(|limit| limit == 0) {
        return Err(AppError::InvalidArgument("limit 必须大于 0".to_string()));
    }
    let mut seen = HashSet::new();
    for spec_id in &input.generation_spec_ids {
        if *spec_id <= 0 {
            return Err(AppError::InvalidArgument(
                "generation_spec_ids 必须全部是正整数".to_string(),
            ));
        }
        if !seen.insert(*spec_id) {
            return Err(AppError::InvalidArgument(format!(
                "generation_spec_ids 不能包含重复 ID: {spec_id}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec(
        target_kind: &str,
        target_entity_kind: &str,
        target_entity_id: Option<i64>,
        params: Value,
    ) -> generation_spec::Model {
        let now = chrono::Utc::now().naive_utc();
        generation_spec::Model {
            id: 1,
            project_id: 10,
            root_project_id: 10,
            source_context_kind: Some("chapter".to_string()),
            source_snapshot_id: Some(20),
            source_chapter_id: Some(30),
            source_snapshot_chapter_id: Some(40),
            canvas_text_asset_id: None,
            target_kind: target_kind.to_string(),
            target_entity_kind: target_entity_kind.to_string(),
            target_entity_id,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            source_version_hash: "hash".to_string(),
            params_json: params,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn visual_plan_rejects_unknown_target_pair() {
        let result = GalgameVisualSpecPlan::from_spec(&spec(
            "unknown",
            GALGAME_ENTITY_SCENE,
            Some(2),
            json!({}),
        ));
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn visual_prompt_variables_require_scene_title() {
        let value = spec(
            GALGAME_TARGET_BACKGROUND,
            GALGAME_ENTITY_SCENE,
            Some(2),
            json!({"scene": {}}),
        );
        let plan = GalgameVisualSpecPlan::from_spec(&value).expect("background plan");
        assert!(visual_prompt_variables(&value, &plan).is_err());
    }

    #[test]
    fn portrait_plan_requires_canonical_variant_key() {
        let value = spec(
            GALGAME_TARGET_PORTRAIT_EXPRESSION,
            GALGAME_ENTITY_CHARACTER_PROFILE,
            Some(7),
            json!({
                "expression": "smile",
                "pose": "standing",
                "canonical_key": "wrong",
                "source_line_ids": [1],
                "position_samples": ["left"]
            }),
        );
        assert!(GalgamePortraitSpecPlan::from_spec(&value).is_err());
    }

    #[test]
    fn cg_importance_is_required_and_rejects_unknown_values() {
        assert!(required_cg_importance(&json!({"cg_importance": "critical"}), 1).is_ok());
        assert!(required_cg_importance(&json!({}), 1).is_err());
        assert!(required_cg_importance(&json!({"cg_importance": "invalid"}), 1).is_err());
    }
}
