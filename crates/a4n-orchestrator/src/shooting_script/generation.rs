mod design;
mod screenplay;

use a4n_db::project::ResolvedChapterSource;
use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, workflow::WORKFLOW_STATUS_INTERRUPTED};

use self::{
    design::{generate_shooting_scene, restore_shooting_scene},
    screenplay::{
        assemble_screenplay_layout_unit, assemble_screenplay_plan_unit,
        generate_screenplay_adaptation_unit, generate_screenplay_layout_unit,
        generate_screenplay_skeleton, restore_screenplay_adaptation_unit,
        restore_screenplay_layout_unit, restore_screenplay_skeleton,
    },
};
use super::{
    ChapterShootingScriptSourceInput, GenerateChapterShootingScriptInput,
    assembly::{assemble_available_shooting_script, assemble_chapter_shooting_script},
    contracts::{
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, CHAPTER_SHOOTING_SCRIPT_WORKFLOW,
        ChapterShootingScriptUnitProgress, RegenerateChapterShootingScriptInput,
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
    Complete(Box<ResolvedChapterShootingScript>),
}

fn progress_source(source: ChapterShootingScriptSourceInput) -> ChapterShootingScriptSourceInput {
    ChapterShootingScriptSourceInput {
        visual_plan_id: None,
        ..source
    }
}

fn shared_workflow_source(source: &ResolvedChapterSource) -> ChapterShootingScriptSourceInput {
    ChapterShootingScriptSourceInput {
        project_id: source.root_project_id,
        source_chapter_id: source.source_chapter_id,
        visual_plan_id: None,
    }
}

impl OrchestratorContext {
    pub async fn current_chapter_shooting_script_unit_progress_for_user_validated(
        &self,
        user_id: i64,
        source: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgress>, AppError> {
        self.verify_project_or_derived_family_access_validated(user_id, source.project_id)
            .await?;
        self.current_chapter_shooting_script_unit_progress(source)
            .await
    }

    pub(crate) async fn current_chapter_shooting_script_unit_progress(
        &self,
        source: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgress>, AppError> {
        let source = progress_source(source);
        let (requested_source, _) = self
            .load_chapter_shooting_script_source_record(&source)
            .await?;
        let workflow_source_input = shared_workflow_source(&requested_source);
        let run = self
            .workflow_runs_by_project_kind(
                workflow_source_input.project_id,
                CHAPTER_SHOOTING_SCRIPT_WORKFLOW,
            )
            .await?
            .into_iter()
            .find(|run| {
                run.chapter_id == Some(workflow_source_input.source_chapter_id)
                    && run.status != crate::workflow::WORKFLOW_STATUS_SUPERSEDED
            });
        let Some(run) = run else {
            return Ok(None);
        };
        let (workflow_source, _) = self
            .load_chapter_shooting_script_source_record(&workflow_source_input)
            .await?;
        let visual_plan = self
            .confirmed_visual_plan_for_shooting_script(&workflow_source, None)
            .await?;
        let prepared = self
            .prepare_chapter_shooting_script_request(RegenerateChapterShootingScriptInput {
                project_id: workflow_source.requested_project_id,
                source_chapter_id: workflow_source.source_chapter_id,
                workflow_run_id: run.id,
                visual_plan_id: visual_plan.record.id,
            })
            .await?;
        if prepared.source.content_sha256 != requested_source.content_sha256 {
            return Ok(None);
        }
        let skeleton = restore_screenplay_skeleton(&prepared)?;
        let total_unit_count = skeleton.units.len();
        let mut plan_units = Vec::with_capacity(total_unit_count);
        let mut scenes = Vec::with_capacity(total_unit_count);
        for offset in 0..total_unit_count {
            let skeleton_unit = &skeleton.units[offset];
            let Some(detail) =
                restore_screenplay_layout_unit(self, run.id, &prepared, &skeleton, skeleton_unit)
                    .await?
            else {
                break;
            };
            let layout_unit = assemble_screenplay_layout_unit(skeleton_unit, detail);
            let Some(adaptation) =
                restore_screenplay_adaptation_unit(self, run.id, &prepared, &layout_unit).await?
            else {
                break;
            };
            let plan_unit = assemble_screenplay_plan_unit(&layout_unit, adaptation);
            let Some(scene) = restore_shooting_scene(self, run.id, &prepared, &plan_unit).await?
            else {
                break;
            };
            plan_units.push(plan_unit);
            scenes.push(scene);
        }
        if scenes.is_empty() {
            return Ok(None);
        }
        let script = assemble_available_shooting_script(
            &skeleton.directing_statement,
            &plan_units,
            scenes,
            &prepared.visual_plan,
            &prepared.visual_plan_projection,
        )?;
        let completed_unit_count = script.scenes.len();
        let resolved = if completed_unit_count == total_unit_count
            && run.status == crate::workflow::WORKFLOW_STATUS_COMPLETED
        {
            let (_, current) = self
                .load_current_chapter_shooting_script_source(&workflow_source_input)
                .await?;
            Some(ResolvedChapterShootingScript {
                source: workflow_source,
                shooting_script: current.ok_or_else(|| AppError::InvalidState {
                    current: "completed_workflow_without_shooting_script".to_string(),
                    operation: "读取已完成拍摄剧本".to_string(),
                })?,
            })
        } else {
            None
        };
        Ok(Some(ChapterShootingScriptUnitProgress {
            workflow_run_id: run.id,
            script,
            completed_unit_count,
            total_unit_count,
            resolved,
        }))
    }

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
        let input = self
            .begin_chapter_shooting_script_generation(&source, active_task_id)
            .await?;
        let workflow_run_id = input.workflow_run_id;
        let result = self
            .regenerate_chapter_shooting_script_units_with_progress(
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
            Ok(ShootingScriptGenerationOutcome::Complete(resolved)) => {
                let total_unit_count = resolved.shooting_script.script.scenes.len();
                let progress = ChapterShootingScriptUnitProgress {
                    workflow_run_id,
                    script: resolved.shooting_script.script.clone(),
                    completed_unit_count: total_unit_count,
                    total_unit_count,
                    resolved: Some(*resolved),
                };
                self.finalize_workflow_result(workflow_run_id, Ok(progress), "章节拍摄剧本单元生成")
                    .await
            }
            Err(error) => {
                self.finalize_workflow_result::<ChapterShootingScriptUnitProgress>(
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
                input,
                false,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await
            .and_then(|outcome| match outcome {
                ShootingScriptGenerationOutcome::Complete(resolved) => Ok(*resolved),
                ShootingScriptGenerationOutcome::Partial { .. } => Err(AppError::InvalidState {
                    current: "partial_shooting_script".to_string(),
                    operation: "发布完整章节拍摄剧本".to_string(),
                }),
            });
        self.finalize_workflow_result(workflow_run_id, result, "章节拍摄剧本生成")
            .await
    }

    async fn begin_chapter_shooting_script_generation(
        &self,
        source: &GenerateChapterShootingScriptInput,
        active_task_id: Option<&str>,
    ) -> Result<RegenerateChapterShootingScriptInput, AppError> {
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
            active_task_id,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow = self
            .begin_or_resume_workflow_run_validated(workflow_input)
            .await?;
        Ok(RegenerateChapterShootingScriptInput {
            project_id: source.project_id,
            source_chapter_id: source.source_chapter_id,
            workflow_run_id: workflow.id,
            visual_plan_id: visual_plan.record.id,
        })
    }

    async fn regenerate_chapter_shooting_script_units_with_progress(
        &self,
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
        let skeleton = generate_screenplay_skeleton(&prepared, progress)?;
        let total_unit_count = skeleton.units.len();
        let mut plan_units = Vec::with_capacity(total_unit_count);
        let mut scenes = Vec::with_capacity(total_unit_count);

        for offset in 0..total_unit_count {
            let skeleton_unit = &skeleton.units[offset];
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
            let layout_unit = assemble_screenplay_layout_unit(skeleton_unit, layout_stage.output);
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
            let generated_unit =
                !layout_stage.restored || !adaptation_stage.restored || !design_stage.restored;
            plan_units.push(plan_unit);
            scenes.push(design_stage.output);

            if stop_after_next_unit && generated_unit && scenes.len() < total_unit_count {
                let script = assemble_available_shooting_script(
                    &skeleton.directing_statement,
                    &plan_units,
                    scenes,
                    &prepared.visual_plan,
                    &prepared.visual_plan_projection,
                )?;
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
                    completed_unit_count: script.scenes.len(),
                    total_unit_count,
                    script: Box::new(script),
                });
            }
        }

        let script = assemble_chapter_shooting_script(
            &skeleton.directing_statement,
            &plan_units,
            scenes,
            &prepared.visual_plan,
            &prepared.visual_plan_projection,
        )?;
        progress(
            94,
            progress_total,
            "确定性装配与合同校验完成，正在发布新版本",
        )?;
        let resolved = self
            .publish_chapter_shooting_script(
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
        Ok(ShootingScriptGenerationOutcome::Complete(Box::new(
            resolved,
        )))
    }
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::shooting_script::ChapterShootingScriptSourceInput;
    use a4n_db::project::ResolvedChapterSource;

    use super::{progress_source, shared_workflow_source};

    #[test]
    fn progress_ignores_requested_visual_plan_id() {
        let source = progress_source(ChapterShootingScriptSourceInput {
            project_id: 26,
            source_chapter_id: 11,
            visual_plan_id: Some(26),
        });

        assert_eq!(source.project_id, 26);
        assert_eq!(source.source_chapter_id, 11);
        assert_eq!(source.visual_plan_id, None);
    }

    #[test]
    fn derived_project_reads_partial_workflow_from_shared_root_project() {
        let source = ResolvedChapterSource {
            requested_project_id: 25,
            root_project_id: 24,
            source_chapter_id: 6,
            source_snapshot_id: Some(18),
            source_snapshot_chapter_id: Some(20),
            chapter_number: 1,
            title: None,
            summary: None,
            analysis_data_json: None,
            content: "章节正文".to_string(),
            content_sha256: "hash".to_string(),
            source_revision_key: "snapshot:18:chapter:20:hash".to_string(),
        };

        let workflow_source = shared_workflow_source(&source);

        assert_eq!(workflow_source.project_id, 24);
        assert_eq!(workflow_source.source_chapter_id, 6);
    }
}
