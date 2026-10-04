use super::*;
#[test]
fn validate_package_manifest_rejects_manga_layout_unknown_field() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapters = vec![chapter_record(11, 1, None)];
    let mut layout = valid_manga_layout(11);
    layout["unknown"] = json!(true);
    package.chapter_manga_prompts = vec![manga_prompt(21, 1, 11, layout)];

    let error = validate_package_manifest(&package)
        .expect_err("canonical manga layout must reject unknown fields");

    assert!(error.contains("v3 layout 无效"));
    assert!(error.contains("unknown field"));
}

#[test]
fn validate_package_manifest_rejects_manga_layout_missing_adaptation_plan() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapters = vec![chapter_record(11, 1, None)];
    package.chapter_manga_prompts = vec![manga_prompt(21, 1, 11, valid_manga_layout(11))];

    let error = validate_package_manifest(&package)
        .expect_err("manga layout must reference its packaged adaptation plan");

    assert!(error.contains("layout 引用了不存在的改编计划"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_v4_entity_ids() {
    let mut package = package_with_derived_project();
    package.generated_images = vec![generated_image(30, 2, None), generated_image(30, 2, None)];

    let error =
        validate_package_manifest(&package).expect_err("v4 generated image ids must remain unique");

    assert!(error.contains("generated_images.id"));
}

#[test]
fn validate_package_manifest_rejects_generated_image_runtime_identity() {
    let mut package = package_with_derived_project();
    let mut image = generated_image(30, 2, None);
    image.metadata_json = Some(json!({"provider_job_id": "remote-job"}));
    package.generated_images = vec![image];

    let error = validate_package_manifest(&package).expect_err("runtime identity must fail");

    assert!(error.contains("metadata_json 不能携带运行身份 provider_job_id"));
}

#[test]
fn validate_package_manifest_rejects_cross_project_portrait_profile() {
    let mut package = package_with_derived_project();
    package.text_core.character_profiles = vec![character_profile_record(40)];
    package.character_portraits = vec![character_portrait::Model {
        id: 41,
        profile_id: 40,
        project_id: 2,
        portrait_type: "full_body".to_string(),
        image_path: "images/portrait.png".to_string(),
        prompt: "portrait".to_string(),
        expression: None,
        pose: None,
        is_primary: true,
        created_at: dt(),
    }];

    let error = validate_package_manifest(&package)
        .expect_err("portrait and profile must belong to the same project");

    assert!(error.contains("角色立绘与角色档案跨项目"));
}

fn manga_prompt(
    id: i64,
    project_id: i64,
    chapter_id: i64,
    layout: Value,
) -> chapter_manga_prompt::Model {
    chapter_manga_prompt::Model {
        id,
        chapter_id,
        project_id,
        page_number: 1,
        prompt_type: "page_generation".to_string(),
        image_prompt: "full page".to_string(),
        dialogue_text: None,
        characters_in_frame: Some(json!([])),
        layout: Some(layout),
        generated_image_id: None,
        status: "pending".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn generated_image(id: i64, project_id: i64, chapter_id: Option<i64>) -> generated_image::Model {
    generated_image::Model {
        id,
        project_id,
        image_type: "manga_page".to_string(),
        file_path: format!("images/{id}.png"),
        prompt: "full page".to_string(),
        model: "test".to_string(),
        width: 1024,
        height: 1536,
        file_size: Some(3),
        ai_score: None,
        is_favorite: false,
        chapter_id,
        panel_id: None,
        metadata_json: None,
        created_at: dt(),
    }
}

fn valid_manga_layout(chapter_id: i64) -> Value {
    json!({
        "schema_version": "manga_page_generation_v3",
        "page_plan": {
            "page_number": 1,
            "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
            "role": "setup",
            "pacing": "medium",
            "importance": "normal",
            "reading_flow": "top_to_bottom_left_to_right",
            "page_goal": "推进场景",
            "visual_thesis": "角色对峙",
            "entry_state": "平静",
            "exit_state": "紧张",
            "dominant_panel_number": 1,
            "panels": [{
                "panel_number": 1,
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                "function": "dialogue",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": ["hero"],
                "composition": "单人构图",
                "observable_action": "抬头",
                "reaction": null,
                "spatial_relationship": "中央",
                "continuity_from_previous": null,
                "text_layers": []
            }]
        },
        "drawing_prompt": {
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": 1,
            "image_prompt": "整页"
        },
        "reference_bindings": [],
        "prompt_fingerprint": {"hash": "sha256"},
        "source": {
            "root_project_id": 1,
            "source_chapter_id": chapter_id,
            "source_snapshot_id": null,
            "source_snapshot_chapter_id": null,
            "source_revision_key": "revision:1",
            "content_sha256": "content-sha256",
            "adaptation_plan_id": 71,
            "adaptation_plan_revision": 1,
            "adaptation_plan_sha256": "plan-sha256"
        }
    })
}
