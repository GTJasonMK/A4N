use std::collections::BTreeSet;

use a4n_db_models::models::{
    chapter::chapter_shooting_script,
    project::source_snapshot_chapter,
    source_sync::{source_sync_conflict, source_sync_run},
};
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use a4n_error::AppError;

use super::spans::galgame_regeneration_source_spans;
use crate::OrchestratorContext;
use crate::galgame::{
    GalgameChapterShootingScript, GalgamePipelinePromptChapter, GalgamePipelinePromptContext,
    GalgamePipelineSourceSpan, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    GalgameSpeakerProfile, galgame_shooting_script_projection,
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
    let visual_plan = source_sync_visual_plan_identity(&source.new_snapshot_chapter)?;
    let shooting_script = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id: run.derived_project_id,
                source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
                visual_plan_id: Some(visual_plan.id),
            },
        )
        .await?;
    let shooting_record = &shooting_script.shooting_script.record;
    ensure_source_sync_shooting_script_visual_plan(shooting_record, &visual_plan)?;
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
        speaker_profiles: speaker_profiles.to_vec(),
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
                speaker_profiles: speaker_profiles.to_vec(),
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

fn source_sync_visual_plan_identity(
    chapter: &source_snapshot_chapter::Model,
) -> Result<a4n_db::shooting_script::ChapterVisualPlanIdentity, AppError> {
    match (
        chapter.visual_plan_id,
        chapter.visual_plan_version,
        chapter.visual_plan_sha256.clone(),
    ) {
        (Some(id), Some(version), Some(sha256)) if id > 0 && version > 0 => {
            Ok(a4n_db::shooting_script::ChapterVisualPlanIdentity {
                id,
                version,
                sha256,
            })
        }
        _ => Err(AppError::NotFound(format!(
            "源同步目标快照第 {} 章缺少已确认的导演编排方案",
            chapter.chapter_number
        ))),
    }
}

fn ensure_source_sync_shooting_script_visual_plan(
    shooting_script: &chapter_shooting_script::Model,
    visual_plan: &a4n_db::shooting_script::ChapterVisualPlanIdentity,
) -> Result<(), AppError> {
    if shooting_script.visual_plan_id == Some(visual_plan.id)
        && shooting_script.visual_plan_version == Some(visual_plan.version)
        && shooting_script.visual_plan_sha256.as_deref() == Some(visual_plan.sha256.as_str())
    {
        return Ok(());
    }
    Err(AppError::InvalidState {
        current: format!(
            "source_sync_shooting_script_visual_plan_mismatch_{}",
            shooting_script.id
        ),
        operation: SourceSyncResolutionStrategy::RegenerateScene
            .as_str()
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    fn snapshot_chapter() -> source_snapshot_chapter::Model {
        source_snapshot_chapter::Model {
            id: 10,
            snapshot_id: 20,
            root_project_id: 30,
            source_chapter_id: 40,
            chapter_number: 5,
            title: Some("第五章".to_string()),
            content: "正文".to_string(),
            summary: None,
            analysis_data_json: None,
            content_hash: "a".repeat(64),
            analysis_hash: None,
            visual_plan_id: Some(50),
            visual_plan_version: Some(3),
            visual_plan_sha256: Some("b".repeat(64)),
        }
    }

    fn shooting_script() -> chapter_shooting_script::Model {
        let now = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        chapter_shooting_script::Model {
            id: 60,
            root_project_id: 30,
            source_chapter_id: 40,
            source_snapshot_id: Some(20),
            source_snapshot_chapter_id: Some(10),
            source_revision_key: "snapshot:20:chapter:10".to_string(),
            source_content_sha256: "a".repeat(64),
            visual_plan_id: Some(50),
            visual_plan_version: Some(3),
            visual_plan_sha256: Some("b".repeat(64)),
            input_fingerprint_sha256: "c".repeat(64),
            contract_version: "chapter_shooting_script_v1".to_string(),
            revision: 1,
            script_sha256: "d".repeat(64),
            script_json: json!({}),
            workflow_run_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn target_snapshot_requires_complete_confirmed_visual_plan_identity() {
        for chapter in [
            source_snapshot_chapter::Model {
                visual_plan_id: None,
                ..snapshot_chapter()
            },
            source_snapshot_chapter::Model {
                visual_plan_version: None,
                ..snapshot_chapter()
            },
            source_snapshot_chapter::Model {
                visual_plan_sha256: None,
                ..snapshot_chapter()
            },
        ] {
            let error = source_sync_visual_plan_identity(&chapter)
                .expect_err("incomplete target snapshot identity must fail");
            assert!(error.to_string().contains("缺少已确认的导演编排方案"));
        }
    }

    #[test]
    fn shooting_script_must_match_snapshot_visual_plan_id_version_and_hash() {
        let identity = source_sync_visual_plan_identity(&snapshot_chapter()).unwrap();
        ensure_source_sync_shooting_script_visual_plan(&shooting_script(), &identity).unwrap();

        for script in [
            chapter_shooting_script::Model {
                visual_plan_id: Some(51),
                ..shooting_script()
            },
            chapter_shooting_script::Model {
                visual_plan_version: Some(4),
                ..shooting_script()
            },
            chapter_shooting_script::Model {
                visual_plan_sha256: Some("e".repeat(64)),
                ..shooting_script()
            },
        ] {
            let error = ensure_source_sync_shooting_script_visual_plan(&script, &identity)
                .expect_err("any visual plan identity mismatch must fail");
            assert!(
                error
                    .to_string()
                    .contains("source_sync_shooting_script_visual_plan_mismatch_60")
            );
        }
    }
}
