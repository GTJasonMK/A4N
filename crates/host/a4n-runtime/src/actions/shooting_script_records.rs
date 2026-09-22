mod delete;
mod edit;
mod edit_progress;
mod generation;
mod read;

pub use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView, DeleteChapterShootingScriptInput, EditChapterShootingScriptInput,
    EditChapterShootingScriptProgressInput, EditedChapterShootingScriptProgressView,
    EditedChapterShootingScriptView, GenerateChapterShootingScriptInput,
};
pub use delete::delete_chapter_shooting_script;
pub use edit::edit_chapter_shooting_script;
pub use edit_progress::edit_chapter_shooting_script_progress;
pub use generation::{
    generate_chapter_shooting_script, generate_next_chapter_shooting_script_unit,
};
pub use read::{get_chapter_shooting_script, get_chapter_shooting_script_unit_progress};

pub const CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL: u32 =
    a4n_novel_host_contract::CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL;
