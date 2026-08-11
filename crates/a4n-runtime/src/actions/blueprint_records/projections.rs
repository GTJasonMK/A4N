use a4n_core::models::project::{blueprint_version, novel_blueprint};
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;

use crate::actions::request_preview::ChatStatusPreviewStep;

use super::{BlueprintSummary, BlueprintVersionSummary};

pub(super) fn blueprint_summary(
    bp: &novel_blueprint::Model,
    sources_need_review: bool,
) -> BlueprintSummary {
    BlueprintSummary {
        id: bp.id,
        project_id: bp.project_id,
        title: bp.title.clone(),
        genre: bp.genre.clone(),
        style: bp.style.clone(),
        tone: bp.tone.clone(),
        target_audience: bp.target_audience.clone(),
        one_sentence_summary: bp.one_sentence_summary.clone(),
        full_synopsis: bp.full_synopsis.clone(),
        world_setting: bp.world_setting.clone(),
        current_version: bp.current_version.clone(),
        total_chapters: bp.total_chapters,
        chapters_per_part: bp.chapters_per_part,
        source_review_status: if sources_need_review {
            "needs_review"
        } else {
            "not_required"
        }
        .to_string(),
    }
}

pub(super) fn blueprint_version_summary(v: &blueprint_version::Model) -> BlueprintVersionSummary {
    BlueprintVersionSummary {
        id: v.id,
        version: v.version.clone(),
        change_description: v.change_description.clone(),
        created_at: v.created_at.to_string(),
        created_by: v.created_by.clone(),
    }
}

pub(super) fn blueprint_preview_step(
    stage: BlueprintGenerationPreviewStep,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        stage.key,
        stage.title,
        Some(stage.prompt_name),
        stage.state,
        stage.note,
        stage.request,
    )
}
