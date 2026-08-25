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
