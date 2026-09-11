mod context_contract;
mod parsing_contract;
mod planning_contract;
mod sync_contract;
mod validation_contract;

use super::{
    application::completed_part_count,
    chapter_contract::{
        optional_i32, parse_beat_sheet_json, parse_chapter_outline_batch,
        parse_outline_json_object, reject_keys, reject_unknown_keys, required_part_summary,
        required_string, sync_chapter_outlines_by_number_in, validate_chapter_generation_range,
        validate_chapter_outline_generation_request, validate_requested_part_context,
    },
    context::{
        chapter_outline_context_json, has_outline_generation_source, minimal_blueprint_context,
        outline_blueprint_context_json, outline_formal_materials_context_json,
    },
    ensure_response_not_truncated, order_parts_for_batch_generation,
    part_contract::{
        PartOutlineDraft, ensure_continue_part_request, infer_chapters_per_part_from_existing,
        parse_part_outline_single_response, resolve_part_generation_plan,
        sync_part_outlines_and_assign_chapters_in, validate_existing_parts_for_plan,
        validate_part_generation_inputs,
    },
    report_progress, validate_existing_chapter_outline_for_request,
};

use a4n_db::project::{OutlineGenerationProgressChapter, OutlineGenerationProgressPart};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::{chapter_outline, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint, novel_project},
    scene::scene_profile,
};
use a4n_error::AppError;
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

async fn insert_project(db: &DatabaseConnection, project_id: i64) {
    let now = fixed_time(1);
    novel_project::ActiveModel {
        id: Set(project_id),
        user_id: Set(1),
        title: Set(format!("大纲测试项目 {project_id}")),
        description: Set("大纲同步测试".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_sync_status: Set("current".to_string()),
        status: Set("writing".to_string()),
        target_chapters: Set(30),
        current_chapter: Set(0),
        total_chapters: Set(Some(30)),
        chapters_per_part: Set(Some(10)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
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

#[test]
fn batch_parts_use_canonical_part_order() {
    let mut parts = [(30, 3), (10, 1), (20, 2), (19, 2)].map(|(id, part_number)| {
        let mut part = sample_part_for_plan(part_number, part_number * 10 - 9, part_number * 10);
        part.id = id;
        part
    });

    order_parts_for_batch_generation(&mut parts);

    assert_eq!(parts.map(|part| part.id), [10, 19, 20, 30]);
}

#[test]
fn completed_part_count_requires_full_completed_chapter_range() {
    let parts = [(1, 2), (3, 4), (5, 5)]
        .into_iter()
        .enumerate()
        .map(|(index, (start, end))| OutlineGenerationProgressPart {
            id: index as i64 + 1,
            start_chapter: start,
            end_chapter: end,
        })
        .collect::<Vec<_>>();
    let outlines = (1..=5)
        .map(|chapter_number| OutlineGenerationProgressChapter {
            id: 100 + i64::from(chapter_number),
            chapter_number,
            status: if chapter_number == 4 {
                chapter_outline::STATUS_DRAFT
            } else {
                chapter_outline::STATUS_COMPLETED
            }
            .to_string(),
        })
        .collect::<Vec<_>>();

    assert_eq!(completed_part_count(&parts, &outlines).unwrap(), 2);
}

#[test]
fn completed_part_count_rejects_duplicate_or_overlapping_chapters() {
    let part = OutlineGenerationProgressPart {
        id: 1,
        start_chapter: 1,
        end_chapter: 2,
    };
    let first = OutlineGenerationProgressChapter {
        id: 101,
        chapter_number: 1,
        status: chapter_outline::STATUS_COMPLETED.to_string(),
    };
    let mut duplicate = first.clone();
    duplicate.id += 1;
    let error = completed_part_count(&[part], &[first, duplicate]).unwrap_err();
    assert!(error.to_string().contains("存在重复章节大纲"));

    let parts = [
        OutlineGenerationProgressPart {
            id: 1,
            start_chapter: 1,
            end_chapter: 2,
        },
        OutlineGenerationProgressPart {
            id: 2,
            start_chapter: 2,
            end_chapter: 3,
        },
    ];
    let error = completed_part_count(&parts, &[]).unwrap_err();
    assert!(error.to_string().contains("被多个分部范围覆盖"));
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
