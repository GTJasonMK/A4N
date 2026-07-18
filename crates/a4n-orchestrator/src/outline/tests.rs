mod context_contract;
mod parsing_contract;
mod planning_contract;
mod sync_contract;
mod validation_contract;

use super::{
    chapter_contract::{
        optional_i32, parse_beat_sheet_json, parse_chapter_outline_batch,
        parse_outline_json_object, reject_keys, reject_unknown_keys, required_part_summary,
        required_string, sync_chapter_outlines_by_number, validate_chapter_generation_range,
        validate_chapter_outline_generation_request, validate_requested_part_context,
    },
    context::{chapter_outline_context_json, outline_blueprint_context_json},
    ensure_response_not_truncated,
    part_contract::{
        PartGenerationPlan, PartOutlineDraft, chapter_outline_ids_in_part_range,
        chapter_outline_ids_in_part_ranges, ensure_continue_part_request,
        parse_part_outline_single_response, resolve_part_generation_plan,
        sync_chapter_outline_part_assignments, sync_part_outlines,
        validate_existing_parts_for_plan, validate_part_generation_inputs,
    },
    report_progress, validate_existing_chapter_outline_for_request,
};

use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::{chapter_outline, part_outline},
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::{Value, json};

#[test]
fn outline_progress_rejects_invalid_ranges() {
    let reporter = |_: u32, _: u32, _: &str| Ok(());

    assert!(report_progress(Some(&reporter), 0, 0, "invalid total").is_err());
    assert!(report_progress(Some(&reporter), -1, 1, "negative current").is_err());
    assert!(report_progress(Some(&reporter), 2, 1, "current too large").is_err());
}

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn part_json(part_number: i32, title: &str) -> Value {
    json!({
        "part_number": part_number,
        "title": title,
        "summary": format!("{title}摘要"),
        "theme": "代价",
        "key_events": ["关键事件"],
        "character_focus": {"主角": "成长"}
    })
}

fn sample_part(summary: Option<&str>) -> part_outline::Model {
    let mut part = sample_part_for_plan(2, 11, 20);
    part.summary = summary.map(ToString::to_string);
    part
}

fn sample_part_for_plan(
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> part_outline::Model {
    let now = Utc::now().naive_utc();
    part_outline::Model {
        id: 1,
        project_id: 10,
        part_number,
        title: "第二卷".to_string(),
        start_chapter,
        end_chapter,
        summary: Some("星火初燃".to_string()),
        theme: None,
        key_events: None,
        character_focus: None,
        created_at: now,
        updated_at: now,
    }
}

fn sample_chapter_outline(chapter_number: i32, part_id: Option<i64>) -> chapter_outline::Model {
    let now = Utc::now().naive_utc();
    chapter_outline::Model {
        id: 100 + i64::from(chapter_number),
        project_id: 10,
        part_id,
        chapter_number,
        title: Some(format!("第 {chapter_number} 章")),
        summary: Some("章节摘要".to_string()),
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    }
}
