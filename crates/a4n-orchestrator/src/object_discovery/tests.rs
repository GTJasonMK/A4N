use std::collections::BTreeSet;

use a4n_core::{
    models::{chapter::chapter, project::text_anchor},
    source_spans::source_spans_for_chapters,
};
use serde_json::{Value, json};

use super::{
    output::parse_and_validate_object_discovery_content,
    request::build_object_discovery_user_prompt,
};

fn chapter_for_test() -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧书房".to_string()),
        content: "他在旧书房抽屉里摸到一枚青铜钥匙，钥匙柄部刻着残缺的星纹。".to_string(),
        word_count: 32,
        status: chapter::STATUS_DRAFT.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .into_iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).expect("test anchor id should fit i64"),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index)
                .expect("test sentence index should fit i32"),
            start_offset: i32::try_from(span.char_start).expect("test start offset should fit i32"),
            end_offset: i32::try_from(span.char_end).expect("test end offset should fit i32"),
            quote: span.text,
            quote_hash: format!("hash-{}", index + 1),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        })
        .collect()
}

fn anchor_id_containing(anchors: &[text_anchor::Model], text: &str) -> i64 {
    anchors
        .iter()
        .find(|anchor| anchor.quote.contains(text))
        .map(|anchor| anchor.id)
        .unwrap_or_else(|| panic!("missing text anchor containing {text}"))
}

fn canonical_object(anchor_ids: Vec<i64>) -> Value {
    json!({
        "name": "青铜钥匙",
        "aliases": ["旧钥匙"],
        "object_type": "token",
        "importance": "primary",
        "visual_description": "柄部刻着残缺星纹的青铜钥匙",
        "material_and_state": "青铜材质，边缘磨损",
        "story_usage": "开启后续密室的关键线索",
        "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
        "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
        "location_context": "旧书房抽屉里",
        "evidence_anchor_ids": anchor_ids,
    })
}

#[test]
fn parses_canonical_object_and_derives_system_fields() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let content = json!({"objects": [canonical_object(vec![anchor_id])]}).to_string();

    let drafts = parse_and_validate_object_discovery_content(
        &content,
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect("canonical output should parse");

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].draft.name, "青铜钥匙");
    assert_eq!(drafts[0].draft.object_type, "token");
    assert_eq!(drafts[0].draft.first_seen_chapter, 3);
    assert_eq!(drafts[0].draft.last_seen_chapter, 3);
    assert_eq!(
        drafts[0].draft.evidence[0],
        "他在旧书房抽屉里摸到一枚青铜钥匙"
    );
    assert!(
        drafts[0]
            .draft
            .image_prompt_seed
            .contains("柄部刻着残缺星纹")
    );
    assert!(
        drafts[0]
            .draft
            .image_prompt_seed
            .contains("约束青铜钥匙星纹、磨损边缘和青铜材质")
    );
}

#[test]
fn evidence_anchor_ids_preserve_exact_source_quote() {
    let mut chapter = chapter_for_test();
    chapter.content = "他用冷水冲洗左眼眶和左手残肢，疼痛让他更加清醒。然后他从柜子里翻出一卷医用绷带，熟练地缠绕起左手的断指处，又用新的绷带蒙住左眼。左手断指处生长出细密的肉芽。".to_string();
    let chapters = vec![chapter];
    let anchors = anchors_for_test(&chapters);
    let anchor_id = anchor_id_containing(&anchors, "新的绷带蒙住左眼");
    let mut object = canonical_object(vec![anchor_id]);
    object["name"] = json!("绷带");
    object["aliases"] = json!([]);
    object["object_type"] = json!("tool");
    let content = json!({"objects": [object]}).to_string();

    let drafts = parse_and_validate_object_discovery_content(
        &content,
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect("anchor id evidence should parse");

    assert_eq!(drafts[0].draft.evidence[0], "又用新的绷带蒙住左眼");
}

#[test]
fn derives_chapter_range_from_multiple_evidence_anchors() {
    let mut later = chapter_for_test();
    later.id = 2;
    later.chapter_number = 4;
    later.content = "第四章里，那枚青铜钥匙被放在石门凹槽前。".to_string();
    let chapters = vec![chapter_for_test(), later];
    let anchors = anchors_for_test(&chapters);
    let first = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let last = anchor_id_containing(&anchors, "那枚青铜钥匙");
    let content = json!({"objects": [canonical_object(vec![first, last])]}).to_string();

    let drafts = parse_and_validate_object_discovery_content(
        &content,
        &BTreeSet::from([3, 4]),
        &chapters,
        &anchors,
    )
    .expect("chapter range should derive from evidence anchors");

    assert_eq!(drafts[0].draft.first_seen_chapter, 3);
    assert_eq!(drafts[0].draft.last_seen_chapter, 4);
}

#[test]
fn rejects_noncanonical_object_type_and_importance() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let analyzed = BTreeSet::from([3]);
    let mut object = canonical_object(vec![anchor_id]);
    object["object_type"] = json!("key_item");
    let error = parse_and_validate_object_discovery_content(
        &json!({"objects": [object]}).to_string(),
        &analyzed,
        &chapters,
        &anchors,
    )
    .expect_err("object_type aliases should fail");
    assert!(error.contains("objects[0].object_type 无效"));

    let mut object = canonical_object(vec![anchor_id]);
    object["importance"] = json!("high");
    let error = parse_and_validate_object_discovery_content(
        &json!({"objects": [object]}).to_string(),
        &analyzed,
        &chapters,
        &anchors,
    )
    .expect_err("importance aliases should fail");
    assert!(error.contains("objects[0].importance 无效"));
}

#[test]
fn rejects_unknown_or_model_derived_fields() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let mut object = canonical_object(vec![anchor_id]);
    object["first_seen_chapter"] = json!(3);
    let error = parse_and_validate_object_discovery_content(
        &json!({"objects": [object]}).to_string(),
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect_err("model supplied derived field should fail");
    assert!(error.contains("未支持字段 `first_seen_chapter`"));
}

#[test]
fn rejects_unknown_evidence_anchor_id() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let content = json!({"objects": [canonical_object(vec![999])]}).to_string();
    let error = parse_and_validate_object_discovery_content(
        &content,
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect_err("unknown anchor should fail");
    assert!(error.contains("text_anchor_id 不在本次输入候选中"));
}

#[test]
fn rejects_duplicate_object_names() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let object = canonical_object(vec![anchor_id]);
    let content = json!({"objects": [object.clone(), object]}).to_string();
    let error = parse_and_validate_object_discovery_content(
        &content,
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect_err("duplicate object should fail");
    assert!(error.contains("objects[1] 与前序物体重复"));
}

#[test]
fn reports_truncated_json_explicitly() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let error = parse_and_validate_object_discovery_content(
        r#"{"objects":[{"name":"青铜钥匙"}"#,
        &BTreeSet::from([3]),
        &chapters,
        &anchors,
    )
    .expect_err("truncated JSON should fail");
    assert!(error.contains("响应在 JSON 闭合前结束"));
}

#[test]
fn user_prompt_is_json_contract_and_names_canonical_fields() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let prompt = build_object_discovery_user_prompt(&[], &[], &chapters, &anchors)
        .expect("prompt should build");
    assert!(prompt.contains("\"required_root_field\": \"objects\""));
    assert!(prompt.contains("\"evidence_anchor_ids\""));
    assert!(prompt.contains("\"object_type_values\""));
    assert!(!prompt.contains("\"first_seen_chapter\": 3"));
}
