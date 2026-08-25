use a4n_core::rag::RagDataType;
use a4n_db::rag::{RagSceneOccurrenceRow, RagSceneProfileRow, RagSceneTransitionRow};
use a4n_orchestrator::rag::{
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};
use serde_json::json;

fn sample_profile() -> RagSceneProfileRow {
    RagSceneProfileRow {
        id: 11,
        name: "北城门".to_string(),
        aliases_json: json!(["北门"]),
        scene_type: "exterior".to_string(),
        importance: "major".to_string(),
        base_description: "边境城北侧主门".to_string(),
        visual_description: "高墙与铁门".to_string(),
        spatial_layout: "城内外由门洞连接".to_string(),
        atmosphere: "戒备森严".to_string(),
        entry_points_json: json!(["门洞"]),
        rules_json: json!(["夜间关闭"]),
        current_state_json: json!({"status": "封锁"}),
        first_seen_chapter: 2,
        last_seen_chapter: 8,
        evidence_json: json!([{"chapter": 2}]),
    }
}

fn sample_occurrence() -> RagSceneOccurrenceRow {
    RagSceneOccurrenceRow {
        id: 21,
        scene_profile_id: 11,
        chapter_number: 3,
        occurrence_order: 2,
        timeline_position: Some("黄昏".to_string()),
        summary: "主角尝试出城".to_string(),
        mood: Some("紧张".to_string()),
        weather_or_lighting: Some("冷雨".to_string()),
        characters_json: json!(["林舟"]),
        objects_json: json!(["通行证"]),
        key_events_json: json!(["守卫拦截"]),
        entry_state_json: json!({"gate": "open"}),
        exit_state_json: json!({"gate": "closed"}),
    }
}

fn sample_transition() -> RagSceneTransitionRow {
    RagSceneTransitionRow {
        id: 31,
        chapter_number: 3,
        from_scene_profile_id: None,
        to_scene_profile_id: 11,
        transition_order: 1,
        movement_type: "walk".to_string(),
        route_description: "沿主街抵达北门".to_string(),
        time_cost: Some("十分钟".to_string()),
        distance_hint: Some("一公里".to_string()),
        participants_json: json!(["林舟"]),
        object_refs_json: json!(["通行证"]),
    }
}

#[test]
fn scene_profile_record_preserves_canonical_text_and_metadata() {
    let profile = sample_profile();
    let record = scene_profile_rag_record(9, &profile).expect("scene profile record");

    assert_eq!(record.chunk_id, "scene_profile:9:11");
    assert!(record.text.contains("场景: 北城门"));
    assert!(record.text.contains("章节范围: 第2章-第8章"));
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::SceneProfile.as_str()
    );
    assert_eq!(record.metadata["name"], "北城门");
}

#[test]
fn scene_occurrence_record_rejects_blank_scene_name() {
    assert!(scene_occurrence_rag_record(9, &sample_occurrence(), "  ").is_err());
}

#[test]
fn scene_occurrence_record_preserves_order_and_metadata() {
    let occurrence = sample_occurrence();
    let record =
        scene_occurrence_rag_record(9, &occurrence, "北城门").expect("scene occurrence record");

    assert_eq!(record.chunk_id, "scene_occurrence:9:21");
    assert!(record.text.contains("章节: 第3章"));
    assert_eq!(record.metadata["chapter_number"], 3);
}

#[test]
fn scene_transition_record_preserves_default_source_target_and_metadata() {
    let transition = sample_transition();
    let record = scene_transition_rag_record(9, &transition, None, "北城门")
        .expect("scene transition record");

    assert_eq!(record.chunk_id, "scene_transition:9:31");
    assert!(record.text.contains("来源场景: 未记录/章节开头"));
    assert!(record.text.contains("目标场景: 北城门"));
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::SceneTransition.as_str()
    );
    assert_eq!(record.metadata["chapter_number"], 3);
}
