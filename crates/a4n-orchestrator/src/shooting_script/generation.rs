mod design;
mod screenplay;

use a4n_core::error::AppError;
use a4n_db::project::ResolvedChapterSource;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, workflow::WORKFLOW_STATUS_INTERRUPTED};

use self::{
    design::{generate_shooting_scene, restore_shooting_scene},
    screenplay::{
        adaptation::{generate_screenplay_adaptation_unit, restore_screenplay_adaptation_unit},
        layout::{
            generate_screenplay_layout_unit, generate_screenplay_skeleton,
            restore_screenplay_layout_unit, restore_screenplay_skeleton,
        },
    },
};
use super::{
    assembly::{assemble_available_shooting_script, assemble_chapter_shooting_script},
    contracts::{
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterScreenplayLayoutV1, ChapterScreenplayPlan,
        ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgress,
        RegenerateChapterShootingScriptInput, ResolvedChapterShootingScript,
        ShootingScriptProgressReporter, ShootingScriptStreamReporter,
    },
    screenplay::{
        assemble_screenplay_layout_unit, assemble_screenplay_plan_unit, validate_screenplay_layout,
        validate_screenplay_plan, validate_script_against_plan, validate_script_source_spans,
    },
};

enum ShootingScriptGenerationOutcome {
    Partial {
        script: Box<a4n_core::shooting_script::ChapterShootingScriptV1>,
        completed_unit_count: usize,
        total_unit_count: usize,
    },
    Complete(Box<ResolvedChapterShootingScript>),
}

fn shared_workflow_source(source: &ResolvedChapterSource) -> ChapterShootingScriptSourceInput {
    ChapterShootingScriptSourceInput {
        project_id: source.root_project_id,
        source_chapter_id: source.source_chapter_id,
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
        let (requested_source, _) = self.load_chapter_shooting_script_source(&source).await?;
        let workflow_source = shared_workflow_source(&requested_source);
        let run = self
            .workflow_runs_by_project_kind(workflow_source.project_id, "chapter_shooting_script")
            .await?
            .into_iter()
            .find(|run| {
                run.chapter_id == Some(workflow_source.source_chapter_id)
                    && run.status != crate::workflow::WORKFLOW_STATUS_SUPERSEDED
            });
        let Some(run) = run else {
            return Ok(None);
        };
        let prepared = self
            .prepare_chapter_shooting_script_request(RegenerateChapterShootingScriptInput {
                project_id: workflow_source.project_id,
                source_chapter_id: workflow_source.source_chapter_id,
                workflow_run_id: run.id,
            })
            .await?;
        if prepared.source.content_sha256 != requested_source.content_sha256 {
            return Ok(None);
        }
        let Some(skeleton) = restore_screenplay_skeleton(self, run.id, &prepared).await? else {
            return Ok(None);
        };
        let total_unit_count = skeleton.units.len();
        let mut layout_units = Vec::with_capacity(total_unit_count);
        let mut plan_units = Vec::with_capacity(total_unit_count);
        let mut scenes = Vec::with_capacity(total_unit_count);
        for offset in 0..total_unit_count {
            let Some(detail) =
                restore_screenplay_layout_unit(self, run.id, &prepared, &skeleton, offset).await?
            else {
                break;
            };
            layout_units.push(assemble_screenplay_layout_unit(
                &skeleton.units[offset],
                detail,
            )?);
            let Some(adaptation) =
                restore_screenplay_adaptation_unit(self, run.id, &prepared, &layout_units[offset])
                    .await?
            else {
                break;
            };
            plan_units.push(assemble_screenplay_plan_unit(
                &layout_units[offset],
                adaptation,
            )?);
            let Some(scene) =
                restore_shooting_scene(self, run.id, &prepared, &plan_units[offset]).await?
            else {
                break;
            };
            scenes.push(scene);
        }
        if scenes.is_empty() {
            return Ok(None);
        }
        let plan = ChapterScreenplayPlan {
            directing_statement: skeleton.directing_statement,
            units: plan_units,
        };
        let script = assemble_available_shooting_script(&plan, scenes)?;
        let completed_unit_count = script.scenes.len();
        let resolved = if completed_unit_count == total_unit_count
            && run.status == crate::workflow::WORKFLOW_STATUS_COMPLETED
        {
            Some(
                self.current_chapter_shooting_script(source)
                    .await?
                    .ok_or_else(|| AppError::InvalidState {
                        current: "completed_workflow_without_shooting_script".to_string(),
                        operation: "读取已完成拍摄剧本".to_string(),
                    })?,
            )
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
        source: ChapterShootingScriptSourceInput,
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
        source: ChapterShootingScriptSourceInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptUnitProgress, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, source.project_id)
            .await?;
        let target_revision = self
            .next_chapter_shooting_script_revision(source.clone())
            .await?;
        let workflow = self
            .begin_chapter_shooting_script_workflow_validated(
                source.project_id,
                source.source_chapter_id,
                target_revision,
                active_task_id,
            )
            .await?;
        let result = self
            .regenerate_chapter_shooting_script_units_with_progress(
                RegenerateChapterShootingScriptInput {
                    project_id: source.project_id,
                    source_chapter_id: source.source_chapter_id,
                    workflow_run_id: workflow.id,
                },
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
                    workflow.id,
                    WORKFLOW_STATUS_INTERRUPTED,
                    true,
                    Some(&reason),
                )
                .await?;
                Ok(ChapterShootingScriptUnitProgress {
                    workflow_run_id: workflow.id,
                    script: *script,
                    completed_unit_count,
                    total_unit_count,
                    resolved: None,
                })
            }
            Ok(ShootingScriptGenerationOutcome::Complete(resolved)) => {
                self.complete_workflow_run_validated(workflow.id).await?;
                let total_unit_count = resolved.shooting_script.script.scenes.len();
                Ok(ChapterShootingScriptUnitProgress {
                    workflow_run_id: workflow.id,
                    script: resolved.shooting_script.script.clone(),
                    completed_unit_count: total_unit_count,
                    total_unit_count,
                    resolved: Some(*resolved),
                })
            }
            Err(error) => {
                self.finalize_workflow_outcome_result::<ChapterShootingScriptUnitProgress>(
                    workflow.id,
                    Err(error),
                    "章节拍摄剧本单元生成",
                )
                .await
            }
        }
    }

    pub async fn generate_chapter_shooting_script_with_progress(
        &self,
        source: ChapterShootingScriptSourceInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let target_revision = self
            .next_chapter_shooting_script_revision(source.clone())
            .await?;
        let workflow = self
            .begin_chapter_shooting_script_workflow_validated(
                source.project_id,
                source.source_chapter_id,
                target_revision,
                active_task_id,
            )
            .await?;
        let result = self
            .regenerate_chapter_shooting_script_units_with_progress(
                RegenerateChapterShootingScriptInput {
                    project_id: source.project_id,
                    source_chapter_id: source.source_chapter_id,
                    workflow_run_id: workflow.id,
                },
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
        self.finalize_workflow_outcome_result(workflow.id, result, "章节拍摄剧本生成")
            .await
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
        let skeleton =
            generate_screenplay_skeleton(
                self,
                workflow_run_id,
                &prepared,
                progress,
                stream_reporter,
                cancel_token,
            )
                .await?;
        let total_unit_count = skeleton.units.len();
        let mut layout_units = Vec::with_capacity(total_unit_count);
        let mut plan_units = Vec::with_capacity(total_unit_count);
        let mut scenes = Vec::with_capacity(total_unit_count);

        for offset in 0..total_unit_count {
            let layout_stage = generate_screenplay_layout_unit(
                self,
                workflow_run_id,
                &prepared,
                &skeleton,
                offset,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            layout_units.push(assemble_screenplay_layout_unit(
                &skeleton.units[offset],
                layout_stage.output,
            )?);
            let layout = ChapterScreenplayLayoutV1 {
                schema_version: "chapter_screenplay_layout_v1".to_string(),
                directing_statement: skeleton.directing_statement.clone(),
                units: layout_units.clone(),
            };
            let adaptation_stage = generate_screenplay_adaptation_unit(
                self,
                workflow_run_id,
                &prepared,
                &layout,
                offset,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            plan_units.push(assemble_screenplay_plan_unit(
                &layout.units[offset],
                adaptation_stage.output,
            )?);
            let plan = ChapterScreenplayPlan {
                directing_statement: skeleton.directing_statement.clone(),
                units: plan_units.clone(),
            };
            let design_stage = generate_shooting_scene(
                self,
                workflow_run_id,
                &prepared,
                &plan,
                offset,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
            let generated_unit =
                !layout_stage.restored || !adaptation_stage.restored || !design_stage.restored;
            scenes.push(design_stage.output);

            if stop_after_next_unit && generated_unit && scenes.len() < total_unit_count {
                let script = assemble_available_shooting_script(&plan, scenes)?;
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

        let layout = ChapterScreenplayLayoutV1 {
            schema_version: "chapter_screenplay_layout_v1".to_string(),
            directing_statement: skeleton.directing_statement.clone(),
            units: layout_units,
        };
        validate_screenplay_layout(&layout, &prepared.source_spans)?;
        let plan = ChapterScreenplayPlan {
            directing_statement: skeleton.directing_statement,
            units: plan_units,
        };
        validate_screenplay_plan(&plan, &prepared.source_spans)?;
        let script = assemble_chapter_shooting_script(&plan, scenes)?;
        validate_script_source_spans(&script, &prepared.source_spans)?;
        validate_script_against_plan(&script, &plan)?;
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
    use a4n_db::project::{ResolvedChapterSource, ResolvedChapterSourceKind};

    use super::shared_workflow_source;

    #[test]
    fn derived_project_reads_partial_workflow_from_shared_root_project() {
        let source = ResolvedChapterSource {
            source_kind: ResolvedChapterSourceKind::Snapshot,
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
