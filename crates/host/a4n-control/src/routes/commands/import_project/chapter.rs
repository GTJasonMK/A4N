mod analysis;
mod content;
mod generation;
mod scenes;
mod shooting_script;
mod visual_plan;

pub(in crate::routes::commands) use analysis::{
    generate_chapter_analysis, preview_chapter_analysis_request,
    preview_resume_chapter_analysis_sync_request, resume_chapter_analysis_sync,
};
pub(in crate::routes::commands) use content::{
    get_chapter, import_chapter, list_chapter_analyses, list_chapter_index, list_chapters,
    list_story_event_chapters, update_chapter_content,
};
pub(in crate::routes::commands) use generation::{
    prepare_chapter_generation_rag_embedding, preview_chapter_prompt,
};
pub(in crate::routes::commands) use scenes::{
    delete_chapter_scene, generate_chapter_scene, list_chapter_scenes,
    preview_chapter_scene_request, reorder_chapter_scenes, update_chapter_scene,
};
pub(in crate::routes::commands) use shooting_script::{
    delete_chapter_shooting_script, edit_chapter_shooting_script,
    edit_chapter_shooting_script_progress, get_chapter_shooting_script,
    get_chapter_shooting_script_unit_progress,
};
pub(in crate::routes::commands) use visual_plan::{
    confirm_chapter_visual_plan, discard_chapter_visual_plan_draft, edit_chapter_visual_plan_draft,
    get_chapter_visual_plan, preview_visual_plan_point_generation, read_chapter_visual_plan,
    read_visual_plan_prompt_candidate, save_chapter_visual_plan_draft,
    validate_chapter_visual_plan,
};
