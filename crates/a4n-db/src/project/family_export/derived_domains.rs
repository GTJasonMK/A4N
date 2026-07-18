use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};

project_family_export_load_by_ids!(
    project_family_export_galgame_scripts,
    galgame_script::Entity,
    galgame_script::Column::ProjectId,
    galgame_script::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_galgame_scenes,
    galgame_scene::Entity,
    galgame_scene::Column::ScriptId,
    galgame_scene::Column::SceneIndex,
    galgame_scene::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_galgame_lines,
    galgame_line::Entity,
    galgame_line::Column::SceneId,
    galgame_line::Column::LineIndex,
    galgame_line::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_galgame_choices,
    galgame_choice::Entity,
    galgame_choice::Column::LineId,
    galgame_choice::Column::OptionIndex,
    galgame_choice::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_video_segments,
    video_segment::Entity,
    video_segment::Column::ProjectId,
    video_segment::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_assembly_timelines,
    assembly_timeline::Entity,
    assembly_timeline::Column::ProjectId,
    assembly_timeline::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_assembly_clips,
    assembly_clip::Entity,
    assembly_clip::Column::TimelineId,
    assembly_clip::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_source_sync_runs,
    source_sync_run::Entity,
    source_sync_run::Column::DerivedProjectId,
    source_sync_run::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_source_sync_conflicts,
    source_sync_conflict::Entity,
    source_sync_conflict::Column::SyncRunId,
    source_sync_conflict::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_source_sync_decisions,
    source_sync_decision::Entity,
    source_sync_decision::Column::ConflictId,
    source_sync_decision::Column::Id,
);
