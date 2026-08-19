mod analysis;
mod content;
mod generation;
mod scenes;
mod shooting_script;

pub(in crate::routes::commands) use analysis::{
    generate_chapter_analysis, preview_chapter_analysis_request,
    preview_resume_chapter_analysis_sync_request, resume_chapter_analysis_sync,
};
pub(in crate::routes::commands) use content::{
    import_chapter, list_chapters, update_chapter_content,
};
pub(in crate::routes::commands) use generation::{
    prepare_chapter_generation_rag_embedding, preview_chapter_prompt,
};
pub(in crate::routes::commands) use scenes::{
    delete_chapter_scene, generate_chapter_scene, list_chapter_scenes,
    preview_chapter_scene_request, reorder_chapter_scenes, update_chapter_scene,
};
pub(in crate::routes::commands) use shooting_script::{
    edit_chapter_shooting_script, generate_chapter_shooting_script,
    generate_next_chapter_shooting_script_unit, get_chapter_shooting_script,
    get_chapter_shooting_script_unit_progress,
};
