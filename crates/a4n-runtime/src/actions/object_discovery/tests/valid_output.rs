use super::support::{anchor_id_containing, anchors_for_test, chapter_for_test};
use crate::actions::object_discovery::parsing::parse_and_validate_object_discovery_content;
use std::collections::BTreeSet;

#[test]
fn parses_valid_object_discovery_output() {
    let chapters = vec![chapter_for_test()];
    let analyzed = BTreeSet::from([3]);
    let anchors = anchors_for_test(&chapters);
    let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

    let content = serde_json::json!({
        "objects": [
            {
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
                "evidence_anchor_ids": [key_anchor_id]
            }
        ]
    })
    .to_string();

    let drafts =
        parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
            .expect("valid object discovery output should parse");

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].draft.name, "青铜钥匙");
    assert_eq!(drafts[0].draft.object_type, "token");
    assert_eq!(
        drafts[0].draft.location_context.as_deref(),
        Some("旧书房抽屉里")
    );
    assert_eq!(drafts[0].draft.first_seen_chapter, 3);
    assert_eq!(drafts[0].draft.last_seen_chapter, 3);
    assert_eq!(
        drafts[0].draft.evidence[0],
        "他在旧书房抽屉里摸到一枚青铜钥匙"
    );
    assert!(drafts[0].draft.image_prompt_seed.contains("青铜钥匙"));
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
fn object_evidence_anchor_ids_avoid_rewritten_source_text() {
    let mut chapter = chapter_for_test();
    chapter.content = "他用冷水冲洗左眼眶和左手残肢，疼痛让他更加清醒。然后他从柜子里翻出一卷医用绷带，熟练地缠绕起左手的断指处，又用新的绷带蒙住左眼。左手断指处生长出细密的肉芽。".to_string();
    let chapters = vec![chapter];
    let anchors = anchors_for_test(&chapters);
    let bandage_anchor_id = anchor_id_containing(&anchors, "新的绷带蒙住左眼");
    let content = serde_json::json!({
        "objects": [
            {
                "name": "绷带",
                "aliases": [],
                "object_type": "tool",
                "importance": "secondary",
                "visual_description": "用于包扎左眼和左手伤处的医用绷带",
                "material_and_state": "新的医用绷带",
                "story_usage": "遮盖伤口并维持角色伤势视觉连续性",
                "visual_usage": "绑定左眼和左手伤处的包扎状态",
                "generation_usage": "约束医用绷带外观、缠绕位置和遮盖关系",
                "location_context": null,
                "evidence_anchor_ids": [bandage_anchor_id]
            }
        ]
    })
    .to_string();
    let analyzed = BTreeSet::from([3]);

    let drafts =
        parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
            .expect("anchor id evidence should parse");

    assert_eq!(drafts[0].draft.evidence[0], "又用新的绷带蒙住左眼");
}

#[test]
fn derives_object_chapter_scope_from_evidence_anchor_ids() {
    let mut later_chapter = chapter_for_test();
    later_chapter.id = 2;
    later_chapter.chapter_number = 4;
    later_chapter.content = "第四章里，那枚青铜钥匙被放在石门凹槽前。".to_string();
    let chapters = vec![chapter_for_test(), later_chapter];
    let anchors = anchors_for_test(&chapters);
    let first_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
    let later_anchor_id = anchor_id_containing(&anchors, "那枚青铜钥匙");
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
                "location_context": "旧书房抽屉与石门凹槽前",
                "evidence_anchor_ids": [first_anchor_id, later_anchor_id]
            }
        ]
    })
    .to_string();
    let analyzed = BTreeSet::from([3, 4]);

    let drafts =
        parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
            .expect("object chapter scope should be derived from evidence anchors");

    assert_eq!(drafts[0].draft.first_seen_chapter, 3);
    assert_eq!(drafts[0].draft.last_seen_chapter, 4);
}
