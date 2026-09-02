use std::collections::HashMap;

use serde_json::json;

use crate::error::AppError;
use a4n_domain::project::ProjectStatus;

use super::{
    blueprint_refinement_business_key, blueprint_refinement_call_key,
    build_blueprint_generation_payload, increment_blueprint_version,
    normalized_blueprint_refinement_focus, project_status_after_blueprint_generation,
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
fn builds_canonical_blueprint_generation_payload() {
    let variables = HashMap::from([
        (
            "inspiration_transcript".to_string(),
            json!(["星潮", "档案城"]),
        ),
        (
            "generation_instruction".to_string(),
            json!("自由随机生成完整蓝图"),
        ),
        ("ignored".to_string(), json!("不应进入 payload")),
    ]);

    let payload = build_blueprint_generation_payload("星潮档案", "科幻", 60, &variables)
        .expect("valid generation payload");

    assert_eq!(payload["novel_title"], "星潮档案");
    assert_eq!(payload["genre"], "科幻");
    assert_eq!(payload["requested_chapter_count"], 60);
    assert_eq!(payload["inspiration_transcript"], json!(["星潮", "档案城"]));
    assert_eq!(payload["generation_instruction"], "自由随机生成完整蓝图");
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

#[test]
fn canonicalizes_blueprint_refinement_identity_and_focus() {
    assert_eq!(blueprint_refinement_business_key(42), "blueprint_refine:42");
    assert_eq!(
        blueprint_refinement_call_key(42),
        "blueprint_refine:project:42"
    );
    assert_eq!(
        normalized_blueprint_refinement_focus(Some(" 强化主角动机 ")),
        Some("强化主角动机")
    );
    assert_eq!(normalized_blueprint_refinement_focus(Some(" \n\t ")), None);
}
