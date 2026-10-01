use a4n_db::import_reverse::ReverseContextRows;
use serde_json::{Value, json};

use super::ReverseContextKind;
use super::blueprint::blueprint_context;
use super::chapter_entries::{
    chapter_outline_entry, chapter_part_entry, chapter_summary_for_reverse,
};

// Select the target-specific projection only after validating the canonical rows scope.
pub(super) fn project_reverse_context(
    rows: &ReverseContextRows,
    project_id: i64,
    kind: ReverseContextKind,
    limit: usize,
) -> Result<Value, String> {
    if rows.project.id != project_id {
        return Err(format!("项目不存在: project_id={project_id}"));
    }
    match kind {
        ReverseContextKind::Outline => {
            ensure_chapter_source(rows)?;
            Ok(json!({
                "novel_title": rows.project.title,
                "total_chapters": rows.chapters.len(),
                "summaries": rows.chapters
                    .iter()
                    .take(limit)
                    .map(chapter_summary_for_reverse)
                    .collect::<Vec<_>>(),
            }))
        }
        ReverseContextKind::PartOutline => {
            ensure_part_outline_source(rows)?;
            let outline_values = if rows.chapter_outlines.is_empty() {
                rows.chapters
                    .iter()
                    .take(limit)
                    .map(chapter_outline_entry)
                    .collect::<Vec<_>>()
            } else {
                rows.chapter_outlines
                    .iter()
                    .take(limit)
                    .map(|outline| {
                        json!({
                            "chapter_number": outline.chapter_number,
                            "title": outline.title,
                            "summary": outline.summary,
                            "key_scenes": outline.key_scenes,
                            "characters_involved": outline.characters_involved,
                        })
                    })
                    .collect::<Vec<_>>()
            };
            Ok(json!({
                "novel_title": rows.project.title,
                "total_chapters": part_outline_source_end(rows),
                "chapter_outlines": outline_values,
                "existing_part_outlines": rows.part_outlines
                    .iter()
                    .map(chapter_part_entry)
                    .collect::<Vec<_>>(),
            }))
        }
        ReverseContextKind::Blueprint => blueprint_context(rows, limit),
    }
}

fn ensure_chapter_source(rows: &ReverseContextRows) -> Result<(), String> {
    if rows.chapters.is_empty() {
        return Err("项目没有可用于反推的章节正文".to_string());
    }
    Ok(())
}

fn ensure_part_outline_source(rows: &ReverseContextRows) -> Result<(), String> {
    if rows.chapters.is_empty() && rows.chapter_outlines.is_empty() {
        return Err("项目没有可用于归纳分部大纲的章节大纲或章节正文".to_string());
    }
    Ok(())
}

fn part_outline_source_end(rows: &ReverseContextRows) -> i32 {
    rows.chapter_outlines
        .iter()
        .map(|outline| outline.chapter_number)
        .chain(rows.chapters.iter().map(|chapter| chapter.chapter_number))
        .chain(rows.part_outlines.iter().map(|part| part.end_chapter))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use a4n_db::import_reverse::ReverseContextRows;
    use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord};
    use a4n_domain::project::ProjectRecord;
    use chrono::NaiveDateTime;

    use super::super::ReverseContextKind;
    use super::project_reverse_context;

    fn rows() -> ReverseContextRows {
        ReverseContextRows {
            project: ProjectRecord {
                id: 3,
                user_id: 1,
                title: "跳写项目".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
                project_kind: "text".to_string(),
                root_project_id: None,
                source_snapshot_id: None,
                source_sync_status: "current".to_string(),
                status: "draft".to_string(),
                lifecycle_gate: "active".to_string(),
                target_chapters: 100,
                current_chapter: 0,
                total_chapters: None,
                chapters_per_part: Some(25),
                avatar_svg: None,
                avatar_animal: None,
                raw_blueprint: None,
                import_analysis_progress: None,
                export_config: None,
                created_at: NaiveDateTime::MIN,
                updated_at: NaiveDateTime::MIN,
            },
            chapters: Vec::new(),
            chapter_outlines: vec![ChapterOutlineRecord {
                id: 12,
                project_id: 3,
                part_id: Some(21),
                chapter_number: 12,
                title: Some("钟楼来信".to_string()),
                summary: Some("林岚收到来自未来的信。".to_string()),
                beats: None,
                key_scenes: None,
                pacing_score: None,
                foreshadowing_setup: None,
                foreshadowing_payoff: None,
                characters_involved: None,
                word_count_target: None,
                status: "draft".to_string(),
                created_at: NaiveDateTime::MIN,
                updated_at: NaiveDateTime::MIN,
            }],
            part_outlines: vec![PartOutlineRecord {
                id: 21,
                project_id: 3,
                part_number: 1,
                title: "旧钟初鸣".to_string(),
                start_chapter: 1,
                end_chapter: 12,
                summary: Some("主角开始调查钟楼。".to_string()),
                theme: Some("启程".to_string()),
                key_events: None,
                character_focus: None,
                created_at: NaiveDateTime::MIN,
                updated_at: NaiveDateTime::MIN,
            }],
            blueprint_context: None,
        }
    }

    #[test]
    fn part_outline_context_uses_chapter_outlines_without_chapter_body() {
        let context = project_reverse_context(&rows(), 3, ReverseContextKind::PartOutline, 200)
            .expect("chapter outlines are sufficient for part-outline aggregation");

        assert_eq!(context["total_chapters"], 12);
        assert_eq!(context["chapter_outlines"][0]["chapter_number"], 12);
        assert_eq!(context["existing_part_outlines"][0]["part_number"], 1);
    }

    #[test]
    fn chapter_outline_reverse_still_requires_chapter_body() {
        let error = project_reverse_context(&rows(), 3, ReverseContextKind::Outline, 100)
            .expect_err("retrospective chapter outlines still need chapter bodies");

        assert_eq!(error, "项目没有可用于反推的章节正文");
    }
}
