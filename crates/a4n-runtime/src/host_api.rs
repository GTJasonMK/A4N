//! Host 可调用的 runtime API 白名单。
//!
//! 这里必须逐项导出，避免 `actions` 新增模块时被自动扩大为公共 API。

pub use crate::actions::{
    admin_records, agent_runtime, auth_records, blueprint_records, canvas_workflow,
    chapter_content, chapter_generation, chapter_scene, chapter_visual_plan_records, character_ai,
    character_conflict, character_implicit_review, character_records, config_records,
    embedding_model, export_files, foreshadowing_records, galgame_records, generation_analytics,
    generation_tasks, image_generation, image_records, import_analysis, import_file,
    import_reverse, import_review, inspiration_records, logging, manga_pipeline, manga_records,
    oauth, object_profile_records, optimize, outline_records, package, portrait_generation,
    portrait_records, profile_materials, project_collaboration, project_duplication,
    project_records, project_status, project_transfer, prompt_records, rag_search,
    scene_profile_records, scene_profile_sync, settings_all, shooting_script_records, source_sync,
    storage_paths, task_records, theme, video_generation, video_prompts, video_records,
};

pub mod chapter_analysis {
    pub use crate::workflow::{
        ChapterAnalysisProgress, generate_chapter_analysis_with_progress,
        preflight_generate_chapter_analysis, preflight_resume_chapter_analysis_sync,
        preview_chapter_analysis_request, preview_resume_chapter_analysis_sync_request,
        resume_chapter_analysis_sync_with_progress,
    };
}

pub mod rag_ingest {
    pub use crate::rag_ingest::{
        ClientRagIngestUploadInput, RagIngestOptions, RagIngestProgress, RagIngestTaskScope,
        rag_ingest_all, rag_ingest_task_scope, rag_prepare_ingest_all,
        rag_upload_ingest_embeddings,
    };
}

pub mod runtime_config {
    pub use crate::runtime_config::{
        RuntimeConfigSnapshot, current_snapshot, request_scope, scope, snapshot_from_args,
    };
}
