use super::{OrchestratorContext, ensure_plan_belongs_to_source, visual_plan_db_error};
use a4n_api_contract::{agent::AgentResourceRef, chapter_visual_plan::*};
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_error::AppError;

impl OrchestratorContext {
    pub async fn read_chapter_visual_plan_for_user(
        &self,
        user_id: i64,
        input: ReadChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanReadResult, AppError> {
        validate_ids(input.project_id, input.source_chapter_id)?;
        if input.expected_revision.is_some() && input.record_id.is_none() {
            return Err(AppError::InvalidArgument(
                "expected_revision 必须同时提供 record_id".into(),
            ));
        }
        let set = self
            .current_chapter_visual_plan_for_user_validated(
                user_id,
                ChapterVisualPlanSourceInput {
                    project_id: input.project_id,
                    source_chapter_id: input.source_chapter_id,
                },
            )
            .await?;
        let current = if let Some(id) = input.record_id {
            if id <= 0 {
                return Err(AppError::InvalidArgument("record_id 必须大于 0".into()));
            }
            let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                id,
            )
            .await
            .map_err(visual_plan_db_error)?;
            ensure_plan_belongs_to_source(&plan, &set.source)?;
            if let Some(revision) = input.expected_revision {
                check_revision(&plan, revision)?;
            }
            Some(plan)
        } else {
            set.plans.draft.or(set.plans.confirmed)
        };
        let resource = current.as_ref().map(resource_ref);
        let projection = match &input.view {
            VisualPlanReadView::Source {
                offset,
                limit,
                expected_hash,
            } => {
                if expected_hash
                    .as_ref()
                    .is_some_and(|hash| hash != &set.source.content_sha256)
                {
                    return Err(AppError::InvalidArgument(
                        "原文版本已变化，请重新读取".into(),
                    ));
                }
                let total_chars = set.source.content.chars().count();
                let (start, end) = page_range(*offset, *limit, total_chars, 2_000, 2_000)?;
                VisualPlanProjection::Source {
                    text: set
                        .source
                        .content
                        .chars()
                        .skip(start)
                        .take(end - start)
                        .collect(),
                    offset: *offset,
                    total_chars,
                    next_offset: next_offset(end, total_chars)?,
                }
            }
            view => project_plan(
                current
                    .as_ref()
                    .map_or(&set.starter_plan, |current| &current.plan),
                view,
            )?,
        };
        Ok(ChapterVisualPlanReadResult {
            resource,
            source_hash: set.source.content_sha256,
            projection,
        })
    }

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

    pub async fn validate_chapter_visual_plan_for_user(
        &self,
        user_id: i64,
        input: ValidateChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanValidationResult, AppError> {
        validate_ids(input.project_id, input.source_chapter_id)?;
        self.verify_project_or_derived_family_access_validated(user_id, input.project_id)
            .await?;
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        let current = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
            self.database.connection(),
            input.record_id,
        )
        .await
        .map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        check_revision(&current, input.expected_revision)?;
        Ok(ChapterVisualPlanValidationResult {
            resource: resource_ref(&current),
            validation: validate_plan(&current.plan, input.mode),
        })
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

fn resource_ref(current: &ParsedChapterVisualPlan) -> AgentResourceRef {
    AgentResourceRef {
        kind: "chapter_visual_plan".into(),
        id: current.record.id.to_string(),
        revision: Some(current.record.revision.to_string()),
        hash: Some(current.record.plan_sha256.clone()),
        label: Some(current.record.status.clone()),
    }
}

fn page_range(
    offset: u32,
    limit: Option<u32>,
    total: usize,
    default: u32,
    max: u32,
) -> Result<(usize, usize), AppError> {
    let limit = limit.unwrap_or(default);
    if limit == 0 || limit > max || offset as usize > total {
        return Err(AppError::InvalidArgument(format!(
            "分页范围无效：offset={offset}，limit 须为 1..={max}，total={total}"
        )));
    }
    let start = offset as usize;
    Ok((start, start.saturating_add(limit as usize).min(total)))
}

fn next_offset(end: usize, total: usize) -> Result<Option<u32>, AppError> {
    (end < total)
        .then(|| u32::try_from(end))
        .transpose()
        .map_err(|_| AppError::InvalidArgument("分页长度超出 u32".into()))
}

fn project_plan(
    plan: &ChapterVisualPlanV3,
    view: &VisualPlanReadView,
) -> Result<VisualPlanProjection, AppError> {
    plan.validate_draft()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    match view {
        VisualPlanReadView::Outline { offset, limit } => {
            let total = plan.segments.len();
            let (start, end) = page_range(*offset, *limit, total, 20, 100)?;
            let items = plan.segments[start..end]
                .iter()
                .map(|segment| {
                    let start_ms = plan
                        .point(&segment.start_point_id)
                        .expect("validated start")
                        .at_ms;
                    let end_ms = plan
                        .point(&segment.end_point_id)
                        .expect("validated end")
                        .at_ms;
                    VisualPlanOutlineItem {
                        segment_id: segment.id.clone(),
                        title: segment.title.clone(),
                        start_ms,
                        end_ms,
                        scene_binding: segment.scene_binding.clone(),
                        point_count: plan
                            .points
                            .iter()
                            .filter(|point| point.at_ms >= start_ms && point.at_ms < end_ms)
                            .count(),
                    }
                })
                .collect();
            Ok(VisualPlanProjection::Outline {
                items,
                total,
                next_offset: next_offset(end, total)?,
            })
        }
        VisualPlanReadView::Point { point_id } => {
            let point = plan
                .point(point_id)
                .ok_or_else(|| AppError::NotFound(format!("图片点不存在: {point_id}")))?;
            Ok(VisualPlanProjection::Point {
                point: point.clone(),
            })
        }
        VisualPlanReadView::Segment { segment_id } => {
            let segment = plan
                .segments
                .iter()
                .find(|segment| segment.id == *segment_id)
                .ok_or_else(|| AppError::NotFound(format!("镜头段不存在: {segment_id}")))?;
            let start = plan
                .point(&segment.start_point_id)
                .expect("validated start")
                .at_ms;
            let end = plan
                .point(&segment.end_point_id)
                .expect("validated end")
                .at_ms;
            let points = plan
                .points
                .iter()
                .filter(|point| point.at_ms >= start && point.at_ms <= end)
                .cloned()
                .collect();
            Ok(VisualPlanProjection::Segment {
                segment: segment.clone(),
                points,
            })
        }
        VisualPlanReadView::Source { .. } => Err(AppError::InvalidArgument(
            "原文由章节 source owner 读取".into(),
        )),
    }
}

pub(crate) fn project_editor_visual_plan(
    plan: &ChapterVisualPlanV3,
    view: &VisualPlanReadView,
) -> Result<VisualPlanProjection, AppError> {
    project_plan(plan, view)
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

    #[test]
    fn reads_are_bounded_and_missing_points_are_not_redirected() {
        let plan = ChapterVisualPlanV3::starter();
        assert!(
            project_plan(
                &plan,
                &VisualPlanReadView::Outline {
                    offset: 0,
                    limit: Some(101)
                }
            )
            .is_err()
        );
        assert!(
            project_plan(
                &plan,
                &VisualPlanReadView::Point {
                    point_id: "missing".into()
                }
            )
            .is_err()
        );
        let result = project_plan(
            &plan,
            &VisualPlanReadView::Segment {
                segment_id: "shot-1".into(),
            },
        )
        .unwrap();
        let VisualPlanProjection::Segment { points, .. } = result else {
            panic!("segment projection expected")
        };
        assert_eq!(points.len(), 2);
        assert!(
            serde_json::from_str::<VisualPlanEditOperation>(
                r#"{"kind":"move_point","point_id":"start","at_ms":1,"unknown":true}"#
            )
            .is_err()
        );
    }
}
