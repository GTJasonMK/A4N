#[path = "chapter_scene/generated.rs"]
mod generated;
#[path = "chapter_scene/records.rs"]
mod records;
#[path = "chapter_scene/reorder.rs"]
mod reorder;
#[path = "chapter_scene/types.rs"]
mod types;

pub use generated::save_generated_chapter_scene_record;
pub use records::{
    chapter_by_project_number, chapter_scene_by_id, chapter_scenes_by_chapter,
    delete_chapter_scene_record, update_chapter_scene_record,
};
pub use reorder::{chapter_scene_reorder_project_id, reorder_chapter_scene_records};
pub use types::{
    ProjectChapterSceneRecordError, ProjectChapterSceneUpdateInput, ProjectChapterSceneWriteError,
    ProjectGeneratedChapterSceneSaveInput,
};

#[cfg(test)]
#[path = "chapter_scene/tests.rs"]
mod tests;
