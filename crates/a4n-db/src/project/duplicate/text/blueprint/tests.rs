use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};
use chrono::NaiveDateTime;

use super::{ProjectBlueprintGraph, project_blueprint_graph_raw_blueprint};

fn blueprint_graph() -> ProjectBlueprintGraph {
    let timestamp = NaiveDateTime::MIN;
    ProjectBlueprintGraph {
        blueprint: novel_blueprint::Model {
            id: 1,
            project_id: 2,
            title: "蓝图标题".to_string(),
            genre: "奇幻".to_string(),
            style: Some("史诗".to_string()),
            tone: Some("明亮".to_string()),
            target_audience: Some("青年".to_string()),
            one_sentence_summary: Some("一句话梗概".to_string()),
            full_synopsis: Some("完整梗概".to_string()),
            world_setting: Some(serde_json::json!({"continent": "A"})),
            current_version: "2.0".to_string(),
            needs_part_outlines: true,
            total_chapters: Some(24),
            chapters_per_part: Some(8),
            avatar_svg: Some("<svg />".to_string()),
            avatar_animal: Some("fox".to_string()),
            created_at: timestamp,
            updated_at: timestamp,
        },
        versions: Vec::new(),
        characters: vec![blueprint_character::Model {
            id: 3,
            project_id: 2,
            name: "角色甲".to_string(),
            character_type: "protagonist".to_string(),
            gender: Some("女".to_string()),
            age: Some("20".to_string()),
            appearance: Some("银发".to_string()),
            personality: Some("坚定".to_string()),
            background: Some("边境出身".to_string()),
            motivation: Some("寻找真相".to_string()),
            arc_summary: Some("学会信任".to_string()),
            relationship_to_protagonist: None,
            arc_stages: Some(serde_json::json!(["拒绝", "接受"])),
            created_at: timestamp,
            updated_at: timestamp,
        }],
        relationships: vec![blueprint_relationship::Model {
            id: 4,
            project_id: 2,
            relationship_type: "mentor".to_string(),
            character_a: "角色甲".to_string(),
            character_b: "角色乙".to_string(),
            description: Some("师徒".to_string()),
            strength: Some(8),
            evolution: Some(serde_json::json!(["疏离", "信任"])),
            created_at: timestamp,
        }],
    }
}

#[test]
fn raw_blueprint_projects_story_contract() -> Result<(), Box<dyn std::error::Error>> {
    let raw = project_blueprint_graph_raw_blueprint(&blueprint_graph())?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;

    assert_eq!(value["blueprint"]["title"], "蓝图标题");
    assert_eq!(value["blueprint"]["world_setting"]["continent"], "A");
    assert_eq!(value["characters"][0]["name"], "角色甲");
    assert_eq!(value["characters"][0]["arc_stages"][1], "接受");
    assert_eq!(value["relationships"][0]["relationship_type"], "mentor");
    assert_eq!(value["relationships"][0]["strength"], 8);
    Ok(())
}

#[test]
fn raw_blueprint_excludes_storage_only_fields() -> Result<(), Box<dyn std::error::Error>> {
    let raw = project_blueprint_graph_raw_blueprint(&blueprint_graph())?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;

    assert!(value.get("versions").is_none());
    assert!(value["blueprint"].get("current_version").is_none());
    assert!(value["blueprint"].get("avatar_svg").is_none());
    assert!(value["blueprint"].get("avatar_animal").is_none());
    Ok(())
}
