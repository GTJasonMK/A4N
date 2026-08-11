mod contracts;
mod edit;
mod generation;
mod read;

pub use contracts::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView, EditChapterShootingScriptInput, EditedChapterShootingScriptView,
};
pub use edit::edit_chapter_shooting_script;
pub use generation::{
    generate_chapter_shooting_script, generate_next_chapter_shooting_script_unit,
};
pub use read::{get_chapter_shooting_script, get_chapter_shooting_script_unit_progress};

pub const CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL: u32 =
    a4n_orchestrator::shooting_script::CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL;
