use a4n_db::optimize::OptimizationChapterSummaryRecord;
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::project::ProjectOptimizationContextRecord;
use serde_json::json;

use super::manifest::optimization_context_manifest;
use super::projection::{
    optimization_chapter_context, optimization_chapter_summary_context,
    optimization_project_context,
};

#[test]
fn context_manifest_keeps_sorted_sections_counts_and_known_descriptions() {
    let manifest = optimization_context_manifest(&json!({
        "previous_chapters": [{"chapter_number": 1}, {"chapter_number": 2}],
        "current_chapter": {"chapter_number": 3},
        "unknown_extension": {"enabled": true},
        "character_profiles": [{"name": "林雾"}],
    }));

    assert_eq!(
        manifest["sections"],
        json!([
            "character_profiles",
            "current_chapter",
            "previous_chapters",
            "unknown_extension"
        ])
    );
    assert_eq!(manifest["counts"]["character_profiles"], json!(1));
    assert_eq!(manifest["counts"]["current_chapter"], json!(1));
    assert_eq!(manifest["counts"]["previous_chapters"], json!(2));
    assert!(manifest["counts"].get("unknown_extension").is_none());
    assert_eq!(manifest["section_details"].as_array().unwrap().len(), 3);
}

#[test]
fn context_manifest_returns_empty_contract_for_non_object_input() {
    assert_eq!(
        optimization_context_manifest(&json!(["invalid"])),
        json!({
            "sections": [],
            "counts": {},
            "section_details": [],
        })
    );
}

#[test]
fn chapter_projection_trims_optional_text_without_copying_derived_source_spans() {
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 7, 13)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let chapter = ChapterRecord {
        id: 7,
        project_id: 3,
        outline_id: None,
        chapter_number: 4,
        title: Some("  旧城回声  ".to_string()),
        content: "第一段正文。\n\n第二段正文。".to_string(),
        word_count: 12,
        status: "completed".to_string(),
        summary: Some("  主角听见回声。  ".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let current = optimization_chapter_context(&chapter);
    assert_eq!(current["content"], chapter.content);
    assert_eq!(current["title"], json!("旧城回声"));
    assert_eq!(current["summary"], json!("主角听见回声。"));
    assert_eq!(current["status"], json!("completed"));
    assert!(current.get("source_spans").is_none());

    let neighboring = optimization_chapter_summary_context(&OptimizationChapterSummaryRecord {
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        word_count: chapter.word_count,
        analysis_data: chapter.analysis_data.clone(),
    });
    assert_eq!(neighboring["title"], json!("旧城回声"));
    assert_eq!(neighboring["summary"], json!("主角听见回声。"));
    assert!(neighboring.get("source_spans").is_none());
    assert!(neighboring.get("content").is_none());
    assert!(neighboring.get("status").is_none());
}

#[test]
fn project_projection_keeps_existing_fields_and_trims_optional_text() {
    let project = ProjectOptimizationContextRecord {
        id: 3,
        title: "旧城".to_string(),
        description: "悬疑故事".to_string(),
        project_type: "novel_long".to_string(),
        status: "draft".to_string(),
        target_chapters: 20,
        current_chapter: 4,
        total_chapters: Some(20),
        chapters_per_part: Some(5),
        raw_blueprint: Some("  蓝图  ".to_string()),
        export_config: Some("  ".to_string()),
    };

    let value = optimization_project_context(&project);

    assert_eq!(value["id"], 3);
    assert_eq!(value["raw_blueprint"], "蓝图");
    assert!(value["export_config"].is_null());
    assert_eq!(value.as_object().unwrap().len(), 11);
}
