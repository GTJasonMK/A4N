mod design;
mod screenplay;

use a4n_api_contract::shooting_script::{
    ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
};
use a4n_domain::shooting_script::{
    ChapterShootingScriptSchemaVersion, ChapterShootingScriptV1, ShootingVisualPlanSourceV1,
};
use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, workflow::WORKFLOW_STATUS_INTERRUPTED};

use self::{
    design::generate_shooting_scene,
    screenplay::{
        assemble_screenplay_plan_unit, generate_screenplay_adaptation_unit,
        generate_screenplay_layout_unit, generate_screenplay_skeleton,
    },
};
use super::{
    ChapterShootingScriptSourceInput, GenerateChapterShootingScriptInput,
    assembly::finish_generated_script,
    contracts::{
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptUnitProgress,
        PreparedChapterShootingScriptRequest, RegenerateChapterShootingScriptInput,
        ResolvedChapterShootingScript, ShootingScriptProgressReporter,
        ShootingScriptStreamReporter,
    },
};

enum ShootingScriptGenerationOutcome {
    Partial {
        script: Box<a4n_domain::shooting_script::ChapterShootingScriptV1>,
        completed_unit_count: usize,
        total_unit_count: usize,
    },
    Complete {
        resolved: Box<ResolvedChapterShootingScript>,
        total_unit_count: usize,
    },
}

impl OrchestratorContext {
    pub async fn generate_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        source: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, source.project_id)
            .await?;
        self.generate_chapter_shooting_script_with_progress(
            user_id,
            source,
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
    }

    pub async fn generate_next_chapter_shooting_script_unit_for_user_validated(
        &self,
        user_id: i64,
        source: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptUnitProgress, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, source.project_id)
            .await?;
        super::validate_shooting_generation_mode(
            a4n_api_contract::shooting_script::ShootingScriptGenerationMode::NextScene,
            &source.shot_ids,
        )?;
        let input = self
            .begin_chapter_shooting_script_generation(&source, active_task_id)
            .await?;
        let workflow_run_id = input.workflow_run_id;
        let result = self
            .regenerate_chapter_shooting_script_units_with_progress(
                user_id,
                input,
                true,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await;
        match result {
            Ok(ShootingScriptGenerationOutcome::Partial {
                script,
                completed_unit_count,
                total_unit_count,
            }) => {
                let reason = format!(
                    "已在场景边界暂停：已完成 {completed_unit_count}/{total_unit_count} 个场景"
                );
                self.mark_workflow_status(
                    workflow_run_id,
                    WORKFLOW_STATUS_INTERRUPTED,
                    true,
                    Some(&reason),
                )
                .await?;
                Ok(ChapterShootingScriptUnitProgress {
                    workflow_run_id,
                    script: *script,
                    completed_unit_count,
                    total_unit_count,
                    resolved: None,
                })
            }
            Ok(ShootingScriptGenerationOutcome::Complete {
                resolved,
                total_unit_count,
            }) => {
                let progress = ChapterShootingScriptUnitProgress {
                    workflow_run_id,
                    script: resolved.shooting_script.script.clone(),
                    completed_unit_count: total_unit_count,
                    total_unit_count,
                    resolved: Some(*resolved),
                };
                self.finalize_shooting_script_result(
                    workflow_run_id,
                    Ok(progress),
                    "章节拍摄剧本单元生成",
                )
                .await
            }
            Err(error) => {
                self.finalize_shooting_script_result::<ChapterShootingScriptUnitProgress>(
                    workflow_run_id,
                    Err(error),
                    "章节拍摄剧本单元生成",
                )
                .await
            }
        }
    }

    async fn generate_chapter_shooting_script_with_progress(
        &self,
        user_id: i64,
        source: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let input = self
            .begin_chapter_shooting_script_generation(&source, active_task_id)
            .await?;
        let workflow_run_id = input.workflow_run_id;
        let result = self
            .regenerate_chapter_shooting_script_units_with_progress(
                user_id,
                input,
                false,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await
            .and_then(|outcome| match outcome {
                ShootingScriptGenerationOutcome::Complete { resolved, .. } => Ok(*resolved),
                ShootingScriptGenerationOutcome::Partial { .. } => Err(AppError::InvalidState {
                    current: "partial_shooting_script".to_string(),
                    operation: "发布完整章节拍摄剧本".to_string(),
                }),
            });
        self.finalize_shooting_script_result(workflow_run_id, result, "章节拍摄剧本生成")
            .await
    }

    async fn finalize_shooting_script_result<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
        operation: &str,
    ) -> Result<T, AppError> {
        if matches!(&result, Err(AppError::Cancelled)) {
            let run = self.workflow_run_model_by_id(workflow_run_id).await?;
            if run.status == a4n_domain::workflow::WORKFLOW_STATUS_RUNNING
                && run
                    .progress_json
                    .as_ref()
                    .and_then(|value| value.get("snapshot"))
                    .is_some()
            {
                self.mark_workflow_status(
                    workflow_run_id,
                    WORKFLOW_STATUS_INTERRUPTED,
                    true,
                    Some("生成已停止，已完成场景已保存，可继续改编"),
                )
                .await?;
                return result;
            }
        }
        self.finalize_workflow_result(workflow_run_id, result, operation)
            .await
    }

    async fn begin_chapter_shooting_script_generation(
        &self,
        source: &GenerateChapterShootingScriptInput,
        active_task_id: Option<&str>,
    ) -> Result<RegenerateChapterShootingScriptInput, AppError> {
        let (workflow_input, shot_ids) = self
            .prepare_shooting_script_workflow_input(source, active_task_id)
            .await?;
        let workflow = self
            .begin_or_resume_workflow_run_validated(workflow_input)
            .await?;
        Ok(RegenerateChapterShootingScriptInput {
            project_id: source.project_id,
            source_chapter_id: source.source_chapter_id,
            workflow_run_id: workflow.id,
            visual_plan_id: source.visual_plan_id,
            shot_ids,
        })
    }

    pub(crate) async fn prepare_shooting_script_workflow_input(
        &self,
        source: &GenerateChapterShootingScriptInput,
        active_task_id: Option<&str>,
    ) -> Result<(crate::workflow::UpsertWorkflowRunInput, Vec<String>), AppError> {
        let source_input = ChapterShootingScriptSourceInput {
            project_id: source.project_id,
            source_chapter_id: source.source_chapter_id,
            visual_plan_id: Some(source.visual_plan_id),
        };
        let (resolved_source, existing) = self
            .load_chapter_shooting_script_source_record(&source_input)
            .await?;
        let visual_plan = self
            .confirmed_visual_plan_for_shooting_script(
                &resolved_source,
                Some(source.visual_plan_id),
            )
            .await?;
        let director = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &visual_plan.plan,
            &[],
        )?;
        director
            .validate_shot_ids(&source.shot_ids)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        if !source.shot_ids.is_empty()
            && existing
                .as_ref()
                .is_none_or(|script| script.record.visual_plan_id != Some(visual_plan.record.id))
        {
            return Err(AppError::InvalidArgument(
                "局部更新需要已绑定当前导演编排的拍摄剧本".into(),
            ));
        }
        let mut shot_ids = source.shot_ids.clone();
        shot_ids.sort();
        let target_revision =
            match existing {
                Some(existing) => existing.record.revision.checked_add(1).ok_or_else(|| {
                    AppError::InvalidState {
                        current: format!(
                            "chapter_shooting_script_revision_exhausted:id={},revision={}",
                            existing.record.id, existing.record.revision
                        ),
                        operation: "创建章节拍摄剧本工作流".to_string(),
                    }
                })?,
                None => 1,
            };
        let workflow_input = super::chapter_shooting_script_workflow_run_input(
            source.project_id,
            source.source_chapter_id,
            target_revision,
            visual_plan.record.id,
            visual_plan.record.plan_version,
            &visual_plan.record.plan_sha256,
            &shot_ids,
            active_task_id,
        )
        .map_err(AppError::InvalidArgument)?;
        Ok((workflow_input, shot_ids))
    }

    async fn regenerate_chapter_shooting_script_units_with_progress(
        &self,
        user_id: i64,
        input: RegenerateChapterShootingScriptInput,
        stop_after_next_unit: bool,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ShootingScriptGenerationOutcome, AppError> {
        let workflow_run_id = input.workflow_run_id;
        let progress_total = CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL;
        progress(0, progress_total, "正在读取章节正文和来源锚点")?;
        let prepared = self.prepare_chapter_shooting_script_request(input).await?;
        let saved_progress = self
            .saved_shooting_script_progress(workflow_run_id, &prepared)
            .await?;
        let skeleton = generate_screenplay_skeleton(&prepared, progress)?;
        let total_unit_count = skeleton.units.len();
        let completed_unit_count = saved_progress
            .as_ref()
            .map_or(0, |progress| progress.completed_unit_count);
        // 已保存的进度是作者可编辑的事实来源；LLM 断点只恢复尚未完成的单元。
        let mut script = saved_progress
            .map(|progress| progress.script)
            .unwrap_or_else(|| ChapterShootingScriptV1 {
                schema_version: ChapterShootingScriptSchemaVersion::V1,
                visual_plan_source: Some(ShootingVisualPlanSourceV1 {
                    plan_id: prepared.visual_plan.record.id,
                    plan_version: prepared.visual_plan.record.plan_version,
                    plan_sha256: prepared.visual_plan.record.plan_sha256.clone(),
                }),
                user_additions: prepared.visual_plan_projection.user_additions.clone(),
                directing_statement: Some(skeleton.directing_statement.clone()),
                scenes: Vec::with_capacity(total_unit_count),
            });

        for offset in completed_unit_count..total_unit_count {
            let skeleton_unit = &skeleton.units[offset];
            if !prepared.shot_ids.is_empty()
                && !prepared.visual_plan_projection.narrative_units[offset]
                    .shots
                    .iter()
                    .any(|shot| prepared.shot_ids.contains(&shot.segment_id))
            {
                continue;
            }
            let layout_stage = generate_screenplay_layout_unit(
                self,
                workflow_run_id,
                &prepared,
                &skeleton,
                skeleton_unit,
                offset,
                total_unit_count,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            let layout_unit = layout_stage.output;
            let adaptation_stage = generate_screenplay_adaptation_unit(
                self,
                workflow_run_id,
                &prepared,
                &layout_unit,
                offset,
                total_unit_count,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            let plan_unit = assemble_screenplay_plan_unit(&layout_unit, adaptation_stage.output);
            let design_stage = generate_shooting_scene(
                self,
                workflow_run_id,
                &prepared,
                &plan_unit,
                offset,
                total_unit_count,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            let mut scene = design_stage.output;
            scene.scene_index = u32::try_from(script.scenes.len() + 1)
                .map_err(|_| AppError::InvalidArgument("拍摄场景数量超出 u32 范围".into()))?;
            scene.timeline_position = scene.scene_index;
            for addition in &prepared.visual_plan_projection.user_additions {
                if scene.user_addition_ids.contains(&addition.id)
                    && !script
                        .user_additions
                        .iter()
                        .any(|saved| saved.id == addition.id)
                {
                    script.user_additions.push(addition.clone());
                }
            }
            script.scenes.push(scene);

            if prepared.shot_ids.is_empty() {
                // Publish completed scenes before starting another provider call. A failed connection
                // or a stopped task must not hide work that has already passed scene validation.
                finish_generated_script(&mut script, offset + 1, total_unit_count)?;
                self.persist_workflow_progress_snapshot(
                    workflow_run_id,
                    serde_json::to_value(ChapterShootingScriptUnitProgressCheckpoint {
                        version: 1,
                        source_content_sha256: prepared.source.content_sha256.clone(),
                        progress: ChapterShootingScriptUnitProgressView {
                            workflow_run_id,
                            script: script.clone(),
                            completed_unit_count: offset + 1,
                            total_unit_count,
                            completed_record: None,
                        },
                    })?,
                )
                .await?;
            }
            if stop_after_next_unit && prepared.shot_ids.is_empty() && offset + 1 < total_unit_count
            {
                progress(
                    92,
                    progress_total,
                    &format!(
                        "场景 {}/{} 已完成，可先进行片段规划",
                        script.scenes.len(),
                        total_unit_count
                    ),
                )?;
                return Ok(ShootingScriptGenerationOutcome::Partial {
                    completed_unit_count: offset + 1,
                    total_unit_count,
                    script: Box::new(script),
                });
            }
        }

        let script = if !prepared.shot_ids.is_empty() {
            prepared
                .existing
                .as_ref()
                .ok_or_else(|| AppError::InvalidArgument("局部更新的拍摄剧本已不存在".into()))?
                .script
                .with_updated_shots(
                    &prepared.visual_plan_projection,
                    script.scenes,
                    &prepared.shot_ids,
                )
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?
        } else {
            finish_generated_script(&mut script, total_unit_count, total_unit_count)?;
            script
        };
        progress(
            94,
            progress_total,
            "确定性装配与合同校验完成，正在发布新版本",
        )?;
        let resolved = self
            .publish_chapter_shooting_script(
                user_id,
                prepared.source,
                prepared.existing,
                prepared.input_fingerprint_sha256,
                a4n_db::shooting_script::ChapterVisualPlanIdentity {
                    id: prepared.visual_plan.record.id,
                    version: prepared.visual_plan.record.plan_version,
                    sha256: prepared.visual_plan.record.plan_sha256,
                },
                script,
                workflow_run_id,
            )
            .await?;
        progress(progress_total, progress_total, "结构化拍摄剧本已原子发布")?;
        Ok(ShootingScriptGenerationOutcome::Complete {
            resolved: Box::new(resolved),
            total_unit_count,
        })
    }

    async fn saved_shooting_script_progress(
        &self,
        workflow_run_id: i64,
        prepared: &PreparedChapterShootingScriptRequest,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        let Some(snapshot) = run
            .progress_json
            .and_then(|progress| progress.get("snapshot").cloned())
        else {
            return Ok(None);
        };
        let checkpoint: ChapterShootingScriptUnitProgressCheckpoint =
            serde_json::from_value(snapshot).map_err(|error| {
                AppError::InvalidArgument(format!("章节拍摄剧本进度 snapshot 无效: {error}"))
            })?;
        checkpoint
            .validate_for_run(workflow_run_id)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        if checkpoint.source_content_sha256 != prepared.source.content_sha256
            || checkpoint.progress.total_unit_count
                != prepared.visual_plan_projection.narrative_units.len()
            || checkpoint.progress.script.visual_plan_source.as_ref()
                != Some(&ShootingVisualPlanSourceV1 {
                    plan_id: prepared.visual_plan.record.id,
                    plan_version: prepared.visual_plan.record.plan_version,
                    plan_sha256: prepared.visual_plan.record.plan_sha256.clone(),
                })
            || !prepared.shot_ids.is_empty()
        {
            return Err(AppError::InvalidArgument(
                "章节拍摄剧本进度 snapshot 与当前生成来源不一致".into(),
            ));
        }
        Ok(Some(checkpoint.progress))
    }
}
