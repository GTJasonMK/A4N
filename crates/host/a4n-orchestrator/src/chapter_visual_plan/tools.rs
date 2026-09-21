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
        let mut current = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
            self.database.connection(),
            input.record_id,
        )
        .await
        .map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        check_revision(&current, input.expected_revision)?;
        if current.status != ChapterVisualPlanStatus::Draft {
            return Err(AppError::InvalidArgument(
                "局部编辑只接受已保存的 draft；请先保存 collaborative 草稿".into(),
            ));
        }
        let changes = apply_operation(&mut current.plan, &input.operation)?;
        let validation = validate_plan(&current.plan, VisualPlanValidationMode::Draft);
        if !validation.valid {
            let diagnostic = &validation.diagnostics[0];
            return Err(AppError::InvalidArgument(format!(
                "{}: {}",
                diagnostic.path, diagnostic.message
            )));
        }
        if !changes.is_empty() {
            current = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
                self.database.connection(),
                a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                    source,
                    author_user_id: user_id,
                    authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
                    expected: Some(a4n_db::chapter_visual_plan::ExpectedVisualPlanDraft {
                        record_id: input.record_id,
                        revision: input.expected_revision,
                    }),
                    expected_source_hash: input.expected_source_hash,
                    plan: current.plan,
                },
            )
            .await
            .map_err(visual_plan_db_error)?;
        }
        Ok(ChapterVisualPlanEditResult {
            resource: resource_ref(&current),
            changed: !changes.is_empty(),
            changes,
            validation,
            visual_plan: super::chapter_visual_plan_view(current),
        })
    }
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

fn check_revision(current: &ParsedChapterVisualPlan, expected: i32) -> Result<(), AppError> {
    if expected <= 0 || current.record.revision != expected {
        return Err(visual_plan_db_error(
            a4n_db::chapter_visual_plan::ChapterVisualPlanDbError::RevisionConflict {
                record_id: current.record.id,
                expected_revision: expected,
            },
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
    let before = serde_json::to_value(&*plan)?;
    let after = serde_json::to_value(&next)?;
    let mut changes = Vec::new();
    for field in ["frame", "points", "segments"] {
        if before[field] != after[field] {
            changes.push(VisualPlanFieldChange {
                object_id: "timeline".into(),
                field: field.into(),
                before: Some(before[field].to_string()),
                after: Some(after[field].to_string()),
            });
        }
    }
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
