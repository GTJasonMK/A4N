use std::collections::HashMap;

use crate::project::duplicate::ProjectDuplicateError;

use super::rules::remap_derived_media_binding_target;

#[test]
fn media_binding_target_remaps_derived_graph_ids() {
    let segments = HashMap::from([(10, 110)]);
    let timelines = HashMap::from([(20, 120)]);
    let videos = HashMap::from([(30, 130)]);

    assert_eq!(
        remap_derived_media_binding_target(7, "video_segment", 10, &segments, &timelines, None)
            .unwrap(),
        110
    );
    assert_eq!(
        remap_derived_media_binding_target(
            7,
            "assembly_timeline",
            20,
            &segments,
            &timelines,
            None,
        )
        .unwrap(),
        120
    );
    assert_eq!(
        remap_derived_media_binding_target(
            7,
            "generated_video",
            30,
            &segments,
            &timelines,
            Some(&videos),
        )
        .unwrap(),
        130
    );
}

#[test]
fn media_binding_target_preserves_authoritative_entity_ids() {
    for target_kind in [
        "character_profile",
        "object_profile",
        "scene_profile",
        "chapter",
    ] {
        assert_eq!(
            remap_derived_media_binding_target(
                7,
                target_kind,
                42,
                &HashMap::new(),
                &HashMap::new(),
                None,
            )
            .unwrap(),
            42
        );
    }
}

#[test]
fn media_binding_target_rejects_missing_video_map_and_unknown_kind() {
    let missing_video_map = remap_derived_media_binding_target(
        7,
        "generated_video",
        30,
        &HashMap::new(),
        &HashMap::new(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        missing_video_map,
        ProjectDuplicateError::InvalidState { current, operation }
            if current == "source_project_id=7 缺少 generated_video 映射"
                && operation == "复制 generated_video 媒体绑定"
    ));

    let unsupported = remap_derived_media_binding_target(
        7,
        "unknown",
        30,
        &HashMap::new(),
        &HashMap::new(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        unsupported,
        ProjectDuplicateError::InvalidArgument(message)
            if message == "不支持复制的媒体绑定目标类型: source_project_id=7, target_kind=unknown"
    ));
}
