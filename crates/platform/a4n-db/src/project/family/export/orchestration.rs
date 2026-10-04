use super::{
    derived_domains::*,
    generation_media::*,
    snapshots::*,
    text_core::project_family_export_text_core_rows,
    types::{ProjectFamilyExportError, ProjectFamilyExportRows, ProjectFamilyTextCoreRows},
};
use a4n_domain::source_text_hash;
use sea_orm::ConnectionTrait;
use std::collections::{HashMap, HashSet};

pub async fn project_family_export_rows<C>(
    db: &C,
    root_project_id: i64,
    derived_project_ids: &[i64],
    derived_source_snapshot_ids: &[i64],
) -> Result<ProjectFamilyExportRows, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let root_project_ids = [root_project_id];
    let family_project_ids = std::iter::once(root_project_id)
        .chain(derived_project_ids.iter().copied())
        .collect::<Vec<_>>();
    let (
        text_core,
        source_sync_runs,
        galgame_snapshot_ids,
        authoring_documents,
        authoring_receipts,
        chapter_visual_plans,
        chapter_shooting_scripts,
    ) = futures::join!(
        project_family_export_text_core_rows(db, root_project_id, &family_project_ids),
        project_family_export_source_sync_runs(db, derived_project_ids),
        project_family_export_galgame_snapshot_ids(db, derived_project_ids),
        crate::media_authoring::authoring_documents_for_projects(db, derived_project_ids),
        crate::media_authoring::authoring_receipts_for_projects(db, &family_project_ids),
        project_family_export_chapter_visual_plans(db, &family_project_ids),
        project_family_export_chapter_shooting_scripts(db, &family_project_ids),
    );
    let text_core: ProjectFamilyTextCoreRows = super::super::transfer::transfer_record(text_core?)?;
    let source_sync_runs = source_sync_runs?;
    let authoring_documents = authoring_documents?;
    let authoring_receipts = authoring_receipts?;
    let chapter_visual_plans = chapter_visual_plans?;
    let chapter_shooting_scripts = chapter_shooting_scripts?;
    let mut authoring_snapshot_ids = Vec::new();
    for record in &authoring_documents {
        authoring_snapshot_ids.push(record.source_snapshot_id);
        let inputs: a4n_domain::media_authoring::AuthoringDocumentInputs =
            serde_json::from_value(record.inputs_json.clone()).map_err(|error| {
                ProjectFamilyExportError::InvalidArgument(format!(
                    "创作文档 {} 输入无效: {error}",
                    record.id
                ))
            })?;
        authoring_snapshot_ids.extend(
            inputs
                .sources
                .iter()
                .map(|source| source.source_snapshot_id),
        );
        if let Some(reference) = inputs.video_reference {
            authoring_snapshot_ids.push(reference.source.source_snapshot_id);
        }
    }
    let source_snapshot_ids = derived_source_snapshot_ids
        .iter()
        .copied()
        .chain(galgame_snapshot_ids?)
        .chain(authoring_snapshot_ids)
        .chain(
            chapter_visual_plans
                .iter()
                .filter_map(|record| record.source_snapshot_id),
        )
        .chain(
            chapter_shooting_scripts
                .iter()
                .filter_map(|record| record.source_snapshot_id),
        )
        .collect::<Vec<_>>();
    let snapshot_ids =
        collect_project_family_export_snapshot_ids(&source_snapshot_ids, &source_sync_runs);
    let snapshot_id_set = snapshot_ids.iter().copied().collect::<HashSet<_>>();
    let (
        source_snapshots,
        source_snapshot_chapters,
        source_snapshot_outlines,
        source_snapshot_profiles,
        text_anchors,
    ) = futures::join!(
        project_family_export_source_snapshots(db, &snapshot_ids),
        project_family_export_source_snapshot_chapters(db, &snapshot_ids),
        project_family_export_source_snapshot_outlines(db, &snapshot_ids),
        project_family_export_source_snapshot_profiles(db, &snapshot_ids),
        project_family_export_text_anchors(db, root_project_id, &snapshot_ids),
    );
    let source_snapshots = source_snapshots?;
    let source_snapshot_chapters = source_snapshot_chapters?;
    let source_snapshot_outlines = source_snapshot_outlines?;
    let source_snapshot_profiles = source_snapshot_profiles?;
    let text_anchors = text_anchors?;
    let (
        profile_evidence,
        character_portraits,
        generated_images,
        chapter_manga_prompts,
        chapter_video_prompts,
        generated_videos,
    ) = futures::join!(
        project_family_export_profile_evidence(db, &text_anchors),
        project_family_export_character_portraits(db, &root_project_ids),
        project_family_export_generated_images(db, &family_project_ids),
        project_family_export_chapter_manga_prompts(db, &family_project_ids),
        project_family_export_chapter_video_prompts(db, &family_project_ids),
        project_family_export_generated_videos(db, &family_project_ids),
    );
    let profile_evidence = profile_evidence?;
    let mut chapter_visual_plans = chapter_visual_plans;
    let chapter_content_by_id = text_core.chapters.iter().fold(
        HashMap::<i64, &str>::new(),
        |mut chapters_by_id, chapter| {
            chapters_by_id
                .entry(chapter.id)
                .or_insert(chapter.content.as_str());
            chapters_by_id
        },
    );
    let mut chapter_hash_by_id = HashMap::<i64, String>::new();
    chapter_visual_plans.retain(|plan| match plan.source_snapshot_id {
        Some(snapshot_id) => snapshot_id_set.contains(&snapshot_id),
        None => chapter_content_by_id
            .get(&plan.source_chapter_id)
            .is_some_and(|content| {
                let hash = chapter_hash_by_id
                    .entry(plan.source_chapter_id)
                    .or_insert_with(|| source_text_hash(content));
                hash == &plan.source_content_sha256
            }),
    });
    let visual_plan_ids = chapter_visual_plans
        .iter()
        .map(|plan| plan.id)
        .collect::<HashSet<_>>();
    let mut chapter_shooting_scripts = chapter_shooting_scripts;
    chapter_shooting_scripts.retain(|script| {
        script
            .visual_plan_id
            .is_none_or(|id| visual_plan_ids.contains(&id))
    });
    let character_portraits = character_portraits?;
    let generated_images = generated_images?;
    let chapter_manga_prompts = chapter_manga_prompts?;
    let chapter_video_prompts = chapter_video_prompts?;
    let generated_videos = generated_videos?;

    let (generation_specs, media_assets) = futures::join!(
        project_family_export_generation_specs(db, derived_project_ids),
        project_family_export_media_assets(db, derived_project_ids),
    );
    let generation_specs = generation_specs?;
    let media_assets = media_assets?;
    let generation_spec_ids = generation_specs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let media_asset_ids = media_assets
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let (
        generation_spec_text_anchors,
        generation_spec_entity_refs,
        prompt_artifacts,
        generation_spec_media_refs,
        media_provenance,
        media_bindings,
    ) = futures::join!(
        project_family_export_generation_spec_text_anchors(db, &generation_spec_ids),
        project_family_export_generation_spec_entity_refs(db, &generation_spec_ids),
        project_family_export_prompt_artifacts(db, &generation_spec_ids),
        project_family_export_generation_spec_media_refs(
            db,
            &generation_spec_ids,
            &media_asset_ids,
        ),
        project_family_export_media_provenance(db, &media_asset_ids),
        project_family_export_media_bindings(db, &media_asset_ids),
    );
    let generation_spec_text_anchors = generation_spec_text_anchors?;
    let generation_spec_entity_refs = generation_spec_entity_refs?;
    let prompt_artifacts = prompt_artifacts?;
    let generation_spec_media_refs = generation_spec_media_refs?;
    let media_provenance = media_provenance?;
    let media_bindings = media_bindings?;

    let sync_run_ids = source_sync_runs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let (galgame_result, video_result, source_sync_result) = futures::join!(
        async {
            let galgame_scripts =
                project_family_export_galgame_scripts(db, derived_project_ids).await?;
            let galgame_script_ids = galgame_scripts
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>();
            let galgame_scenes =
                project_family_export_galgame_scenes(db, &galgame_script_ids).await?;
            let galgame_scene_ids = galgame_scenes
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>();
            let galgame_lines = project_family_export_galgame_lines(db, &galgame_scene_ids).await?;
            let galgame_line_ids = galgame_lines
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>();
            let galgame_choices =
                project_family_export_galgame_choices(db, &galgame_line_ids).await?;
            Ok::<_, ProjectFamilyExportError>((
                galgame_scripts,
                galgame_scenes,
                galgame_lines,
                galgame_choices,
            ))
        },
        async {
            let (video_segments, assembly_timelines) = futures::join!(
                project_family_export_video_segments(db, derived_project_ids),
                project_family_export_assembly_timelines(db, derived_project_ids),
            );
            let video_segments = video_segments?;
            let assembly_timelines = assembly_timelines?;
            let timeline_ids = assembly_timelines
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>();
            let assembly_clips = project_family_export_assembly_clips(db, &timeline_ids).await?;
            Ok::<_, ProjectFamilyExportError>((video_segments, assembly_timelines, assembly_clips))
        },
        async {
            let source_sync_conflicts =
                project_family_export_source_sync_conflicts(db, &sync_run_ids).await?;
            let conflict_ids = source_sync_conflicts
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>();
            let source_sync_decisions =
                project_family_export_source_sync_decisions(db, &conflict_ids).await?;
            Ok::<_, ProjectFamilyExportError>((source_sync_conflicts, source_sync_decisions))
        },
    );
    let (galgame_scripts, galgame_scenes, galgame_lines, galgame_choices) = galgame_result?;
    let (video_segments, assembly_timelines, assembly_clips) = video_result?;
    let (source_sync_conflicts, source_sync_decisions) = source_sync_result?;

    Ok(ProjectFamilyExportRows {
        snapshot_ids,
        text_core,
        source_snapshots: super::super::transfer::transfer_records(source_snapshots)?,
        source_snapshot_chapters: super::super::transfer::transfer_records(
            source_snapshot_chapters,
        )?,
        source_snapshot_outlines: super::super::transfer::transfer_records(
            source_snapshot_outlines,
        )?,
        source_snapshot_profiles: super::super::transfer::transfer_records(
            source_snapshot_profiles,
        )?,
        text_anchors: super::super::transfer::transfer_records(text_anchors)?,
        profile_evidence: super::super::transfer::transfer_records(profile_evidence)?,
        chapter_visual_plans: super::super::transfer::transfer_records(chapter_visual_plans)?,
        chapter_shooting_scripts: super::super::transfer::transfer_records(
            chapter_shooting_scripts,
        )?,
        authoring_documents: super::super::transfer::transfer_records(authoring_documents)?,
        authoring_receipts: super::super::transfer::transfer_records(authoring_receipts)?,
        character_portraits: super::super::transfer::transfer_records(character_portraits)?,
        generated_images: super::super::transfer::transfer_records(generated_images)?,
        chapter_manga_prompts: super::super::transfer::transfer_records(chapter_manga_prompts)?,
        chapter_video_prompts,
        generated_videos: super::super::transfer::transfer_records(generated_videos)?,
        generation_specs: super::super::transfer::transfer_records(generation_specs)?,
        generation_spec_text_anchors: super::super::transfer::transfer_records(
            generation_spec_text_anchors,
        )?,
        generation_spec_entity_refs: super::super::transfer::transfer_records(
            generation_spec_entity_refs,
        )?,
        generation_spec_media_refs: super::super::transfer::transfer_records(
            generation_spec_media_refs,
        )?,
        prompt_artifacts: super::super::transfer::transfer_records(prompt_artifacts)?,
        media_assets: super::super::transfer::transfer_records(media_assets)?,
        media_provenance: super::super::transfer::transfer_records(media_provenance)?,
        media_bindings: super::super::transfer::transfer_records(media_bindings)?,
        galgame_scripts: super::super::transfer::transfer_records(galgame_scripts)?,
        galgame_scenes: super::super::transfer::transfer_records(galgame_scenes)?,
        galgame_lines: super::super::transfer::transfer_records(galgame_lines)?,
        galgame_choices: super::super::transfer::transfer_records(galgame_choices)?,
        video_segments: super::super::transfer::transfer_records(video_segments)?,
        assembly_timelines: super::super::transfer::transfer_records(assembly_timelines)?,
        assembly_clips: super::super::transfer::transfer_records(assembly_clips)?,
        source_sync_runs: super::super::transfer::transfer_records(source_sync_runs)?,
        source_sync_conflicts: super::super::transfer::transfer_records(source_sync_conflicts)?,
        source_sync_decisions: super::super::transfer::transfer_records(source_sync_decisions)?,
    })
}
