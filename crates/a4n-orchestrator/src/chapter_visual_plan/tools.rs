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

fn find_shot<'a>(
    plan: &'a ChapterVisualPlanV2,
    shot_id: &str,
) -> Result<(&'a VisualSceneV2, &'a VisualShotV2), AppError> {
    plan.scenes
        .iter()
        .find_map(|scene| {
            scene
                .shots
                .iter()
                .find(|shot| shot.id == shot_id)
                .map(|shot| (scene, shot))
        })
        .ok_or_else(|| AppError::NotFound(format!("镜头不存在: {shot_id}")))
}

fn project_plan(
    plan: &ChapterVisualPlanV2,
    view: &VisualPlanReadView,
) -> Result<VisualPlanProjection, AppError> {
    match view {
        VisualPlanReadView::Outline { offset, limit } => {
            let items = plan
                .scenes
                .iter()
                .flat_map(|scene| {
                    let row = |shot: Option<&VisualShotV2>| VisualPlanOutlineItem {
                        scene_id: scene.id.clone(),
                        scene_title: scene.title.clone(),
                        shot_id: shot.map(|shot| shot.id.clone()),
                        title: shot.map(|shot| shot.title.clone()),
                        duration_ms: shot.map_or(0, |shot| shot.duration_ms),
                        keyframe_count: shot.map_or(0, |shot| shot.keyframes.len()),
                    };
                    if scene.shots.is_empty() {
                        vec![row(None)]
                    } else {
                        scene.shots.iter().map(|shot| row(Some(shot))).collect()
                    }
                })
                .collect::<Vec<_>>();
            let total = items.len();
            let (start, end) = page_range(*offset, *limit, total, 20, 100)?;
            Ok(VisualPlanProjection::Outline {
                items: items.into_iter().skip(start).take(end - start).collect(),
                total,
                next_offset: next_offset(end, total)?,
            })
        }
        VisualPlanReadView::Shot {
            shot_id,
            offset,
            limit,
        } => {
            let (scene, shot) = find_shot(plan, shot_id)?;
            let total = shot.keyframes.len();
            let (start, end) = page_range(*offset, *limit, total, 20, 100)?;
            Ok(VisualPlanProjection::Shot {
                scene_id: scene.id.clone(),
                shot: VisualPlanShotDetails {
                    id: shot.id.clone(),
                    title: shot.title.clone(),
                    duration_ms: shot.duration_ms,
                    narrative_purpose: shot.narrative_purpose.clone(),
                    composition: shot.composition.clone(),
                    lens_intent: shot.lens_intent.clone(),
                    keyframes: shot
                        .keyframes
                        .iter()
                        .skip(start)
                        .take(end - start)
                        .map(|frame| VisualPlanKeyframeIndex {
                            id: frame.id.clone(),
                            at_ms: frame.at_ms,
                            label: frame.label.clone(),
                        })
                        .collect(),
                    total_keyframes: total,
                    next_offset: next_offset(end, total)?,
                },
            })
        }
        VisualPlanReadView::Keyframe {
            shot_id,
            keyframe_id,
        } => {
            let (_, shot) = find_shot(plan, shot_id)?;
            let keyframe = shot
                .keyframes
                .iter()
                .find(|frame| frame.id == *keyframe_id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("镜头 {shot_id} 中没有关键帧 {keyframe_id}"))
                })?;
            Ok(VisualPlanProjection::Keyframe {
                shot_id: shot_id.clone(),
                keyframe: keyframe.clone(),
            })
        }
        VisualPlanReadView::Entity { entity_id } => {
            let entity = plan
                .entities
                .iter()
                .find(|entity| entity.id == *entity_id)
                .ok_or_else(|| AppError::NotFound(format!("实体不存在: {entity_id}")))?;
            Ok(VisualPlanProjection::Entity {
                entity: entity.clone(),
            })
        }
        VisualPlanReadView::Source { .. } => Err(AppError::InvalidArgument(
            "原文由章节 source owner 读取".into(),
        )),
    }
}

pub(crate) fn project_editor_visual_plan(
    plan: &ChapterVisualPlanV2,
    view: &VisualPlanReadView,
) -> Result<VisualPlanProjection, AppError> {
    project_plan(plan, view)
}

fn apply_operation(
    plan: &mut ChapterVisualPlanV2,
    operation: &VisualPlanEditOperation,
) -> Result<Vec<VisualPlanFieldChange>, AppError> {
    let VisualPlanEditOperation::UpdateShotFields { shot_id, changes } = operation;
    if changes == &VisualShotFieldChanges::default() {
        return Err(AppError::InvalidArgument("changes 不能为空".into()));
    }
    let shot = plan
        .scenes
        .iter_mut()
        .flat_map(|scene| &mut scene.shots)
        .find(|shot| shot.id == *shot_id)
        .ok_or_else(|| AppError::NotFound(format!("镜头不存在: {shot_id}")))?;
    let mut diff = Vec::new();
    for (field, before, after) in [
        (
            "title",
            Some(shot.title.clone()),
            changes.title.as_ref().map(|value| Some(value.clone())),
        ),
        (
            "narrative_purpose",
            Some(shot.narrative_purpose.clone()),
            changes
                .narrative_purpose
                .as_ref()
                .map(|value| Some(value.clone())),
        ),
        (
            "composition",
            shot.composition.clone(),
            changes.composition.clone(),
        ),
        (
            "lens_intent",
            shot.lens_intent.clone(),
            changes.lens_intent.clone(),
        ),
    ] {
        if let Some(after) = after {
            if after
                .as_ref()
                .is_some_and(|text| text.trim().is_empty() || text.chars().count() > 4_000)
            {
                return Err(AppError::InvalidArgument(format!(
                    "{field} 必须为 1..=4000 字符；可选文本使用 null 清空"
                )));
            }
            if before != after {
                diff.push(VisualPlanFieldChange {
                    object_id: shot_id.clone(),
                    field: field.into(),
                    before,
                    after,
                });
            }
        }
    }
    if let Some(value) = &changes.title {
        shot.title.clone_from(value);
    }
    if let Some(value) = &changes.narrative_purpose {
        shot.narrative_purpose.clone_from(value);
    }
    if let Some(value) = &changes.composition {
        shot.composition.clone_from(value);
    }
    if let Some(value) = &changes.lens_intent {
        shot.lens_intent.clone_from(value);
    }
    Ok(diff)
}

fn validate_plan(
    plan: &ChapterVisualPlanV2,
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
    use serde_json::json;

    fn plan() -> ChapterVisualPlanV2 {
        serde_json::from_value(json!({
            "schema_version":"chapter_visual_plan_v2", "frame":{"width":1920,"height":1080,"background":null}, "entities":[],
            "scenes":[{"id":"scene-1","title":"场景","narrative_purpose":"叙事","editing_mode":"continuity","intercut_group_id":null,"location":null,"time_of_day":null,"participant_entity_ids":[],"focal_entity_id":null,
                "shots":[{"id":"shot-1","title":"镜头","duration_ms":1000,"narrative_purpose":"推进","coverage":"must_show","shot_size":null,"camera_angle":null,"camera_height":null,"lens_intent":null,"composition":"原构图","keyframes":[],"entity_events":[],"camera_events":[],"sound_events":[],"technique_events":[],"continuity_notes":["保持连续"],"transition_to_next":null}]}]
        })).unwrap()
    }

    #[test]
    fn local_edit_preserves_every_unrequested_field_and_reports_noop() {
        let mut value = plan();
        let original = value.clone();
        let operation = serde_json::from_value(json!({"kind":"update_shot_fields","shot_id":"shot-1","changes":{"composition":"门框遮挡"}})).unwrap();
        let diff = apply_operation(&mut value, &operation).unwrap();
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].before.as_deref(), Some("原构图"));
        assert!(apply_operation(&mut value, &operation).unwrap().is_empty());
        value.scenes[0].shots[0].composition = original.scenes[0].shots[0].composition.clone();
        assert_eq!(value, original);
        assert!(validate_plan(&value, VisualPlanValidationMode::Draft).valid);
        let strict = validate_plan(&value, VisualPlanValidationMode::Confirmed);
        assert!(!strict.valid);
        assert!(strict.diagnostics[0].path.contains("keyframes"));
    }

    #[test]
    fn contract_rejects_identity_timing_unknown_fields_and_empty_changes() {
        for changes in [
            json!({"id":"other"}),
            json!({"duration_ms":2000}),
            json!({"title":null}),
        ] {
            assert!(serde_json::from_value::<VisualShotFieldChanges>(changes).is_err());
        }
        let empty = VisualPlanEditOperation::UpdateShotFields {
            shot_id: "shot-1".into(),
            changes: Default::default(),
        };
        assert!(apply_operation(&mut plan(), &empty).is_err());
        let clear = serde_json::from_value(
            json!({"kind":"update_shot_fields","shot_id":"shot-1","changes":{"composition":null}}),
        )
        .unwrap();
        let mut value = plan();
        apply_operation(&mut value, &clear).unwrap();
        assert!(value.scenes[0].shots[0].composition.is_none());
    }

    #[test]
    fn outline_is_bounded_and_missing_target_is_not_redirected() {
        let value = plan();
        let result = project_plan(
            &value,
            &VisualPlanReadView::Outline {
                offset: 0,
                limit: Some(1),
            },
        )
        .unwrap();
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("原构图"));
        assert!(text.contains("shot-1"));
        assert!(
            project_plan(
                &value,
                &VisualPlanReadView::Shot {
                    shot_id: "deleted".into(),
                    offset: 0,
                    limit: None
                }
            )
            .is_err()
        );
        assert!(page_range(0, Some(101), 500, 20, 100).is_err());
    }
}
