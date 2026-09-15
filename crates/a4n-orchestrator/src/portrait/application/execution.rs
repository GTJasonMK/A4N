use std::collections::HashMap;

use a4n_api_contract::portrait::{
    GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
    GeneratePortraitPromptInput, PortraitSummary, PortraitType, RegeneratePortraitInput,
    UploadPortraitInput,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    portrait::{
        PortraitPromptGenerationInput, PortraitPromptSpec, missing_portrait_item_fingerprint,
        portrait_generate_workflow_fingerprint, portrait_image_workflow_fingerprint,
        portrait_prompt_workflow_fingerprint, profile_portrait_seed_prompt,
    },
};

use super::{
    contracts::{
        PortraitBatchItem, PortraitBatchOutcome, PortraitGenerationHostPort, PortraitProgress,
    },
    image::{
        PersistGeneratedPortraitInput, generate_portrait_image, persist_generated_portrait,
        portrait_summary, portrait_type_from_record, record_initial_prompt_generation,
        required_portrait, required_portrait_profile, upload_portrait,
    },
    request::{prepare_named_portrait_request, prepare_profile_portrait_request},
    workflow::{
        load_completed_portrait_checkpoint, report_progress, require_not_cancelled,
        resolve_initial_portrait_prompt, save_completed_portrait_checkpoint,
    },
};

struct PortraitVariantRun<'a> {
    workflow_kind: &'a str,
    variant_kind: &'a str,
    step_key: &'a str,
    path_context: &'a str,
    provider_progress_message: &'a str,
    restored_progress_message: &'a str,
    save_progress_message: &'a str,
    record_progress_message: &'a str,
    completed_message: &'a str,
    requested_by_user_id: i64,
    existing: &'a a4n_db_models::models::character::character_portrait::Model,
    character_name: &'a str,
    prompt: &'a str,
    width: i32,
    height: i32,
    reference_image_ids: &'a [i64],
    fingerprint: Value,
}

impl OrchestratorContext {
    pub async fn prepare_generate_portrait_request_preview_validated(
        &self,
        input: &GeneratePortraitInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        Ok(prepare_named_portrait_request(self, input, prompt_renderer)
            .await?
            .request)
    }

    pub async fn prepare_generate_portrait_prompt_request_preview_validated(
        &self,
        input: &GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        Ok(
            prepare_profile_portrait_request(self, input, prompt_renderer, variables)
                .await?
                .request,
        )
    }

    pub async fn generate_portrait_prompt_validated(
        &self,
        input: GeneratePortraitPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<String, AppError> {
        let prepared =
            prepare_profile_portrait_request(self, &input, prompt_renderer, variables).await?;
        let workflow_fingerprint = portrait_prompt_workflow_fingerprint(
            input.project_id,
            &prepared.profile,
            input.portrait_type.as_str(),
            input.variables.as_ref(),
            &prepared.profile_context,
            prepared.project_context.as_ref(),
            &prepared.request,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self
            .begin_portrait_prompt_workflow_validated(
                input.project_id,
                input.profile_id,
                input.portrait_type,
                &workflow_fingerprint,
            )
            .await?;
        let profile = prepared.profile;
        let result = self
            .generate_prompt_with_checkpoint(PortraitPromptGenerationInput {
                project_id: input.project_id,
                spec: PortraitPromptSpec {
                    profile_name: &profile.name,
                    appearance: profile.appearance.as_deref(),
                    gender: profile.gender.as_deref(),
                    age: profile.age.as_deref(),
                    personality: profile.personality.as_deref(),
                    background: profile.background.as_deref(),
                    profile_context: Some(&prepared.profile_context),
                    project_context: prepared.project_context.as_ref(),
                    portrait_type: input.portrait_type,
                    art_style: None,
                    expression: None,
                    existing_portrait_count: None,
                },
                prompt_renderer,
                variables,
                workflow_run_id: Some(workflow_run.id),
            })
            .await;
        self.finalize_workflow_result(workflow_run.id, result, "人物三视图提示词生成")
            .await
            .map(|prompt| prompt.trim().to_string())
    }

    pub async fn generate_portrait_validated(
        &self,
        requested_by_user_id: i64,
        input: GeneratePortraitInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn PortraitGenerationHostPort,
        mut progress: Option<PortraitProgress<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitSummary, AppError> {
        report_progress(&mut progress, 0, 6, "正在读取角色档案")?;
        require_not_cancelled(&cancel_token)?;
        let prepared = prepare_named_portrait_request(self, &input, prompt_renderer).await?;
        let workflow_fingerprint = portrait_generate_workflow_fingerprint(
            input.project_id,
            &prepared.profile,
            input.style.as_deref(),
            &prepared.expression,
            input.pose.as_deref(),
            &input.reference_image_ids,
            input.width,
            input.height,
            &prepared.request,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self
            .begin_portrait_generation_workflow_validated(
                input.project_id,
                prepared.profile.id,
                &workflow_fingerprint,
            )
            .await?;
        let workflow_run_id = workflow_run.id;
        let result = async {
            let prompt = resolve_initial_portrait_prompt(
                self,
                workflow_run_id,
                &workflow_fingerprint,
                prepared.request,
                &mut progress,
                &cancel_token,
            )
            .await?;
            report_progress(&mut progress, 3, 6, "正在调用图片模型生成人物三视图拆解")?;
            require_not_cancelled(&cancel_token)?;
            if let Some(summary) = load_completed_portrait_checkpoint(
                self,
                host_port,
                workflow_run_id,
                "portrait_generation.final",
                &workflow_fingerprint,
                input.project_id,
            )
            .await?
            {
                report_progress(&mut progress, 6, 6, "人物三视图拆解已从断点恢复")?;
                return Ok(summary);
            }
            let generated = generate_portrait_image(
                self,
                host_port,
                input.project_id,
                &prompt.image_prompt,
                input.width,
                input.height,
                &input.reference_image_ids,
                Some(workflow_run_id),
                cancel_token.clone(),
            )
            .await?;
            require_not_cancelled(&cancel_token)?;
            report_progress(&mut progress, 4, 6, "正在保存人物三视图拆解图片")?;
            report_progress(&mut progress, 5, 6, "正在写入人物三视图拆解记录")?;
            let portrait = persist_generated_portrait(
                self,
                host_port,
                &generated,
                PersistGeneratedPortraitInput {
                    workflow_run_id: Some(workflow_run_id),
                    requested_by_user_id,
                    profile_id: prepared.profile.id,
                    project_id: input.project_id,
                    portrait_type: PortraitType::FullBody,
                    prompt: &prompt.image_prompt,
                    expression: Some(&prepared.expression),
                    pose: input.pose.as_deref(),
                    character_name: &prepared.profile.name,
                    variant_kind: None,
                    auto_generated: false,
                    path_context: "人物三视图拆解图片",
                    reference_image_ids: &input.reference_image_ids,
                },
            )
            .await?;
            record_initial_prompt_generation(
                self,
                &portrait,
                &input,
                &prepared.profile,
                &prepared.expression,
                workflow_run_id,
                workflow_fingerprint.clone(),
                prompt,
            )
            .await?;
            report_progress(&mut progress, 6, 6, "人物三视图拆解生成完成")?;
            portrait_summary(&portrait)
        }
        .await;
        self.finalize_workflow_result(workflow_run_id, result, "人物三视图拆解生成")
            .await
    }

    pub async fn generate_missing_portraits_validated(
        &self,
        requested_by_user_id: i64,
        input: GenerateMissingPortraitsInput,
        host_port: &dyn PortraitGenerationHostPort,
        mut progress: Option<PortraitProgress<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitBatchOutcome, AppError> {
        let missing_profiles = self.missing_portrait_profiles(input.project_id).await?;
        let portrait_count = u32::try_from(missing_profiles.len()).map_err(|_| {
            AppError::InvalidArgument("待生成人物三视图拆解数量超出 u32 范围".to_string())
        })?;
        if portrait_count == 0 {
            return Err(AppError::InvalidArgument(
                "当前项目没有缺失立绘，无需生成".to_string(),
            ));
        }
        i32::try_from(missing_profiles.len()).map_err(|_| {
            AppError::InvalidArgument(
                "待生成人物三视图拆解数量超出 checkpoint 序号范围".to_string(),
            )
        })?;
        let workflow_fingerprint = json!({
            "workflow": "missing_portrait_generation",
            "project_id": input.project_id,
            "width": input.width,
            "height": input.height,
        });
        let workflow_run = self
            .begin_missing_portrait_workflow_validated(input.project_id, &workflow_fingerprint)
            .await?;
        let workflow_run_id = workflow_run.id;
        let result = async {
            let total = portrait_count;
            let mut outcome = PortraitBatchOutcome {
                total,
                items: Vec::new(),
                failed_item_keys: Vec::new(),
                cancelled: 0,
                skipped: 0,
                interrupted: false,
                first_error: None,
            };
            for (index, profile) in missing_profiles.iter().enumerate() {
                let display_index = index + 1;
                let current = u32::try_from(index).map_err(|_| {
                    AppError::InvalidArgument("人物三视图拆解进度序号超出 u32 范围".to_string())
                })?;
                let next_current = u32::try_from(display_index).map_err(|_| {
                    AppError::InvalidArgument("人物三视图拆解进度序号超出 u32 范围".to_string())
                })?;
                let step_order = i32::try_from(display_index).map_err(|_| {
                    AppError::InvalidArgument(
                        "人物三视图拆解 checkpoint 序号超出 i32 范围".to_string(),
                    )
                })?;
                report_progress(
                    &mut progress,
                    current,
                    total,
                    &format!(
                        "正在生成人物三视图拆解 {display_index}/{}：{}",
                        missing_profiles.len(),
                        profile.name
                    ),
                )?;
                let item_key = format!("portrait-profile-{}", profile.id);
                let item_result = async {
                    require_not_cancelled(&cancel_token)?;
                    let prompt =
                        profile_portrait_seed_prompt(profile).map_err(AppError::InvalidArgument)?;
                    let step_key = format!("missing_portrait_generation.profile.{}", profile.id);
                    let item_fingerprint = missing_portrait_item_fingerprint(
                        input.project_id,
                        profile,
                        &prompt,
                        input.width,
                        input.height,
                    );
                    if let Some(summary) = load_completed_portrait_checkpoint(
                        self,
                        host_port,
                        workflow_run_id,
                        &step_key,
                        &item_fingerprint,
                        input.project_id,
                    )
                    .await?
                    {
                        return Ok(summary);
                    }
                    let generated = generate_portrait_image(
                        self,
                        host_port,
                        input.project_id,
                        &prompt,
                        input.width,
                        input.height,
                        &[],
                        Some(workflow_run_id),
                        cancel_token.clone(),
                    )
                    .await?;
                    require_not_cancelled(&cancel_token)?;
                    let portrait = persist_generated_portrait(
                        self,
                        host_port,
                        &generated,
                        PersistGeneratedPortraitInput {
                            workflow_run_id: Some(workflow_run_id),
                            requested_by_user_id,
                            profile_id: profile.id,
                            project_id: input.project_id,
                            portrait_type: PortraitType::HalfBody,
                            prompt: &prompt,
                            expression: None,
                            pose: None,
                            character_name: &profile.name,
                            variant_kind: None,
                            auto_generated: true,
                            path_context: "缺失人物三视图拆解",
                            reference_image_ids: &[],
                        },
                    )
                    .await?;
                    let summary = portrait_summary(&portrait)?;
                    save_completed_portrait_checkpoint(
                        self,
                        workflow_run_id,
                        step_key,
                        step_order,
                        item_fingerprint,
                        &summary,
                    )
                    .await?;
                    Ok(summary)
                }
                .await;

                match item_result {
                    Ok(summary) => {
                        outcome.items.push(PortraitBatchItem {
                            item_key,
                            item_index: next_current - 1,
                            summary,
                        });
                        report_progress(
                            &mut progress,
                            next_current,
                            total,
                            &format!(
                                "已生成人物三视图拆解 {display_index}/{}：{}",
                                missing_profiles.len(),
                                profile.name
                            ),
                        )?;
                    }
                    Err(AppError::Cancelled) => {
                        outcome.cancelled = 1;
                        outcome.skipped = total.saturating_sub(next_current);
                        outcome.interrupted = true;
                        outcome
                            .first_error
                            .get_or_insert_with(|| AppError::Cancelled.to_string());
                        break;
                    }
                    Err(error) => {
                        outcome.failed_item_keys.push(item_key);
                        outcome.first_error.get_or_insert_with(|| error.to_string());
                        if let Err(progress_error) = report_progress(
                            &mut progress,
                            next_current,
                            total,
                            &format!(
                                "人物三视图拆解 {display_index}/{} 失败：{}",
                                missing_profiles.len(),
                                error
                            ),
                        ) {
                            if matches!(progress_error, AppError::Cancelled) {
                                outcome.skipped = total.saturating_sub(next_current);
                                outcome.interrupted = true;
                                outcome
                                    .first_error
                                    .get_or_insert_with(|| progress_error.to_string());
                                break;
                            }
                            return Err(progress_error);
                        }
                    }
                }
            }
            Ok(outcome)
        }
        .await;
        match &result {
            Ok(outcome) if outcome.interrupted => {
                let error =
                    outcome
                        .first_error
                        .as_deref()
                        .ok_or_else(|| AppError::InvalidState {
                            current: "portrait_batch_interrupted_without_error".to_string(),
                            operation: "写入缺失立绘取消终态".to_string(),
                        })?;
                self.cancel_workflow_run_status_validated(workflow_run_id, error)
                    .await?;
            }
            Ok(outcome) if outcome.succeeded() == 0 => {
                let error =
                    outcome
                        .first_error
                        .as_deref()
                        .ok_or_else(|| AppError::InvalidState {
                            current: "portrait_batch_failed_without_error".to_string(),
                            operation: "写入缺失立绘失败终态".to_string(),
                        })?;
                self.fail_workflow_run_validated(workflow_run_id, error)
                    .await?;
            }
            _ => {
                return self
                    .finalize_workflow_result(workflow_run_id, result, "缺失立绘生成")
                    .await;
            }
        }
        result
    }

    pub async fn regenerate_portrait_validated(
        &self,
        requested_by_user_id: i64,
        input: RegeneratePortraitInput,
        host_port: &dyn PortraitGenerationHostPort,
        mut progress: Option<PortraitProgress<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitSummary, AppError> {
        let existing = required_portrait(self, input.portrait_id).await?;
        report_progress(&mut progress, 0, 4, "正在读取立绘和角色档案")?;
        require_not_cancelled(&cancel_token)?;
        let profile = required_portrait_profile(self, existing.profile_id, existing.id).await?;
        let fingerprint = portrait_image_workflow_fingerprint(
            "portrait_regeneration",
            existing.project_id,
            existing.id,
            &existing.prompt,
            &input.reference_image_ids,
            json!({
                "variant_kind": "regenerated",
                "width": input.width,
                "height": input.height,
            }),
        );
        self.generate_portrait_image_variant(
            PortraitVariantRun {
                workflow_kind: "portrait_regeneration",
                variant_kind: "regenerated",
                step_key: "portrait_regeneration.final",
                path_context: "立绘重绘",
                provider_progress_message: "正在调用图片模型重绘立绘",
                restored_progress_message: "立绘重绘已从断点恢复",
                save_progress_message: "正在保存重绘图片",
                record_progress_message: "正在写入重绘记录",
                completed_message: "立绘重绘完成",
                requested_by_user_id,
                existing: &existing,
                character_name: &profile.name,
                prompt: &existing.prompt,
                width: input.width,
                height: input.height,
                reference_image_ids: &input.reference_image_ids,
                fingerprint,
            },
            host_port,
            &mut progress,
            cancel_token,
        )
        .await
    }

    pub async fn generate_age_variant_validated(
        &self,
        requested_by_user_id: i64,
        input: GenerateAgeVariantInput,
        host_port: &dyn PortraitGenerationHostPort,
        mut progress: Option<PortraitProgress<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitSummary, AppError> {
        let existing = required_portrait(self, input.portrait_id).await?;
        report_progress(&mut progress, 0, 4, "正在读取立绘和角色档案")?;
        require_not_cancelled(&cancel_token)?;
        let profile = required_portrait_profile(self, existing.profile_id, existing.id).await?;
        let new_prompt = format!("{}，age: {}", existing.prompt, input.age);
        let fingerprint = portrait_image_workflow_fingerprint(
            "portrait_age_variant_generation",
            existing.project_id,
            existing.id,
            &new_prompt,
            &input.reference_image_ids,
            json!({
                "variant_kind": "age_variant",
                "age": input.age,
                "width": input.width,
                "height": input.height,
            }),
        );
        self.generate_portrait_image_variant(
            PortraitVariantRun {
                workflow_kind: "portrait_age_variant_generation",
                variant_kind: "age_variant",
                step_key: "portrait_age_variant_generation.final",
                path_context: "年龄变体立绘",
                provider_progress_message: "正在调用图片模型生成年龄变体",
                restored_progress_message: "年龄变体已从断点恢复",
                save_progress_message: "正在保存年龄变体图片",
                record_progress_message: "正在写入年龄变体记录",
                completed_message: "年龄变体生成完成",
                requested_by_user_id,
                existing: &existing,
                character_name: &profile.name,
                prompt: &new_prompt,
                width: input.width,
                height: input.height,
                reference_image_ids: &input.reference_image_ids,
                fingerprint,
            },
            host_port,
            &mut progress,
            cancel_token,
        )
        .await
    }

    pub async fn upload_portrait_validated(
        &self,
        requested_by_user_id: i64,
        input: UploadPortraitInput,
        host_port: &dyn PortraitGenerationHostPort,
    ) -> Result<PortraitSummary, AppError> {
        upload_portrait(self, host_port, requested_by_user_id, input).await
    }

    async fn generate_portrait_image_variant(
        &self,
        run: PortraitVariantRun<'_>,
        host_port: &dyn PortraitGenerationHostPort,
        progress: &mut Option<PortraitProgress<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitSummary, AppError> {
        let PortraitVariantRun {
            workflow_kind,
            variant_kind,
            step_key,
            path_context,
            provider_progress_message,
            restored_progress_message,
            save_progress_message,
            record_progress_message,
            completed_message,
            requested_by_user_id,
            existing,
            character_name,
            prompt,
            width,
            height,
            reference_image_ids,
            fingerprint,
        } = run;
        let portrait_type = portrait_type_from_record(existing)?;
        let workflow_run = self
            .begin_portrait_image_workflow_validated(
                workflow_kind,
                existing.project_id,
                existing.id,
                variant_kind,
                step_key,
                &fingerprint,
            )
            .await?;
        let result = async {
            report_progress(progress, 1, 4, provider_progress_message)?;
            if let Some(summary) = load_completed_portrait_checkpoint(
                self,
                host_port,
                workflow_run.id,
                step_key,
                &fingerprint,
                existing.project_id,
            )
            .await?
            {
                report_progress(progress, 4, 4, restored_progress_message)?;
                return Ok(summary);
            }
            let generated = generate_portrait_image(
                self,
                host_port,
                existing.project_id,
                prompt,
                width,
                height,
                reference_image_ids,
                Some(workflow_run.id),
                cancel_token.clone(),
            )
            .await?;
            require_not_cancelled(&cancel_token)?;
            report_progress(progress, 2, 4, save_progress_message)?;
            report_progress(progress, 3, 4, record_progress_message)?;
            let portrait = persist_generated_portrait(
                self,
                host_port,
                &generated,
                PersistGeneratedPortraitInput {
                    workflow_run_id: Some(workflow_run.id),
                    requested_by_user_id,
                    profile_id: existing.profile_id,
                    project_id: existing.project_id,
                    portrait_type,
                    prompt,
                    expression: existing.expression.as_deref(),
                    pose: existing.pose.as_deref(),
                    character_name,
                    variant_kind: Some(variant_kind),
                    auto_generated: false,
                    path_context,
                    reference_image_ids,
                },
            )
            .await?;
            let summary = portrait_summary(&portrait)?;
            save_completed_portrait_checkpoint(
                self,
                workflow_run.id,
                step_key.to_string(),
                1,
                fingerprint.clone(),
                &summary,
            )
            .await?;
            report_progress(progress, 4, 4, completed_message)?;
            Ok(summary)
        }
        .await;
        self.finalize_workflow_result(workflow_run.id, result, path_context)
            .await
    }
}
