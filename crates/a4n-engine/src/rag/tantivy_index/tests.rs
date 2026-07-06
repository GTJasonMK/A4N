use std::collections::HashMap;

use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use tantivy::schema::Field;

use super::query::ngram_query_terms;
use super::{TantivyTextIndex, TantivyTextRecord};
use crate::rag::vector_store::SearchFilter;

fn text_record(
    id: impl Into<String>,
    content: impl Into<String>,
    project_id: i64,
) -> TantivyTextRecord {
    TantivyTextRecord::new(id, content, Some(project_id))
}

fn typed_record(
    id: impl Into<String>,
    content: impl Into<String>,
    project_id: i64,
    data_type: impl Into<String>,
    chapter_number: i64,
) -> TantivyTextRecord {
    TantivyTextRecord {
        id: id.into(),
        content: content.into(),
        project_id: Some(project_id),
        data_type: Some(data_type.into()),
        chapter_number: Some(chapter_number),
    }
}

#[tokio::test]
async fn upsert_search_delete_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();

    index
        .upsert(
            "proj1",
            vec![
                text_record("a", "the quick brown fox", 1),
                text_record("b", "lazy dog under the tree", 1),
            ],
        )
        .await
        .unwrap();

    let r = index.search("proj1", "quick fox", 10).await.unwrap();
    assert!(r.iter().any(|x| x.id == "a"));
    assert!(r[0].bm25_score.unwrap_or(0.0) > 0.0);

    let deleted = index
        .delete_by_ids("proj1", vec!["a".to_string()])
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    let r2 = index.search("proj1", "quick fox", 10).await.unwrap();
    assert!(r2.iter().all(|x| x.id != "a"));
}

#[tokio::test]
async fn delete_by_prefixes_removes_matching_documents() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();

    index
        .upsert(
            "proj1",
            vec![
                text_record(
                    "chapter:1:100:p1".to_string(),
                    "needle old chapter".to_string(),
                    1,
                ),
                text_record(
                    "chapter:1:101:p1".to_string(),
                    "needle other chapter".to_string(),
                    1,
                ),
                text_record("outline:1:1".to_string(), "needle outline".to_string(), 1),
            ],
        )
        .await
        .unwrap();

    let deleted = index
        .delete_by_prefixes("proj1", vec!["chapter:1:100:".to_string()])
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    let ids = index
        .search("proj1", "needle", 10)
        .await
        .unwrap()
        .into_iter()
        .map(|result| result.id)
        .collect::<Vec<_>>();
    assert!(!ids.contains(&"chapter:1:100:p1".to_string()));
    assert!(ids.contains(&"chapter:1:101:p1".to_string()));
    assert!(ids.contains(&"outline:1:1".to_string()));
}

#[test]
fn new_rejects_invalid_existing_collection_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let invalid_dir = tmp.path().join(RAG_TEXT_INDEX_VERSION).join("broken");
    std::fs::create_dir_all(&invalid_dir).unwrap();
    std::fs::write(invalid_dir.join("not-an-index"), "bad").unwrap();

    let err = match TantivyTextIndex::new(tmp.path().to_path_buf()) {
        Ok(_) => panic!("invalid existing collection directory should be rejected"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("打开 tantivy 索引失败"));
}

#[tokio::test]
async fn search_treats_query_syntax_chars_as_plain_text() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();
    index
        .upsert(
            "proj1",
            vec![text_record(
                "a",
                "暗棋初动 陈恪 母亲 魏征 玄武门 character_secret plot_twist".to_string(),
                1,
            )],
        )
        .await
        .unwrap();

    let results = index
        .search(
            "proj1",
            "第2章\n章节标题: 暗棋初动\n待处理伏笔: 陈恪开始遗忘现代记忆 [character_secret] 原文: 母亲 魏征 玄武门",
            10,
        )
        .await
        .unwrap();

    assert!(results.iter().any(|result| result.id == "a"));
}

#[tokio::test]
async fn search_retrieves_chinese_source_fragment_with_ngram_index() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();
    index
        .upsert(
            "proj1",
            vec![
                text_record(
                    "chapter:5:2:p7",
                    "他吹熄了油灯，躺下来，闭上眼睛。黑暗中有一种被注视的压迫感，悄无声息地包裹着他，像一条看不见的蛇，在他身边缓缓游走，等待着他犯下一个错误。".to_string(),
                    5,
                ),
                text_record(
                    "character:5:1",
                    "陈恪 外貌 中等身材 面容清瘦 肩窄 青色文官袍".to_string(),
                    5,
                ),
            ],
        )
        .await
        .unwrap();

    let results = index
        .search(
            "proj1",
            "黑暗中有一种被注视的压迫感，悄无声息地包裹着他",
            10,
        )
        .await
        .unwrap();

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("chapter:5:2:p7")
    );
}

#[test]
fn ngram_query_terms_normalize_query_syntax_as_plain_text() {
    let field = Field::from_field_id(1);
    let terms = ngram_query_terms(field, "\"unterminated field:value [tag] AND 陈恪");
    let texts = terms
        .iter()
        .map(|term| term.serialized_term().to_vec())
        .collect::<Vec<_>>();

    assert!(!terms.is_empty());
    assert!(texts.iter().any(|bytes| bytes.ends_with("陈恪".as_bytes())));
}

#[tokio::test]
async fn search_filtered_applies_data_type_before_top_k() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();
    let mut records = (0..8)
        .map(|i| {
            typed_record(
                format!("outline:{i}"),
                "needle needle needle outline",
                1,
                "outline",
                1,
            )
        })
        .collect::<Vec<_>>();
    records.push(typed_record("chapter:1", "needle chapter", 1, "chapter", 2));
    index.upsert("proj1", records).await.unwrap();

    let results = index
        .search_filtered(
            "proj1",
            "needle",
            1,
            Some(&SearchFilter {
                equals: None,
                data_types: Some(vec!["chapter".to_string()]),
                chapter_range: None,
            }),
        )
        .await
        .unwrap();

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("chapter:1")
    );
}

#[tokio::test]
async fn search_filtered_applies_chapter_range_before_top_k() {
    let tmp = tempfile::tempdir().unwrap();
    let index = TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap();
    let mut records = (0..8)
        .map(|i| {
            typed_record(
                format!("chapter:old:{i}"),
                "needle needle needle old",
                1,
                "chapter",
                1,
            )
        })
        .collect::<Vec<_>>();
    records.push(typed_record(
        "chapter:target",
        "needle target",
        1,
        "chapter",
        4,
    ));
    index.upsert("proj1", records).await.unwrap();

    let results = index
        .search_filtered(
            "proj1",
            "needle",
            1,
            Some(&SearchFilter {
                equals: Some(HashMap::from([(
                    "project_id".to_string(),
                    serde_json::json!(1),
                )])),
                data_types: Some(vec!["chapter".to_string()]),
                chapter_range: Some((4, 4)),
            }),
        )
        .await
        .unwrap();

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("chapter:target")
    );
}
