use a4n_domain::manga::MangaPageSlotsV1;
use a4n_domain::manga::MangaPipelineArtifactStage;
use a4n_error::AppError;

use super::super::super::adaptation_projection::manga_page_slots_source;
use super::super::json::parse_manga_page_slots;
use super::super::prompt::{MangaRenderTraceInput, manga_page_slots_user_message};
use super::super::state::MangaPipelineStage;
use super::contracts::{MangaRunContext, manga_stage_output_exists_error};

pub(super) async fn run_page_slots(
    context: &MangaRunContext<'_>,
    extraction_summary: &serde_json::Value,
) -> Result<MangaPageSlotsV1, AppError> {
    context.check_cancelled()?;
    let user_content = manga_page_slots_user_message(
        extraction_summary.clone(),
        manga_page_slots_source(&context.adaptation),
        &context.source_spans,
        &context.chapter_content,
        &context.source_markers,
    )?;
    let input_fingerprint =
        context.artifact_input_fingerprint(MangaPipelineArtifactStage::PageSlots, &user_content)?;
    let existing = context
        .service
        .manga_pipeline_artifact(
            context.project_id,
            context.chapter_id,
            MangaPipelineArtifactStage::PageSlots,
            None,
        )
        .await?;
    let explicitly_requested = context.run_stage == Some(MangaPipelineArtifactStage::PageSlots);
    if let Some(artifact) = existing {
        if artifact.stale || artifact.input_fingerprint != input_fingerprint {
            return Err(manga_stage_output_exists_error(
                MangaPipelineArtifactStage::PageSlots,
                None,
            ));
        }
        if explicitly_requested && !context.resume_from_checkpoint {
            return Err(manga_stage_output_exists_error(
                MangaPipelineArtifactStage::PageSlots,
                None,
            ));
        }
        let slots = parse_manga_page_slots(
            &artifact.payload,
            &context.adaptation,
            &context.source_spans,
            "manga_page_slots",
        )?;
        context.advance_stage(MangaPipelineStage::PageSlots).await?;
        return Ok(slots);
    }
    context.advance_stage(MangaPipelineStage::PageSlots).await?;
    let (_, slots) = context
        .service
        .render_and_validate_manga_call_with_markers(
            MangaRenderTraceInput {
                prompt_renderer: context.prompt_renderer,
                prompt_name: "manga_page_slots",
                project_id: context.project_id,
                workflow_run_id: context.workflow_run_id,
                call_key: format!("manga_pipeline:{}:page_slots", context.chapter_id),
                call_order: 6,
                restore_from_checkpoint: context.resume_from_checkpoint,
                variables: &context.variables,
                user_content,
                temperature: 0.3,
                cancel_token: context.cancel_token,
                stream_id: "raw_json".to_string(),
                stream_reporter: context.stream_reporter,
            },
            &context.source_markers,
            |value| {
                parse_manga_page_slots(
                    value,
                    &context.adaptation,
                    &context.source_spans,
                    "manga_page_slots",
                )
            },
        )
        .await?;
    context
        .persist_artifact(
            MangaPipelineArtifactStage::PageSlots,
            None,
            input_fingerprint,
            serde_json::to_value(&slots)?,
        )
        .await?;
    Ok(slots)
}
