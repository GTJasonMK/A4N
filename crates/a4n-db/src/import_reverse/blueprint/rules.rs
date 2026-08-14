use a4n_domain::project::{
    BLUEPRINT_MAX_TOTAL_CHAPTERS, BLUEPRINT_MIN_TOTAL_CHAPTERS, CHAPTERS_PER_PART,
    is_valid_blueprint_total_chapters, needs_part_outlines_for_chapters,
};
use chrono::NaiveDateTime;

use crate::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseBlueprintApplyPlanRecord,
    ReverseBlueprintApplyRecord,
};

pub(in crate::import_reverse) fn resolve_reverse_blueprint_apply_record(
    draft: &ReverseBlueprintApplyDraft,
    apply_plan: &ReverseBlueprintApplyPlanRecord,
    applied_at: NaiveDateTime,
) -> Result<ReverseBlueprintApplyRecord, ImportReverseDbError> {
    let existing_total_chapters = apply_plan.existing_blueprint_total_chapters;
    if let (Some(existing_total), Some(incoming_total)) =
        (existing_total_chapters, draft.incoming_total_chapters)
        && existing_total != incoming_total
    {
        return Err(ImportReverseDbError::InvalidArgument(format!(
            "reverse_blueprint.total_chapters 必须与现有蓝图总章节数一致（existing={existing_total}, incoming={incoming_total}）"
        )));
    }

    let total_chapters = existing_total_chapters
        .or(draft.incoming_total_chapters)
        .or(apply_plan.project_total_chapters)
        .ok_or_else(|| {
            ImportReverseDbError::InvalidArgument(
                "reverse_blueprint.total_chapters 不能为空".to_string(),
            )
        })?;
    if !is_valid_blueprint_total_chapters(total_chapters) {
        return Err(ImportReverseDbError::InvalidArgument(format!(
            "reverse_blueprint.total_chapters 必须在 {BLUEPRINT_MIN_TOTAL_CHAPTERS} 到 {BLUEPRINT_MAX_TOTAL_CHAPTERS} 之间"
        )));
    }
    let needs_part_outlines = needs_part_outlines_for_chapters(total_chapters);
    if let Some(incoming) = draft.incoming_needs_part_outlines
        && incoming != needs_part_outlines
    {
        return Err(ImportReverseDbError::InvalidArgument(format!(
            "reverse_blueprint.needs_part_outlines 必须与有效总章节数推导结果一致（expected={needs_part_outlines}, incoming={incoming}）"
        )));
    }

    Ok(ReverseBlueprintApplyRecord {
        applied_at,
        title: draft.title.clone(),
        genre: draft.genre.clone(),
        style: draft.style.clone(),
        tone: draft.tone.clone(),
        target_audience: draft.target_audience.clone(),
        one_sentence_summary: draft.one_sentence_summary.clone(),
        full_synopsis: draft.full_synopsis.clone(),
        world_setting: draft.world_setting.clone(),
        total_chapters,
        needs_part_outlines,
        chapters_per_part: needs_part_outlines.then_some(CHAPTERS_PER_PART),
        current_version: apply_plan.next_version.clone(),
    })
}
