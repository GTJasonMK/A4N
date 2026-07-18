use super::{
    blueprint::{
        ImportTextProjectBlueprint, ImportTextProjectBlueprintVersion, ImportTextProjectCharacter,
        ImportTextProjectRelationship,
    },
    outlines::{ImportTextProjectChapterOutline, ImportTextProjectPartOutline},
};
use a4n_core::models::chapter::chapter;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectPayload {
    pub chapters: Vec<ImportTextProjectChapter>,
    pub blueprint: Option<ImportTextProjectBlueprint>,
    pub blueprint_versions: Vec<ImportTextProjectBlueprintVersion>,
    pub characters: Vec<ImportTextProjectCharacter>,
    pub relationships: Vec<ImportTextProjectRelationship>,
    pub part_outlines: Vec<ImportTextProjectPartOutline>,
    pub chapter_outlines: Vec<ImportTextProjectChapterOutline>,
}

impl ImportTextProjectPayload {
    pub fn chapter_bound_counts(&self) -> (i32, i32, i32) {
        let mut max_chapter_number = 0;
        let mut max_available_content_chapter_number = 0;
        let mut available_content_chapter_count = 0_i32;
        for chapter in &self.chapters {
            max_chapter_number = max_chapter_number.max(chapter.chapter_number);
            if chapter.status != chapter::STATUS_DELETED && !chapter.content.trim().is_empty() {
                max_available_content_chapter_number =
                    max_available_content_chapter_number.max(chapter.chapter_number);
                available_content_chapter_count += 1;
            }
        }
        (
            max_chapter_number,
            max_available_content_chapter_number,
            available_content_chapter_count,
        )
    }

    pub fn has_preserved_chapter_tags(&self) -> bool {
        self.chapters
            .iter()
            .any(|chapter| chapter.style_tags.is_some())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectChapter {
    pub old_outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
    pub analysis_data: Option<Value>,
}
