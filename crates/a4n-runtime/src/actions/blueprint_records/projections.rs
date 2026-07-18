use a4n_core::models::project::{blueprint_version, novel_blueprint};
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;

use crate::actions::request_preview::ChatStatusPreviewStep;

use super::{BlueprintSummary, BlueprintVersionSummary};

pub(super) fn blueprint_summary(bp: &novel_blueprint::Model) -> BlueprintSummary {
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
        needs_part_outlines: bp.needs_part_outlines,
        total_chapters: bp.total_chapters,
        chapters_per_part: bp.chapters_per_part,
        avatar_svg: bp.avatar_svg.clone(),
        avatar_animal: bp.avatar_animal.clone(),
        created_at: Some(bp.created_at.to_string()),
        updated_at: Some(bp.updated_at.to_string()),
    }
}

pub(super) fn blueprint_version_summary(v: &blueprint_version::Model) -> BlueprintVersionSummary {
    BlueprintVersionSummary {
        id: v.id,
        project_id: v.project_id,
        version: v.version.clone(),
        change_description: v.change_description.clone(),
        blueprint_data: v.blueprint_data.clone(),
        world_setting_snapshot: v.world_setting_snapshot.clone(),
        characters_snapshot: v.characters_snapshot.clone(),
        relationships_snapshot: v.relationships_snapshot.clone(),
        synopsis_snapshot: v.synopsis_snapshot.clone(),
        created_at: Some(v.created_at.to_string()),
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
