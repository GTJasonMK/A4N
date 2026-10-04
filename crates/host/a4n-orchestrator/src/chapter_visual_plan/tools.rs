use super::{OrchestratorContext, ensure_plan_belongs_to_source, visual_plan_db_error};
use a4n_api_contract::{agent::AgentResourceRef, chapter_visual_plan::*};
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_error::AppError;

impl OrchestratorContext {
    pub async fn edit_chapter_visual_plan_draft_for_user(
        &self,
        user_id: i64,
        input: EditChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanEditResult, AppError> {
        validate_ids(input.project_id, input.source_chapter_id)?;
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let payload_hash = a4n_domain::generation::canonical_json_sha256(&(user_id, &input))?;
        let operation_id = input
            .operation_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        if input
            .expected_source_hash
            .as_ref()
            .is_some_and(|hash| hash != &source.content_sha256)
        {
            return Err(AppError::InvalidArgument(
                "原文版本已变化，不能提交基于旧原文的修改".into(),
            ));
        }
        if let Some((_, saved)) = a4n_db::chapter_visual_plan::replay_visual_plan_edit(
            self.database.executor(),
            input.project_id,
            &operation_id,
            &payload_hash,
        )
        .await
        .map_err(visual_plan_db_error)?
        {
            ensure_plan_belongs_to_source(&saved, &source)?;
            return edit_result(saved);
        }
        let mut current = a4n_db::chapter_visual_plan::visual_plan_revision(
            self.database.executor(),
            input.record_id,
            input.expected_revision,
        )
        .await
        .map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        if !matches!(
            current.status,
            ChapterVisualPlanStatus::Draft | ChapterVisualPlanStatus::Confirmed
        ) {
            return Err(AppError::InvalidArgument(
                "编排状态已变化，请重新读取可编辑版本".into(),
            ));
        }
        let base = current.plan.clone();
        let changes = apply_operation(&mut current.plan, &input.operation)?;
        if changes.is_empty() {
            let saved = a4n_db::chapter_visual_plan::record_visual_plan_noop(
                self.database.executor(),
                &current,
                &operation_id,
                &payload_hash,
            )
            .await
            .map_err(visual_plan_db_error)?;
            return edit_result(saved);
        }
        let expected = if current.status == ChapterVisualPlanStatus::Confirmed {
            let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
                self.database.executor(),
                &source,
            )
            .await
            .map_err(visual_plan_db_error)?;
            if let Some(draft) = plans.draft {
                current.plan = draft
                    .plan
                    .merge_edit(&base, &current.plan)
                    .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
                Some(a4n_domain::chapter_visual_plan::ExpectedVisualPlanDraft {
                    record_id: draft.record.id,
                    revision: draft.record.revision,
                })
            } else {
                None
            }
        } else {
            Some(a4n_domain::chapter_visual_plan::ExpectedVisualPlanDraft {
                record_id: input.record_id,
                revision: input.expected_revision,
            })
        };
        let validation = validate_plan(&current.plan, VisualPlanValidationMode::Draft);
        if !validation.valid {
            let diagnostic = &validation.diagnostics[0];
            return Err(AppError::InvalidArgument(format!(
                "{}: {}",
                diagnostic.path, diagnostic.message
            )));
        }
        current = a4n_db::chapter_visual_plan::save_chapter_visual_plan_edit(
            self.database.executor(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
                expected,
                expected_source_hash: input.expected_source_hash,
                plan: current.plan,
            },
            &operation_id,
            &payload_hash,
        )
        .await
        .map_err(visual_plan_db_error)?;
        edit_result(current)
    }
}

fn edit_result(current: ParsedChapterVisualPlan) -> Result<ChapterVisualPlanEditResult, AppError> {
    let receipt = current
        .edit_receipt
        .as_ref()
        .ok_or_else(|| AppError::InvalidState {
            current: "保存结果缺少编辑回执".into(),
            operation: "返回导演编辑结果".into(),
        })?;
    let changes = field_changes(receipt.changes.clone());
    Ok(ChapterVisualPlanEditResult {
        resource: resource_ref(&current),
        changed: !changes.is_empty(),
        changes,
        validation: validate_plan(&current.plan, VisualPlanValidationMode::Draft),
        visual_plan: super::chapter_visual_plan_view(current),
    })
}

fn field_changes(changes: Vec<VisualPlanChange>) -> Vec<VisualPlanFieldChange> {
    changes
        .into_iter()
        .map(|change| VisualPlanFieldChange {
            object_id: change.object_id,
            collection: change.collection,
            field: change.field,
            before: change.before.map(|value| value.to_string()),
            after: change.after.map(|value| value.to_string()),
        })
        .collect()
}

fn resource_ref(current: &ParsedChapterVisualPlan) -> AgentResourceRef {
    AgentResourceRef {
        kind: "chapter_visual_plan".into(),
        id: current.record.id.to_string(),
        revision: Some(current.record.revision.to_string()),
        hash: Some(current.record.plan_sha256.clone()),
        label: Some(current.record.status.clone()),
    }
}

fn validate_ids(project_id: i64, chapter_id: i64) -> Result<(), AppError> {
    if project_id <= 0 || chapter_id <= 0 {
        return Err(AppError::InvalidArgument(
            "project_id 和 source_chapter_id 必须大于 0".into(),
        ));
    }
    Ok(())
}

fn apply_operation(
    plan: &mut ChapterVisualPlanV3,
    operation: &VisualPlanEditOperation,
) -> Result<Vec<VisualPlanFieldChange>, AppError> {
    let next = plan
        .edited(operation)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let changes = field_changes(plan.changes_to(&next));
    *plan = next;
    Ok(changes)
}

fn validate_plan(
    plan: &ChapterVisualPlanV3,
    mode: VisualPlanValidationMode,
) -> VisualPlanValidationResult {
    let result = match mode {
        VisualPlanValidationMode::Draft => plan.validate_draft(),
        VisualPlanValidationMode::Confirmed => plan.validate_confirmed(),
    };
    VisualPlanValidationResult {
        mode,
        valid: result.is_ok(),
        diagnostics: result
            .err()
            .into_iter()
            .map(|error| VisualPlanDiagnostic {
                path: error.path,
                message: error.message,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_operations_share_domain_rules_and_report_noop() {
        let mut plan = ChapterVisualPlanV3::starter();
        let operation = VisualPlanEditOperation::ScaleTimeline {
            duration_ms: 20_000,
        };
        assert_eq!(apply_operation(&mut plan, &operation).unwrap().len(), 1);
        assert!(apply_operation(&mut plan, &operation).unwrap().is_empty());
        let original = plan.clone();
        assert!(
            apply_operation(
                &mut plan,
                &VisualPlanEditOperation::MovePoint {
                    point_id: "start".into(),
                    at_ms: 1
                }
            )
            .is_err()
        );
        assert_eq!(plan, original);
        assert!(validate_plan(&plan, VisualPlanValidationMode::Draft).valid);
        assert!(!validate_plan(&plan, VisualPlanValidationMode::Confirmed).valid);
    }
}
