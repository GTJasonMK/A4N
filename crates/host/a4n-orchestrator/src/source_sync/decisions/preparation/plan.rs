use std::collections::BTreeSet;

use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::media_authoring::{AdaptationTarget, AuthoringDocumentInputs};
use a4n_error::AppError;

use super::spans::galgame_regeneration_source_spans;
use crate::{
    OrchestratorContext,
    galgame::{
        GalgameChapterAdaptation, GalgamePipelinePromptChapter, GalgamePipelinePromptContext,
        GalgamePipelineSourceSpan, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
        GalgameSpeakerProfile, galgame_adaptation_projection,
    },
    media_authoring::{GenerateAdaptation, authoring_error, authoring_source},
    source_sync::{
        contracts::{GalgameSourceSyncExecutionContext, RegenerateGalgameScenePayload},
        decisions::validation::validate_galgame_scene_regeneration_plan_for_apply,
        source_sync_db_error,
    },
};

pub(super) async fn build_prepared_galgame_scene_regeneration_plan(
    service: &OrchestratorContext,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    payload: &RegenerateGalgameScenePayload,
    galgame_execution: GalgameSourceSyncExecutionContext<'_>,
    speaker_profiles: &[GalgameSpeakerProfile],
) -> Result<GalgameSceneRegenerationPlan, AppError> {
    let source = a4n_db::source_sync::galgame_scene_regeneration_source_record(
        service.database.connection(),
        run,
        conflict,
        payload.galgame_scene_id,
    )
    .await
    .map_err(source_sync_db_error)?;
    let unit_id = source
        .scene
        .adaptation_unit_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Galgame 场 {} 缺少本地改编单元身份",
                source.scene.id
            ))
        })?;
    let previous_plan_id = source.scene.adaptation_plan_id.ok_or_else(|| {
        AppError::InvalidArgument(format!("Galgame 场 {} 缺少本地改编计划", source.scene.id))
    })?;
    let mut sources = a4n_db::project::resolve_snapshot_sources(
        service.database.connection(),
        run.derived_project_id,
        run.root_project_id,
        run.to_snapshot_id,
        &[source.new_snapshot_chapter.source_chapter_id],
    )
    .await
    .map_err(crate::shooting_script::errors::resolve_source_error)?;
    let target = sources
        .pop()
        .ok_or_else(|| AppError::InvalidArgument("目标章节来源不存在".into()))?;

    // Adopt per source lineage: updating this scene must not replace the plan
    // still used by scenes that deliberately keep the previous source snapshot.
    if service
        .load_current_adaptation(&target, AdaptationTarget::Galgame)
        .await?
        .is_none()
    {
        let previous = a4n_db::media_authoring::authoring_document(
            service.database.connection(),
            run.derived_project_id,
            previous_plan_id,
        )
        .await
        .map_err(authoring_error)?;
        service.generate_adaptation_plan(GenerateAdaptation {
            source: &target,
            target: AdaptationTarget::Galgame,
            inputs: AuthoringDocumentInputs {
                dependencies: Vec::new(),
                sources: vec![authoring_source(&target)?],
                video_reference: previous.inputs.video_reference,
            },
            record_id: None,
            expected_revision: None,
            previous_plan_id: Some(previous_plan_id),
            instruction: "依据新的原文更新本章改编计划。保留原有单元与节拍 ID，未受原文变化影响的内容保持原样；明确删除的单元不应伪造替代身份。",
            workflow_run_id: galgame_execution.workflow_run_id,
            workflow: "source_sync",
            renderer: galgame_execution.prompt_renderer,
            stream_reporter: None,
        }).await?;
    }
    let adaptation = service
        .ensure_adaptation_plan(
            &target,
            AdaptationTarget::Galgame,
            galgame_execution.workflow_run_id,
            "source_sync",
            galgame_execution.prompt_renderer,
        )
        .await?;
    let source_spans =
        galgame_regeneration_source_spans(&source.new_snapshot_chapter, &source.anchors);
    let chapter = GalgamePipelinePromptChapter {
        source_snapshot_chapter_id: source.new_snapshot_chapter.id,
        source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
        chapter_number: source.new_snapshot_chapter.chapter_number,
        title: source.new_snapshot_chapter.title.clone(),
        summary: source.new_snapshot_chapter.summary.clone(),
        content_hash: source.new_snapshot_chapter.content_hash.clone(),
        content: source.new_snapshot_chapter.content.clone(),
        source_spans: source_spans.clone(),
    };
    let projection = galgame_adaptation_projection(&GalgamePipelinePromptContext {
        project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.to_snapshot_id,
        chapters: vec![chapter],
        source_spans: source_spans
            .iter()
            .map(|span| GalgamePipelineSourceSpan {
                id: span.id.clone(),
                text_anchor_id: span.text_anchor_id,
                source_snapshot_chapter_id: span.source_snapshot_chapter_id,
                chapter_number: source.new_snapshot_chapter.chapter_number,
            })
            .collect(),
        speaker_profiles: speaker_profiles.to_vec(),
        adaptations: vec![GalgameChapterAdaptation {
            source_chapter_id: target.source_chapter_id,
            source_snapshot_chapter_id: source.new_snapshot_chapter.id,
            chapter_number: target.chapter_number,
            adaptation_plan_id: adaptation.record.id,
            adaptation_plan_revision: adaptation.record.revision,
            plan: adaptation.plan,
        }],
    })?;
    let directing_scene = projection
        .scenes
        .into_iter()
        .find(|scene| scene.unit_id == unit_id)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("目标改编计划中不存在单元 {unit_id}"),
            operation: "更新 Galgame 场景；需要先处理原文删除或单元重排决策".into(),
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
                speaker_profiles: speaker_profiles.to_vec(),
            },
            galgame_execution.prompt_renderer,
        )
        .await?;
    let anchors = source
        .anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<BTreeSet<_>>();
    validate_galgame_scene_regeneration_plan_for_apply(
        &plan,
        source.scene.scene_index,
        source.new_snapshot_chapter.id,
        &anchors,
    )?;
    Ok(plan)
}
