use std::collections::HashMap;

use serde_json::json;

use crate::error::AppError;
use a4n_domain::project::ProjectStatus;

use super::{
    blueprint_stage_previous_context, build_blueprint_generation_payload,
    increment_blueprint_version, project_status_after_blueprint_generation,
};

#[test]
fn increments_major_minor_blueprint_versions() {
    assert_eq!(increment_blueprint_version("1.0").unwrap(), "1.1");
    assert_eq!(increment_blueprint_version("1.9").unwrap(), "1.10");
    assert_eq!(increment_blueprint_version("2.0").unwrap(), "2.1");
}

#[test]
fn rejects_invalid_blueprint_versions() {
    for current in ["1.0.0", "v1.0", "1.x", "-1.0", "1.-1", "1"] {
        assert!(matches!(
            increment_blueprint_version(current),
            Err(AppError::InvalidArgument(_))
        ));
    }
}

#[test]
fn blueprint_generation_status_never_rolls_project_back() {
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Draft),
        ProjectStatus::BlueprintReady
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::BlueprintReady),
        ProjectStatus::BlueprintReady
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Writing),
        ProjectStatus::Writing
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Completed),
        ProjectStatus::Completed
    );
}

#[test]
fn builds_previous_context_for_each_blueprint_stage() {
    let outputs = vec![
        json!({"title": "星潮档案"}),
        json!({"world_setting": {"premise": "潮汐城"}}),
        json!({"characters": [{"name": "林烬"}]}),
    ];

    assert_eq!(blueprint_stage_previous_context(0, &outputs).unwrap(), None);
    assert_eq!(
        blueprint_stage_previous_context(1, &outputs).unwrap(),
        Some(json!({"story_core": outputs[0]}))
    );
    assert_eq!(
        blueprint_stage_previous_context(2, &outputs).unwrap(),
        Some(json!({"story_core": outputs[0], "world": outputs[1]}))
    );
    assert_eq!(
        blueprint_stage_previous_context(3, &outputs).unwrap(),
        Some(json!({
            "story_core": outputs[0],
            "world": outputs[1],
            "characters": outputs[2],
        }))
    );
}

#[test]
fn rejects_missing_or_unknown_blueprint_stage_context() {
    let missing = blueprint_stage_previous_context(2, &[json!({"title": "星潮档案"})])
        .expect_err("characters stage requires world context");
    assert_eq!(missing.to_string(), "参数无效：角色阶段缺少世界观上下文");

    let unknown =
        blueprint_stage_previous_context(4, &[]).expect_err("unknown stage index must fail");
    assert_eq!(unknown.to_string(), "参数无效：未知蓝图生成阶段索引: 4");
}

#[test]
fn builds_canonical_blueprint_generation_payload() {
    let variables = HashMap::from([
        (
            "inspiration_transcript".to_string(),
            json!(["星潮", "档案城"]),
        ),
        ("ignored".to_string(), json!("不应进入 payload")),
    ]);

    let payload = build_blueprint_generation_payload("星潮档案", "科幻", 60, &variables)
        .expect("valid generation payload");

    assert_eq!(payload["novel_title"], "星潮档案");
    assert_eq!(payload["genre"], "科幻");
    assert_eq!(payload["requested_chapter_count"], 60);
    assert_eq!(payload["inspiration_transcript"], json!(["星潮", "档案城"]));
    assert!(payload.get("ignored").is_none());
}

#[test]
fn rejects_blueprint_chapter_count_outside_contract() {
    for count in [4, 10_001] {
        let error = build_blueprint_generation_payload("星潮档案", "科幻", count, &HashMap::new())
            .expect_err("out-of-range count must fail");
        assert_eq!(
            error.to_string(),
            "参数无效：requested_chapter_count 必须在 5 到 10000 之间"
        );
    }
}
