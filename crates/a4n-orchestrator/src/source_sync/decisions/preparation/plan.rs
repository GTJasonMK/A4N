use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::SourceSyncResolutionStrategy;

use super::spans::galgame_regeneration_source_spans;
use crate::OrchestratorContext;
use crate::galgame::{
    GalgameChapterShootingScript, GalgamePipelinePromptChapter, GalgamePipelinePromptContext,
    GalgamePipelineSourceSpan, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    galgame_shooting_script_projection,
};
use crate::source_sync::contracts::{
    GalgameSourceSyncExecutionContext, RegenerateGalgameScenePayload,
};
use crate::source_sync::decisions::validation::validate_galgame_scene_regeneration_plan_for_apply;
use crate::source_sync::source_sync_db_error;

pub(super) async fn build_prepared_galgame_scene_regeneration_plan(
    service: &OrchestratorContext,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    payload: &RegenerateGalgameScenePayload,
    galgame_execution: GalgameSourceSyncExecutionContext<'_>,
) -> Result<GalgameSceneRegenerationPlan, AppError> {
    let source = a4n_db::source_sync::galgame_scene_regeneration_source_record(
        service.database.connection(),
        run,
        conflict,
        payload.galgame_scene_id,
    )
    .await
    .map_err(source_sync_db_error)?;
    let speaker_profiles = service
        .speaker_profiles_for_root_project(run.root_project_id)
        .await?;
    let source_spans =
        galgame_regeneration_source_spans(&source.new_snapshot_chapter, &source.anchors);
    let prompt_chapter = GalgamePipelinePromptChapter {
        source_snapshot_chapter_id: source.new_snapshot_chapter.id,
        source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
        chapter_number: source.new_snapshot_chapter.chapter_number,
        title: source.new_snapshot_chapter.title.clone(),
        summary: source.new_snapshot_chapter.summary.clone(),
        content_hash: source.new_snapshot_chapter.content_hash.clone(),
        content: source.new_snapshot_chapter.content.clone(),
        source_spans: source_spans.clone(),
    };
    let shooting_script = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id: run.derived_project_id,
                source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
            },
        )
        .await?;
    let projection = galgame_shooting_script_projection(&GalgamePipelinePromptContext {
        project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.to_snapshot_id,
        chapters: vec![prompt_chapter],
        source_spans: source_spans
            .iter()
            .map(|span| GalgamePipelineSourceSpan {
                id: span.id.clone(),
                text_anchor_id: span.text_anchor_id,
                source_snapshot_chapter_id: span.source_snapshot_chapter_id,
            })
            .collect(),
        speaker_profiles: speaker_profiles.clone(),
        shooting_scripts: vec![GalgameChapterShootingScript {
            source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
            source_snapshot_chapter_id: source.new_snapshot_chapter.id,
            chapter_number: source.new_snapshot_chapter.chapter_number,
            shooting_script_id: shooting_script.shooting_script.record.id,
            shooting_script_revision: shooting_script.shooting_script.record.revision,
            script: shooting_script.shooting_script.script,
        }],
    })?;
    let target_span_ids = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<BTreeSet<_>>();
    let directing_scene = projection
        .scenes
        .into_iter()
        .map(|scene| {
            let overlap = scene
                .source_span_ids
                .iter()
                .filter(|id| target_span_ids.contains(id.as_str()))
                .count();
            (overlap, scene)
        })
        .max_by_key(|(overlap, _)| *overlap)
        .filter(|(overlap, _)| *overlap > 0)
        .map(|(_, scene)| scene)
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "galgame_directing_scene_not_found_for_source_sync_scene_{}",
                source.scene.id
            ),
            operation: SourceSyncResolutionStrategy::RegenerateScene
                .as_str()
                .to_string(),
        })?;
    let plan = service
        .build_source_sync_scene_regeneration(
            GalgameSceneRegenerationInput {
                project_id: run.derived_project_id,
                root_project_id: run.root_project_id,
                workflow_run_id: galgame_execution.workflow_run_id,
                source_snapshot_id: run.to_snapshot_id,
                source_snapshot_chapter_id: source.new_snapshot_chapter.id,
                source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
                chapter_number: source.new_snapshot_chapter.chapter_number,
                chapter_title: source.new_snapshot_chapter.title.clone(),
                chapter_summary: source.new_snapshot_chapter.summary.clone(),
                chapter_content_hash: source.new_snapshot_chapter.content_hash.clone(),
                chapter_content: source.new_snapshot_chapter.content.clone(),
                scene_id: source.scene.id,
                scene_index: source.scene.scene_index,
                scene_title: source.scene.title.clone(),
                location: source.scene.location.clone(),
                time_of_day: source.scene.time_of_day.clone(),
                background_note: source.scene.background_note.clone(),
                directing_scene,
                source_spans,
                speaker_profiles,
            },
            galgame_execution.prompt_renderer,
        )
        .await?;
    let new_anchor_ids = source
        .anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<BTreeSet<_>>();
    validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        source.scene.scene_index,
        source.new_snapshot_chapter.id,
        &new_anchor_ids,
    )?;
    Ok(plan)
}
