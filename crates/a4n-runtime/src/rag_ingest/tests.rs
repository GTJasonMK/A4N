mod expected_records_contract;
mod sync_contract;

use a4n_core::models::chapter::chapter;
use chrono::{NaiveDate, NaiveDateTime};

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid test date")
        .and_hms_opt(0, 0, 0)
        .expect("valid test time")
}

fn sample_chapter(status: &str, content: &str) -> chapter::Model {
    chapter::Model {
        id: 90,
        project_id: 1,
        outline_id: None,
        chapter_number: 5,
        title: Some("旧城回声".to_string()),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: Some("主角听见旧城广播。".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: ts(),
        updated_at: ts(),
    }
}
