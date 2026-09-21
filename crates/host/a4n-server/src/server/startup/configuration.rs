use super::super::*;
use super::state::StartupConfig;

pub(crate) fn load() -> anyhow::Result<StartupConfig> {
    let control_addr = control_addr()?;
    let agent_worker_addr = agent_worker_addr()?;
    let worker_token = worker_token()?;
    let novel_planning_worker_addr = novel_planning_worker_addr()?;
    let novel_planning_worker_token = worker_token_for_env(NOVEL_PLANNING_WORKER_TOKEN_ENV)?;
    let artifact_media_worker_addr = artifact_media_worker_addr()?;
    let artifact_media_worker_token = worker_token_for_env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV)?;
    let outline_worker_addr = outline_worker_addr()?;
    let outline_worker_token = worker_token_for_env(OUTLINE_WORKER_TOKEN_ENV)?;
    let blueprint_worker_addr = blueprint_worker_addr()?;
    let blueprint_worker_token = worker_token_for_env(BLUEPRINT_WORKER_TOKEN_ENV)?;
    let asset_graph_worker_addr = asset_graph_worker_addr()?;
    let asset_graph_worker_token = worker_token_for_env(ASSET_GRAPH_WORKER_TOKEN_ENV)?;
    let foreshadowing_worker_addr = foreshadowing_worker_addr()?;
    let foreshadowing_worker_token = worker_token_for_env(FORESHADOWING_WORKER_TOKEN_ENV)?;
    let object_profile_worker_addr = object_profile_worker_addr()?;
    let object_profile_worker_token = worker_token_for_env(OBJECT_PROFILE_WORKER_TOKEN_ENV)?;
    let scene_profile_worker_addr = scene_profile_worker_addr()?;
    let scene_profile_worker_token = worker_token_for_env(SCENE_PROFILE_WORKER_TOKEN_ENV)?;
    let profile_material_worker_addr = profile_material_worker_addr()?;
    let profile_material_worker_token = worker_token_for_env(PROFILE_MATERIAL_WORKER_TOKEN_ENV)?;
    let project_status_worker_addr = project_status_worker_addr()?;
    let project_status_worker_token = worker_token_for_env(PROJECT_STATUS_WORKER_TOKEN_ENV)?;
    let portrait_worker_addr = portrait_worker_addr()?;
    let portrait_worker_token = worker_token_for_env(PORTRAIT_WORKER_TOKEN_ENV)?;
    let source_snapshot_worker_addr = source_snapshot_worker_addr()?;
    let source_snapshot_worker_token = worker_token_for_env(SOURCE_SNAPSHOT_WORKER_TOKEN_ENV)?;
    let theme_read_worker_addr = theme_read_worker_addr()?;
    let theme_read_worker_token = worker_token_for_env(THEME_READ_WORKER_TOKEN_ENV)?;
    let project_read_worker_addr = project_read_worker_addr()?;
    let project_read_worker_token = worker_token_for_env(PROJECT_READ_WORKER_TOKEN_ENV)?;
    let import_review_read_worker_addr = import_review_read_worker_addr()?;
    let import_review_read_worker_token =
        worker_token_for_env(IMPORT_REVIEW_READ_WORKER_TOKEN_ENV)?;
    let inspiration_history_worker_addr = inspiration_history_worker_addr()?;
    let inspiration_history_worker_token =
        worker_token_for_env(INSPIRATION_HISTORY_WORKER_TOKEN_ENV)?;
    let project_access_worker_addr = project_access_worker_addr()?;
    let project_access_worker_token = worker_token_for_env(PROJECT_ACCESS_WORKER_TOKEN_ENV)?;
    let project_collaboration_worker_addr = project_collaboration_worker_addr()?;
    let project_collaboration_worker_token =
        worker_token_for_env(PROJECT_COLLABORATION_WORKER_TOKEN_ENV)?;
    let project_mutation_worker_addr = project_mutation_worker_addr()?;
    let project_mutation_worker_token = worker_token_for_env(PROJECT_MUTATION_WORKER_TOKEN_ENV)?;
    let chapter_scene_worker_addr = chapter_scene_worker_addr()?;
    let chapter_scene_worker_token = worker_token_for_env(CHAPTER_SCENE_WORKER_TOKEN_ENV)?;
    let chapter_content_worker_addr = chapter_content_worker_addr()?;
    let chapter_content_worker_token = worker_token_for_env(CHAPTER_CONTENT_WORKER_TOKEN_ENV)?;
    let workflow_read_worker_addr = workflow_read_worker_addr()?;
    let workflow_read_worker_token = worker_token_for_env(WORKFLOW_READ_WORKER_TOKEN_ENV)?;
    let config_read_worker_addr = config_read_worker_addr()?;
    let config_read_worker_token = worker_token_for_env(CONFIG_READ_WORKER_TOKEN_ENV)?;
    let character_mutation_worker_addr = character_mutation_worker_addr()?;
    let character_mutation_worker_token =
        worker_token_for_env(CHARACTER_MUTATION_WORKER_TOKEN_ENV)?;
    let video_record_worker_addr = video_record_worker_addr()?;
    let video_record_worker_token = worker_token_for_env(VIDEO_RECORD_WORKER_TOKEN_ENV)?;
    let manga_record_worker_addr = manga_record_worker_addr()?;
    let manga_record_worker_token = worker_token_for_env(MANGA_RECORD_WORKER_TOKEN_ENV)?;
    let control_binary = control_binary()?;
    let control_fingerprint = binary_fingerprint(&control_binary)?;
    let worker_binary = agent_worker_binary()?;
    let worker_fingerprint = binary_fingerprint(&worker_binary)?;
    let edge_binary = edge_binary()?;
    let edge_fingerprint = binary_fingerprint(&edge_binary)?;
    let novel_planning_worker_binary = novel_planning_worker_binary()?;
    let novel_planning_worker_fingerprint = binary_fingerprint(&novel_planning_worker_binary)?;
    let artifact_media_worker_binary = artifact_media_worker_binary()?;
    let artifact_media_worker_fingerprint = binary_fingerprint(&artifact_media_worker_binary)?;
    let outline_worker_binary = outline_worker_binary()?;
    let outline_worker_fingerprint = binary_fingerprint(&outline_worker_binary)?;
    let blueprint_worker_binary = blueprint_worker_binary()?;
    let blueprint_worker_fingerprint = binary_fingerprint(&blueprint_worker_binary)?;
    let asset_graph_worker_binary = asset_graph_worker_binary()?;
    let asset_graph_worker_fingerprint = binary_fingerprint(&asset_graph_worker_binary)?;
    let foreshadowing_worker_binary = foreshadowing_worker_binary()?;
    let foreshadowing_worker_fingerprint = binary_fingerprint(&foreshadowing_worker_binary)?;
    let object_profile_worker_binary = object_profile_worker_binary()?;
    let object_profile_worker_fingerprint = binary_fingerprint(&object_profile_worker_binary)?;
    let scene_profile_worker_binary = scene_profile_worker_binary()?;
    let scene_profile_worker_fingerprint = binary_fingerprint(&scene_profile_worker_binary)?;
    let profile_material_worker_binary = profile_material_worker_binary()?;
    let profile_material_worker_fingerprint = binary_fingerprint(&profile_material_worker_binary)?;
    let project_status_worker_binary = project_status_worker_binary()?;
    let project_status_worker_fingerprint = binary_fingerprint(&project_status_worker_binary)?;
    let portrait_worker_binary = portrait_worker_binary()?;
    let portrait_worker_fingerprint = binary_fingerprint(&portrait_worker_binary)?;
    let source_snapshot_worker_binary = source_snapshot_worker_binary()?;
    let source_snapshot_worker_fingerprint = binary_fingerprint(&source_snapshot_worker_binary)?;
    let theme_read_worker_binary = theme_read_worker_binary()?;
    let import_review_read_worker_binary = import_review_read_worker_binary()?;
    let inspiration_history_worker_binary = inspiration_history_worker_binary()?;
    let project_access_worker_binary = project_access_worker_binary()?;
    let project_collaboration_worker_binary = project_collaboration_worker_binary()?;
    let project_mutation_worker_binary = project_mutation_worker_binary()?;
    let chapter_scene_worker_binary = chapter_scene_worker_binary()?;
    let chapter_content_worker_binary = chapter_content_worker_binary()?;
    let workflow_read_worker_binary = workflow_read_worker_binary()?;
    let config_read_worker_binary = config_read_worker_binary()?;
    let character_mutation_worker_binary = character_mutation_worker_binary()?;
    let video_record_worker_binary = video_record_worker_binary()?;
    let manga_record_worker_binary = manga_record_worker_binary()?;
    let reload_marker = reload_marker_path();
    let reload_marker_fingerprint = optional_binary_fingerprint(reload_marker.as_deref())?;
    Ok(StartupConfig {
        control_addr,
        agent_worker_addr,
        novel_planning_worker_addr,
        artifact_media_worker_addr,
        outline_worker_addr,
        blueprint_worker_addr,
        asset_graph_worker_addr,
        foreshadowing_worker_addr,
        object_profile_worker_addr,
        scene_profile_worker_addr,
        profile_material_worker_addr,
        project_status_worker_addr,
        portrait_worker_addr,
        source_snapshot_worker_addr,
        theme_read_worker_addr,
        project_read_worker_addr,
        import_review_read_worker_addr,
        inspiration_history_worker_addr,
        project_access_worker_addr,
        project_collaboration_worker_addr,
        project_mutation_worker_addr,
        chapter_scene_worker_addr,
        chapter_content_worker_addr,
        workflow_read_worker_addr,
        config_read_worker_addr,
        character_mutation_worker_addr,
        video_record_worker_addr,
        manga_record_worker_addr,
        worker_token,
        novel_planning_worker_token,
        artifact_media_worker_token,
        outline_worker_token,
        blueprint_worker_token,
        asset_graph_worker_token,
        foreshadowing_worker_token,
        object_profile_worker_token,
        scene_profile_worker_token,
        profile_material_worker_token,
        project_status_worker_token,
        portrait_worker_token,
        source_snapshot_worker_token,
        theme_read_worker_token,
        project_read_worker_token,
        import_review_read_worker_token,
        inspiration_history_worker_token,
        project_access_worker_token,
        project_collaboration_worker_token,
        project_mutation_worker_token,
        chapter_scene_worker_token,
        chapter_content_worker_token,
        workflow_read_worker_token,
        config_read_worker_token,
        character_mutation_worker_token,
        video_record_worker_token,
        manga_record_worker_token,
        control_binary,
        worker_binary,
        edge_binary,
        novel_planning_worker_binary,
        artifact_media_worker_binary,
        outline_worker_binary,
        blueprint_worker_binary,
        asset_graph_worker_binary,
        foreshadowing_worker_binary,
        object_profile_worker_binary,
        scene_profile_worker_binary,
        profile_material_worker_binary,
        project_status_worker_binary,
        portrait_worker_binary,
        source_snapshot_worker_binary,
        theme_read_worker_binary,
        import_review_read_worker_binary,
        inspiration_history_worker_binary,
        project_access_worker_binary,
        project_collaboration_worker_binary,
        project_mutation_worker_binary,
        chapter_scene_worker_binary,
        chapter_content_worker_binary,
        workflow_read_worker_binary,
        config_read_worker_binary,
        character_mutation_worker_binary,
        video_record_worker_binary,
        manga_record_worker_binary,
        reload_marker,
        control_fingerprint,
        worker_fingerprint,
        edge_fingerprint,
        novel_planning_worker_fingerprint,
        artifact_media_worker_fingerprint,
        outline_worker_fingerprint,
        blueprint_worker_fingerprint,
        asset_graph_worker_fingerprint,
        foreshadowing_worker_fingerprint,
        object_profile_worker_fingerprint,
        scene_profile_worker_fingerprint,
        profile_material_worker_fingerprint,
        project_status_worker_fingerprint,
        portrait_worker_fingerprint,
        source_snapshot_worker_fingerprint,
        reload_marker_fingerprint,
    })
}
