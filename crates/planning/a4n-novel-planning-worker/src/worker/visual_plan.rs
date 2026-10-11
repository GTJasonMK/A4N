use super::errors::{source_error, visual_plan_error};
use super::helpers::*;
use crate::*;

mod mutations;
mod validation;

impl WorkerState {
    pub(super) async fn get_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanSourceInput>,
    ) -> Result<ChapterVisualPlanSetView, WorkerFailure> {
        validate_request(&request, GET_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        let scene_choices =
            a4n_db::project::authoring_scene_profiles(self.database.connection(), &source)
                .await
                .map_err(source_error)?
                .into_iter()
                .map(|profile| {
                    a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSceneChoice {
                        id: profile.id,
                        project_id: profile.project_id,
                        name: profile.name,
                    }
                })
                .collect();
        Ok(ChapterVisualPlanSetView {
            edit_cursor: a4n_db::chapter_visual_plan::visual_plan_edit_cursor(
                self.database.connection(),
                plans.draft.as_ref().or(plans.confirmed.as_ref()),
            )
            .await
            .map_err(visual_plan_error)?,
            scene_choices,
            source: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSourceView {
                chapter_number: source.chapter_number,
                title: source.title,
                content: source.content,
                content_sha256: source.content_sha256,
            },
            starter_plan: a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter(),
            draft: plans.draft.map(visual_plan_view),
            confirmed: plans.confirmed.map(visual_plan_view),
        })
    }

    pub(super) async fn read_visual_plan(
        &self,
        request: WorkerEnvelope<ReadChapterVisualPlanInput>,
    ) -> Result<ChapterVisualPlanReadResult, WorkerFailure> {
        validate_request(&request, READ_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_optional_target_revision(&request, request.payload.expected_revision)?;
        let user_id = request.principal.require_user_id()?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        let current = if let Some(record_id) = request.payload.record_id {
            let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                record_id,
            )
            .await
            .map_err(visual_plan_error)?;
            ensure_visual_plan_belongs_to_source_record(&plan, &source)?;
            if let Some(expected_revision) = request.payload.expected_revision
                && plan.record.revision != expected_revision
            {
                return Err(visual_plan_revision_conflict(
                    plan.record.id,
                    expected_revision,
                ));
            }
            Some(plan)
        } else {
            plans.draft.or(plans.confirmed)
        };
        let resource = current.as_ref().map(visual_plan_resource);
        let starter_plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
        let projection = match &request.payload.view {
            VisualPlanReadView::Edits { after_sequence } => {
                let items = a4n_db::chapter_visual_plan::visual_plan_edits(
                    self.database.connection(),
                    &source,
                    *after_sequence,
                )
                .await
                .map_err(visual_plan_error)?;
                let next_sequence = items
                    .last()
                    .and_then(|item| item.edit_receipt.as_ref())
                    .map_or(*after_sequence, |receipt| receipt.sequence);
                VisualPlanProjection::Edits {
                    items: items.into_iter().map(visual_plan_view).collect(),
                    next_sequence,
                }
            }
            VisualPlanReadView::Source {
                offset,
                limit,
                expected_hash,
            } => {
                if expected_hash
                    .as_ref()
                    .is_some_and(|hash| hash != &source.content_sha256)
                {
                    return Err(WorkerFailure::conflict("原文版本已变化，请重新读取"));
                }
                let total_chars = source.content.chars().count();
                let (start, end) = page_range(*offset, *limit, total_chars, 2_000, 2_000)?;
                VisualPlanProjection::Source {
                    text: source
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
            view => a4n_api_contract::chapter_visual_plan::project_visual_plan(
                current.as_ref().map_or(&starter_plan, |plan| &plan.plan),
                view,
            )
            .map_err(projection_error)?,
        };
        Ok(ChapterVisualPlanReadResult {
            chapter_id: source.source_chapter_id,
            chapter_number: source.chapter_number,
            resource,
            source_hash: source.content_sha256,
            projection,
        })
    }
}

pub(super) fn visual_plan_view(
    value: a4n_db::chapter_visual_plan::ParsedChapterVisualPlan,
) -> ChapterVisualPlanView {
    ChapterVisualPlanView {
        edit_receipt: value.edit_receipt,
        record: ChapterVisualPlanRecordSummary {
            id: value.record.id,
            plan_version: value.record.plan_version,
            revision: value.record.revision,
            status: value.status,
            authoring_mode: value.authoring_mode,
            source_content_sha256: value.record.source_content_sha256,
            plan_sha256: value.record.plan_sha256,
            confirmed_at: value.record.confirmed_at.map(|value| value.to_string()),
        },
        plan: value.plan,
    }
}

fn visual_plan_resource(
    current: &a4n_db::chapter_visual_plan::ParsedChapterVisualPlan,
) -> a4n_api_contract::agent::AgentResourceRef {
    a4n_api_contract::agent::AgentResourceRef {
        kind: "chapter_visual_plan".into(),
        id: current.record.id.to_string(),
        revision: Some(current.record.revision.to_string()),
        hash: Some(current.record.plan_sha256.clone()),
        label: Some(current.record.status.clone()),
    }
}

fn ensure_visual_plan_belongs_to_source_record(
    plan: &a4n_db::chapter_visual_plan::ParsedChapterVisualPlan,
    source: &a4n_domain::chapter::ResolvedChapterSource,
) -> Result<(), WorkerFailure> {
    if plan.record.project_id != source.requested_project_id
        || plan.record.root_project_id != source.root_project_id
        || plan.record.source_chapter_id != source.source_chapter_id
        || plan.record.source_snapshot_id != source.source_snapshot_id
        || plan.record.source_snapshot_chapter_id != source.source_snapshot_chapter_id
    {
        return Err(WorkerFailure::invalid_request(
            "章节视觉编排与当前章节正文不匹配",
        ));
    }
    Ok(())
}

fn visual_plan_revision_conflict(record_id: i64, expected_revision: i32) -> WorkerFailure {
    WorkerFailure::conflict(format!(
        "chapter_visual_plan_revision_conflict:id={record_id},expected={expected_revision}"
    ))
}

fn page_range(
    offset: u32,
    limit: Option<u32>,
    total: usize,
    default: u32,
    max: u32,
) -> Result<(usize, usize), WorkerFailure> {
    let limit = limit.unwrap_or(default);
    if limit == 0 || limit > max || offset as usize > total {
        return Err(WorkerFailure::invalid_request(format!(
            "分页范围无效：offset={offset}，limit 须为 1..={max}，total={total}"
        )));
    }
    let start = offset as usize;
    Ok((start, start.saturating_add(limit as usize).min(total)))
}

fn next_offset(end: usize, total: usize) -> Result<Option<u32>, WorkerFailure> {
    (end < total)
        .then(|| u32::try_from(end))
        .transpose()
        .map_err(|_| WorkerFailure::invalid_request("分页长度超出 u32"))
}

fn projection_error(
    error: a4n_api_contract::chapter_visual_plan::VisualPlanProjectionError,
) -> WorkerFailure {
    match error {
        a4n_api_contract::chapter_visual_plan::VisualPlanProjectionError::InvalidRequest(
            message,
        ) => WorkerFailure::invalid_request(message),
        a4n_api_contract::chapter_visual_plan::VisualPlanProjectionError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
    }
}
