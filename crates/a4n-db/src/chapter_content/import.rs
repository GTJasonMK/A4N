mod execution;
mod persistence;
mod preparation;

use a4n_core::models::chapter::chapter;

pub use execution::import_chapter_content_record;

struct ImportChapterContentTarget {
    existing: Option<chapter::Model>,
}
