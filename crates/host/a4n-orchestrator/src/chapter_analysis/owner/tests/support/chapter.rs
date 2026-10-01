pub fn sample_chapter() -> ChapterRecord {
    let now = NaiveDate::from_ymd_opt(2026, 5, 26)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    ChapterRecord {
        id: 11,
        project_id: 22,
        outline_id: None,
        chapter_number: 3,
        title: Some("第三章".into()),
        content: "正文".into(),
        word_count: 2,
        status: "completed".into(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}
