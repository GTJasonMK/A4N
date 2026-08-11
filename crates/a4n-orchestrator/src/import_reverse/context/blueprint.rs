use a4n_db::import_reverse::ReverseContextRows;
use serde_json::{Value, json};

use self::chapters::blueprint_chapter_context;
use self::extracted::blueprint_extracted_context;
use self::projections::{existing_blueprint_projection, existing_relationship_projections};
use super::chapter_entries::chapter_part_entry;
use crate::outline::{chapter_outline_context_json, outline_formal_materials_context_json};

mod chapters;
mod extracted;
mod projections;

// Build the compact project context consumed by reverse-engineering requests.
pub(super) fn blueprint_context(rows: &ReverseContextRows, limit: usize) -> Result<Value, String> {
    let blueprint_rows = rows
        .blueprint_context
        .as_ref()
        .ok_or_else(|| "蓝图反推上下文缺少蓝图读取结果".to_string())?;
    let chapter_context = blueprint_chapter_context(&rows.chapters, limit)?;
    let extracted_context = blueprint_extracted_context(&rows.chapters, blueprint_rows);
    let existing_blueprint = existing_blueprint_projection(blueprint_rows);
    let formal_materials = outline_formal_materials_context_json(
        &blueprint_rows.protagonist_profiles,
        &blueprint_rows.objects,
        &blueprint_rows.scenes,
        &blueprint_rows.foreshadowing,
    );
    let total_chapters = rows
        .project
        .total_chapters
        .filter(|value| *value > 0)
        .or_else(|| (rows.project.target_chapters > 0).then_some(rows.project.target_chapters))
        .or_else(|| {
            blueprint_rows
                .existing_blueprint
                .as_ref()
                .and_then(|blueprint| blueprint.total_chapters.filter(|value| *value > 0))
        });

    Ok(json!({
        "novel_title": rows.project.title,
        "project_type": rows.project.project_type,
        "project_description": rows.project.description,
        "total_chapters": total_chapters,
        "sampled_chapters_count": chapter_context.summaries.len(),
        "chapter_summaries": chapter_context.summaries,
        "chapter_outlines": rows.chapter_outlines
            .iter()
            .take(limit)
            .map(chapter_outline_context_json)
            .collect::<Vec<_>>(),
        "part_outlines": rows.part_outlines.iter().map(chapter_part_entry).collect::<Vec<_>>(),
        "sample_chapters": chapter_context.samples,
        "extracted_characters": extracted_context.characters,
        "extracted_foreshadowing": extracted_context.foreshadowing,
        "formal_materials": formal_materials,
        "existing_relationships": existing_relationship_projections(blueprint_rows),
        "existing_blueprint": existing_blueprint,
    }))
}

pub(super) fn has_blueprint_source(rows: &ReverseContextRows) -> bool {
    let Some(blueprint_rows) = rows.blueprint_context.as_ref() else {
        return false;
    };
    !rows.chapters.is_empty()
        || !rows.project.description.trim().is_empty()
        || !rows.chapter_outlines.is_empty()
        || !rows.part_outlines.is_empty()
        || blueprint_rows.existing_blueprint.is_some()
        || !blueprint_rows.blueprint_characters.is_empty()
        || !blueprint_rows.blueprint_relationships.is_empty()
        || !blueprint_rows.protagonist_profiles.is_empty()
        || !blueprint_rows.objects.is_empty()
        || !blueprint_rows.scenes.is_empty()
        || !blueprint_rows.foreshadowing.is_empty()
}

#[cfg(test)]
mod tests {
    use a4n_core::models::project::{blueprint_relationship, novel_project};
    use a4n_db::import_reverse::{ReverseBlueprintContextRows, ReverseContextRows};
    use chrono::NaiveDate;
    use serde_json::json;

    use super::{blueprint_context, has_blueprint_source};

    fn rows(description: &str) -> ReverseContextRows {
        let now = NaiveDate::from_ymd_opt(2026, 8, 11)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        ReverseContextRows {
            project: novel_project::Model {
                id: 9,
                user_id: 1,
                title: "钟楼旧稿".to_string(),
                description: description.to_string(),
                project_type: "novel_long".to_string(),
                project_kind: "text".to_string(),
                source_mode: "self".to_string(),
                root_project_id: None,
                parent_project_id: None,
                source_snapshot_id: None,
                source_sync_status: "current".to_string(),
                derived_project_status: None,
                status: "draft".to_string(),
                lifecycle_gate: "active".to_string(),
                target_chapters: 30,
                current_chapter: 0,
                total_chapters: None,
                chapters_per_part: Some(10),
                avatar_svg: None,
                avatar_animal: None,
                raw_blueprint: None,
                import_analysis_progress: None,
                export_config: None,
                created_at: now,
                updated_at: now,
            },
            chapters: Vec::new(),
            chapter_outlines: Vec::new(),
            part_outlines: Vec::new(),
            blueprint_context: Some(ReverseBlueprintContextRows {
                foreshadowing: Vec::new(),
                existing_blueprint: None,
                blueprint_characters: Vec::new(),
                blueprint_relationships: Vec::new(),
                protagonist_profiles: Vec::new(),
                objects: Vec::new(),
                scenes: Vec::new(),
            }),
        }
    }

    #[test]
    fn blueprint_context_uses_project_plan_without_chapter_body() {
        let rows = rows("从废弃钟楼和旧钥匙继续创作");

        let context = blueprint_context(&rows, 50).unwrap();

        assert!(has_blueprint_source(&rows));
        assert_eq!(context["total_chapters"], 30);
        assert_eq!(context["sampled_chapters_count"], 0);
        assert_eq!(context["project_description"], rows.project.description);
        assert_eq!(context["formal_materials"]["objects"], json!([]));
    }

    #[test]
    fn existing_relationship_is_a_blueprint_source_and_context_fact() {
        let mut rows = rows("");
        let now = rows.project.created_at;
        rows.blueprint_context
            .as_mut()
            .unwrap()
            .blueprint_relationships
            .push(blueprint_relationship::Model {
                id: 1,
                project_id: 9,
                relationship_type: "friend".to_string(),
                character_a: "林岚".to_string(),
                character_b: "陈恪".to_string(),
                description: Some("旧友".to_string()),
                strength: Some(7),
                evolution: None,
                created_at: now,
            });

        let context = blueprint_context(&rows, 50).unwrap();

        assert!(has_blueprint_source(&rows));
        assert_eq!(context["existing_relationships"][0]["character_b"], "陈恪");
    }

    #[test]
    fn title_alone_is_not_enough_to_invent_a_blueprint() {
        assert!(!has_blueprint_source(&rows("")));
    }
}
