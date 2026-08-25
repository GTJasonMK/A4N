use a4n_core::{blueprint_context::blueprint_llm_context, chapter_analysis::summary_text};
use a4n_db::inspiration::{InspirationProjectContextRows, InspirationRecentChapterRow};
use serde_json::{Value, json};

pub(super) fn inspiration_project_context(rows: InspirationProjectContextRows) -> Value {
    let project = rows.project;
    let mut context = serde_json::Map::new();
    context.insert(
        "project".to_string(),
        json!({
            "title": project.title,
            "description": optional_trimmed_string(&project.description),
            "project_type": project.project_type,
            "status": project.status,
            "target_chapters": project.target_chapters,
            "current_chapter": project.current_chapter,
            "total_chapters": project.total_chapters,
            "chapters_per_part": project.chapters_per_part,
        }),
    );

    if let Some(blueprint) = rows.blueprint.as_ref() {
        context.insert(
            "blueprint".to_string(),
            blueprint_llm_context(
                &blueprint.blueprint,
                &blueprint.characters,
                &blueprint.relationships,
            ),
        );
    }

    let recent_chapters = rows
        .recent_chapters
        .iter()
        .filter_map(inspiration_chapter_context)
        .collect::<Vec<_>>();
    if !recent_chapters.is_empty() {
        context.insert("recent_chapters".to_string(), Value::Array(recent_chapters));
    }

    Value::Object(context)
}

pub(super) fn optional_trimmed_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn inspiration_chapter_context(ch: &InspirationRecentChapterRow) -> Option<Value> {
    if !ch.has_content && ch.summary.as_deref().unwrap_or("").trim().is_empty() {
        return None;
    }
    let summary = ch
        .analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        });
    Some(json!({
        "chapter_number": ch.chapter_number,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": summary,
        "word_count": ch.word_count,
    }))
}
