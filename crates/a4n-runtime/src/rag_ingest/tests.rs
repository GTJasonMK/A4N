mod blueprint_contract;
mod character_contract;
mod expected_records_contract;
mod outline_contract;
mod sync_contract;

use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_core::models::character::{
    character_state_index, foreshadowing_index, protagonist_profile,
};
use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::json;

fn test_embedding_identity(model: &str) -> EmbeddingModelIdentity {
    EmbeddingModelIdentity {
        backend: "LocalHash".to_string(),
        model: model.to_string(),
        dimension: 2,
    }
}

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid test date")
        .and_hms_opt(0, 0, 0)
        .expect("valid test time")
}

fn sample_outline() -> chapter_outline::Model {
    chapter_outline::Model {
        id: 10,
        project_id: 1,
        part_id: Some(2),
        chapter_number: 3,
        title: Some("入城".to_string()),
        summary: Some("主角进入边境城。".to_string()),
        beats: Some(json!(["抵达", "发现异常"])),
        key_scenes: Some(json!([{"title": "城门", "description": "守卫盘查"}])),
        pacing_score: Some(0.8),
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: Some(3000),
        status: "planned".to_string(),
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_chapter(status: &str, content: &str) -> chapter::Model {
    chapter::Model {
        id: 90,
        project_id: 1,
        outline_id: None,
        chapter_number: 5,
        title: Some("旧城回声".to_string()),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: Some("主角听见旧城广播。".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_part() -> part_outline::Model {
    part_outline::Model {
        id: 20,
        project_id: 1,
        part_number: 1,
        title: "第一部".to_string(),
        start_chapter: 1,
        end_chapter: 8,
        summary: Some("主角建立目标并进入冲突。".to_string()),
        theme: Some("选择".to_string()),
        key_events: Some(json!(["离乡", "结盟"])),
        character_focus: Some(json!(["林舟"])),
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_blueprint() -> novel_blueprint::Model {
    novel_blueprint::Model {
        id: 30,
        project_id: 1,
        title: "星门余烬".to_string(),
        genre: "科幻".to_string(),
        style: Some("冷峻".to_string()),
        tone: Some("克制".to_string()),
        target_audience: Some("成年读者".to_string()),
        one_sentence_summary: Some("幸存者寻找失落星门。".to_string()),
        full_synopsis: Some("主角沿废弃航线追索星门真相。".to_string()),
        world_setting: Some(json!({"premise": "星门网络崩塌"})),
        current_version: "1.0.0".to_string(),
        needs_part_outlines: true,
        total_chapters: Some(24),
        chapters_per_part: Some(8),
        avatar_svg: None,
        avatar_animal: None,
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_profile() -> protagonist_profile::Model {
    protagonist_profile::Model {
        id: 40,
        project_id: 1,
        name: "林舟".to_string(),
        gender: Some("女".to_string()),
        age: Some("29".to_string()),
        base_attributes: json!({"desire": "自由"}),
        current_state: json!({"location": "边境城"}),
        character_arc: Some(json!({"start": "逃避", "end": "承担"})),
        voice_fingerprint: Some(json!({"voice_style": "简短直接"})),
        appearance: Some("短发，旧外套".to_string()),
        background: Some("前领航员".to_string()),
        personality: Some("谨慎".to_string()),
        last_synced_chapter: 0,
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_state_index() -> character_state_index::Model {
    character_state_index::Model {
        id: 50,
        profile_id: 40,
        chapter_number: 3,
        state_summary: "确认星门坐标可能仍有效。".to_string(),
        key_attributes: json!({"trust": "低"}),
        emotional_state: Some("警惕".to_string()),
        current_location: Some("边境城".to_string()),
        current_goal: Some("找到旧导航图".to_string()),
        created_at: ts(),
    }
}

fn sample_relationship() -> blueprint_relationship::Model {
    blueprint_relationship::Model {
        id: 60,
        project_id: 1,
        relationship_type: "ally".to_string(),
        character_a: "林舟".to_string(),
        character_b: "沈砚".to_string(),
        description: Some("互相利用，逐步建立信任。".to_string()),
        strength: Some(6),
        evolution: Some(json!([{"event": "共同脱险", "impact": "信任上升"}])),
        created_at: ts(),
    }
}

fn sample_blueprint_character() -> blueprint_character::Model {
    blueprint_character::Model {
        id: 70,
        project_id: 1,
        name: "沈砚".to_string(),
        character_type: "supporting".to_string(),
        gender: Some("男".to_string()),
        age: Some("34".to_string()),
        appearance: Some("灰色风衣".to_string()),
        personality: Some("克制".to_string()),
        background: Some("旧航线调查员".to_string()),
        motivation: Some("查清星门事故".to_string()),
        arc_summary: Some("从旁观到介入".to_string()),
        relationship_to_protagonist: Some("临时盟友".to_string()),
        arc_stages: Some(json!([{"turning_point": "共同脱险"}])),
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_foreshadowing() -> foreshadowing_index::Model {
    foreshadowing_index::Model {
        id: 80,
        project_id: 1,
        description: "旧导航图上的缺口指向被删除的星门坐标。".to_string(),
        setup_chapter: 3,
        payoff_chapter: Some(12),
        status: "pending".to_string(),
        related_characters: Some(json!(["林舟", {"name": "沈砚", "role": "知情者"}])),
        importance: 5,
        created_at: ts(),
        updated_at: ts(),
    }
}
