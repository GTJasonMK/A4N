use super::support::{anchor_id_containing, anchors_for_test, chapter_for_test};
use crate::actions::object_discovery::parsing::parse_and_validate_object_discovery_content;
use std::collections::BTreeSet;

#[test]
fn rejects_object_type_and_importance_aliases() {
    let chapters = vec![chapter_for_test()];
    let analyzed = BTreeSet::from([3]);
    let anchors = anchors_for_test(&chapters);
    let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

    let content = serde_json::json!({
        "objects": [
            {
                "name": "青铜钥匙",
                "aliases": ["旧钥匙"],
                "object_type": "key_item",
                "importance": "high",
                "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                "material_and_state": "青铜材质，边缘磨损",
                "story_usage": "开启后续密室的关键线索",
                "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                "location_context": "旧书房抽屉里",
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("object enum aliases should fail visibly");

    assert!(err.contains("objects[0].object_type 无效"));

    let content = serde_json::json!({
        "objects": [
            {
                "name": "青铜钥匙",
                "aliases": ["旧钥匙"],
                "object_type": "token",
                "importance": "high",
                "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                "material_and_state": "青铜材质，边缘磨损",
                "story_usage": "开启后续密室的关键线索",
                "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                "location_context": "旧书房抽屉里",
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("importance aliases should fail visibly");

    assert!(err.contains("objects[0].importance 无效"));
}

#[test]
fn rejects_unknown_object_type() {
    let chapters = vec![chapter_for_test()];
    let analyzed = BTreeSet::from([3]);
    let anchors = anchors_for_test(&chapters);
    let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

    let content = serde_json::json!({
        "objects": [
            {
                "name": "青铜钥匙",
                "aliases": [],
                "object_type": "memory",
                "importance": "primary",
                "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                "material_and_state": "青铜材质",
                "story_usage": "开启后续密室的关键线索",
                "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                "location_context": null,
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("unknown object_type should fail");

    assert!(err.contains("objects[0].object_type 无效"));
    assert!(err.contains("token"));
}

#[test]
fn rejects_unknown_object_importance() {
    let chapters = vec![chapter_for_test()];
    let analyzed = BTreeSet::from([3]);
    let anchors = anchors_for_test(&chapters);
    let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

    let content = serde_json::json!({
        "objects": [
            {
                "name": "青铜钥匙",
                "aliases": [],
                "object_type": "token",
                "importance": "legendary",
                "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                "material_and_state": "青铜材质",
                "story_usage": "开启后续密室的关键线索",
                "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                "location_context": null,
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("unknown importance should fail");

    assert!(err.contains("objects[0].importance 无效"));
    assert!(err.contains("primary"));
}

#[test]
fn rejects_unknown_object_evidence_anchor_id() {
    let content = serde_json::json!({
        "objects": [
            {
                "name": "不存在的戒指",
                "aliases": [],
                "object_type": "token",
                "importance": "secondary",
                "visual_description": "银色戒指",
                "material_and_state": "银质",
                "story_usage": "线索",
                "visual_usage": "保持银色戒指作为线索物的画面连续性",
                "generation_usage": "约束银色戒指外观和银质材质",
                "location_context": null,
                "evidence_anchor_ids": [999]
            }
        ]
    })
    .to_string();
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let analyzed = BTreeSet::from([3]);

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("unknown anchor should fail");

    assert!(err.contains("evidence_anchor_ids 无效"));
    assert!(err.contains("text_anchor_id 不在本次输入候选中"));
}

#[test]
fn rejects_model_supplied_object_chapter_scope_fields() {
    let chapters = vec![chapter_for_test()];
    let anchors = anchors_for_test(&chapters);
    let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let content = serde_json::json!({
        "objects": [
            {
                "name": "青铜钥匙",
                "aliases": [],
                "object_type": "token",
                "importance": "primary",
                "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                "material_and_state": "青铜材质",
                "story_usage": "开启后续密室的关键线索",
                "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                "location_context": null,
                "first_seen_chapter": 3,
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();
    let analyzed = BTreeSet::from([3]);

    let err = parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
        .expect_err("model supplied derived chapter field should fail");

    assert!(err.contains("未支持字段 `first_seen_chapter`"));
}
