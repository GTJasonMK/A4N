use super::*;
use serde_json::json;

use super::execution::{execute_edit_article, execute_get_project_information};

#[test]
fn information_tool_reads_context_section() {
    let context = json!({
        "current_chapter": {
            "chapter_number": 1,
            "content": "第一段正文。第二段出现动机断裂。"
        }
    });
    let result = execute_get_project_information(
        json!({
            "mode": "read",
            "source": "current_chapter"
        }),
        &context,
    )
    .unwrap();

    assert_eq!(result["tool"], GET_PROJECT_INFORMATION_TOOL);
    assert!(
        result["content"]["content"]
            .as_str()
            .unwrap()
            .contains("第二段")
    );
}

#[test]
fn paged_reads_preserve_text_and_only_expose_read_evidence() {
    let context =
        json!({"current_chapter": {"chapter_number": 1, "content": "第一句。第二句。第三句。"}});
    let first = execute_get_project_information(
        json!({"mode": "read", "source": "current_chapter", "limit": 4}),
        &context,
    )
    .unwrap();
    assert_eq!(first["content"]["content"], "#0 第一句。");
    assert_eq!(first["next_offset"], 4);
    assert_eq!(
        first["content"]["source_span_ids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let second = execute_get_project_information(json!({"mode": "read", "source": "current_chapter", "offset": 4, "limit": 4, "expected_hash": first["resource"]["hash"]}), &context).unwrap();
    assert_eq!(second["content"]["content"], "#1 第二句。");
    let observed =
        observed_evidence_context(&context, &[json!({"success": true, "result": first})]).unwrap();
    assert_eq!(
        observed["current_chapter"]["source_spans"][0]["id"],
        "ch1:s1:f1"
    );
    assert_eq!(
        observed["current_chapter"]["source_spans"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        execute_get_project_information(
            json!({"mode": "read", "source": "current_chapter", "expected_hash": "stale"}),
            &context
        )
        .is_err()
    );
    assert!(
        execute_get_project_information(
            json!({"mode": "read", "source": "current_chapter", "offset": 100}),
            &context
        )
        .is_err()
    );
    assert!(
        execute_get_project_information(
            json!({"mode": "read", "source": "current_chapter", "limit": 4097}),
            &context
        )
        .is_err()
    );
    assert!(
        execute_get_project_information(
            json!({"mode": "read", "source": "current_chapter", "path": "/source_spans"}),
            &context
        )
        .is_err()
    );
    let stale = observed_evidence_context(
        &json!({"current_chapter": {"chapter_number": 1, "content": "新正文"}}),
        &[json!({"success": true, "result": second})],
    )
    .unwrap();
    assert!(
        stale["current_chapter"]["source_spans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn read_source_avoids_marker_collisions_and_preserves_literal_hashes() {
    let context = json!({"current_chapter": {
        "chapter_number": 1,
        "content": "#0开头。下一句。"
    }});
    let result = execute_get_project_information(
        json!({"mode": "read", "source": "current_chapter"}),
        &context,
    )
    .unwrap();

    assert_eq!(result["content"]["content"], "#1 #0开头。#2 下一句。");
    assert_eq!(result["content"]["source_span_ids"], json!(["#1", "#2"]));
    assert!(result["content"]["source_marker_protocol"].is_string());
    assert!(!result.to_string().contains("ch1:s1:f1"));

    let path_read = execute_get_project_information(
        json!({"mode": "read", "source": "current_chapter", "path": "/content"}),
        &context,
    )
    .unwrap();
    assert_eq!(
        path_read["content"]["content"],
        result["content"]["content"]
    );
    assert_eq!(
        path_read["content"]["source_span_ids"],
        result["content"]["source_span_ids"]
    );
    assert!(!path_read.to_string().contains("ch1:s1:f1"));
}

#[test]
fn information_tool_searches_across_sources() {
    let context = json!({
        "current_chapter": {
            "chapter_number": 1,
            "content": "陈恪进入旧城。"
        },
        "previous_chapters": [{
            "summary": "旧城线索第一次出现。"
        }]
    });
    let result = execute_get_project_information(
        json!({
            "mode": "search",
            "query": "旧城",
            "limit": 2
        }),
        &context,
    )
    .unwrap();

    assert_eq!(result["matches"].as_array().unwrap().len(), 2);
    for hit in result["matches"].as_array().unwrap() {
        let read = execute_get_project_information(
            json!({
                "mode": "read", "source": hit["source"], "path": hit["path"],
                "expected_hash": hit["resource"]["hash"],
            }),
            &context,
        )
        .unwrap();
        let content = read["content"]
            .as_str()
            .or_else(|| read["content"]["content"].as_str())
            .unwrap();
        assert!(content.contains("旧城"));
    }
}

#[test]
fn search_paths_escape_keys_and_snippets_use_original_unicode_offsets() {
    let text = format!("{}needle 在此", "İ".repeat(200));
    let context = json!({"project_detail": {"a/b~c": [{"with.dot": text}]}});
    let found = execute_get_project_information(
        json!({
            "mode": "search", "source": "project_detail", "query": "NEEDLE",
        }),
        &context,
    )
    .unwrap();
    let hit = &found["matches"][0];
    assert_eq!(hit["path"], "/a~1b~0c/0/with.dot");
    assert!(hit["snippet"].as_str().unwrap().contains("needle 在此"));
    let read = execute_get_project_information(
        json!({
            "mode": "read", "source": "project_detail", "path": hit["path"],
        }),
        &context,
    )
    .unwrap();
    assert_eq!(read["content"], text);
}

#[test]
fn edit_tool_returns_validated_proposal() {
    let context = json!({
        "current_chapter": {
            "content": "主角收到旧信，决定进入旧城。"
        }
    });
    let result = execute_edit_article(
        json!({
            "summary": "增强动机",
            "patches": [{
                "patch_id": "patch-001",
                "issue_id": null,
                "operation": "replace",
                "old_text": "主角收到旧信，决定进入旧城",
                "new_text": "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城",
                "reason": "补足行动动机",
                "risk": null
            }]
        }),
        &context,
        1,
        2,
        3,
    )
    .unwrap();

    assert_eq!(result["tool"], EDIT_ARTICLE_TOOL);
    assert_eq!(result["mode"], "pending_confirmation");
    assert_eq!(result["patch_count"], 1);
    assert_eq!(result["session_id"], 1);
}
