use super::*;

fn source(status: &str) -> ChapterMangaSourceSummary {
    ChapterMangaSourceSummary {
        chapter_id: 7,
        chapter_number: 3,
        status: status.to_string(),
        content: "章节正文".to_string(),
    }
}

#[test]
fn manga_source_write_gate_preserves_deleted_and_generating_errors() {
    assert_eq!(
        ensure_manga_source_can_write(&source("deleted"), "生成漫画")
            .expect_err("deleted source should reject")
            .to_string(),
        "参数无效：第 3 章已删除，无法生成漫画"
    );
    assert_eq!(
        ensure_manga_source_can_write(&source("generating"), "生成漫画")
            .expect_err("generating source should reject")
            .to_string(),
        "参数无效：第 3 章正在生成中，请等待完成后再生成漫画"
    );
    ensure_manga_source_can_write(&source("draft"), "生成漫画")
        .expect("draft source should allow writes");
}

#[test]
fn manga_reference_context_preserves_portrait_summary_fields() {
    let context = manga_reference_context(vec![MangaCharacterPortraitRefSummary {
        profile_id: 1,
        portrait_id: 2,
        character_name: "林岚".to_string(),
        image_path: "portraits/2.png".to_string(),
        prompt: "portrait".to_string(),
        portrait_type: "primary".to_string(),
        expression: Some("calm".to_string()),
        pose: Some("front".to_string()),
        is_primary: true,
    }]);

    let portrait = &context.character_portraits[0];
    assert_eq!(portrait.profile_id, 1);
    assert_eq!(portrait.portrait_id, 2);
    assert_eq!(portrait.character_name, "林岚");
    assert_eq!(portrait.image_path, "portraits/2.png");
    assert_eq!(portrait.prompt, "portrait");
    assert_eq!(portrait.portrait_type, "primary");
    assert_eq!(portrait.expression.as_deref(), Some("calm"));
    assert_eq!(portrait.pose.as_deref(), Some("front"));
    assert!(portrait.is_primary);
}
