use a4n_db_models::models::chapter::chapter_outline;
use chrono::NaiveDateTime;

use super::super::types::OutlineDbError;
use super::rules::index_chapter_outlines_by_number;

fn outline(id: i64, chapter_number: i32) -> chapter_outline::Model {
    chapter_outline::Model {
        id,
        project_id: 7,
        part_id: Some(20),
        chapter_number,
        title: Some(format!("第 {chapter_number} 章")),
        summary: Some("摘要".to_string()),
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
    }
}

#[test]
fn index_chapter_outlines_by_number_rejects_duplicate_chapter_number() {
    let error = index_chapter_outlines_by_number(vec![outline(101, 3), outline(202, 3)])
        .expect_err("duplicate chapter outlines must fail");

    assert!(matches!(
        error,
        OutlineDbError::InvalidArgument(message)
            if message == "第 3 章已有重复章纲记录：101, 202，请先清理后再应用节拍映射"
    ));
}

#[test]
fn index_chapter_outlines_by_number_preserves_distinct_chapters() {
    let outlines = index_chapter_outlines_by_number(vec![outline(202, 8), outline(101, 3)])
        .expect("distinct chapter outlines must be indexed");

    assert_eq!(outlines.keys().copied().collect::<Vec<_>>(), vec![3, 8]);
    assert_eq!(outlines.get(&3).map(|outline| outline.id), Some(101));
    assert_eq!(outlines.get(&8).map(|outline| outline.id), Some(202));
}
