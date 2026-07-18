use super::super::super::*;
use super::super::support::{manifest_with_pack, pack_ref};

#[test]
fn manifest_render_skips_empty_stable_context_section() {
    let manifest = manifest_with_pack("当前任务");
    let selected_system_pack = PlannedContextPack {
        pack_ref: pack_ref("system_prompt_prefix", COMPRESSION_FULL_RAW, 10),
        stable_prefix: true,
        canonical_content_text: None,
    };

    let rendered = render_context_manifest_user_content(&manifest, &[selected_system_pack])
        .expect("rendered content");

    assert!(!rendered.contains("[稳定上下文]"));
    assert!(rendered.contains("[当前任务]"));
}

#[test]
fn manifest_render_separates_variable_context_from_stable_prefix() {
    let manifest = manifest_with_pack("当前任务");
    let stable_pack = PlannedContextPack {
        pack_ref: pack_ref("stable", COMPRESSION_FULL_RAW, 10),
        stable_prefix: true,
        canonical_content_text: Some("稳定内容".to_string()),
    };
    let variable_pack = PlannedContextPack {
        pack_ref: pack_ref("asset_binding", COMPRESSION_MINIMAL, 10),
        stable_prefix: false,
        canonical_content_text: Some("可变素材".to_string()),
    };

    let rendered = render_context_manifest_user_content(&manifest, &[stable_pack, variable_pack])
        .expect("rendered content");

    let stable_index = rendered.find("[稳定上下文]").expect("stable section");
    let variable_index = rendered.find("[可变上下文]").expect("variable section");
    let task_index = rendered.find("[当前任务]").expect("task section");
    assert!(stable_index < variable_index);
    assert!(variable_index < task_index);
    assert!(rendered.contains("稳定内容"));
    assert!(rendered.contains("可变素材"));
}
