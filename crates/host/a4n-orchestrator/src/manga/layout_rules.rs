//! 漫画页面 layout JSON 纯字段规则。

use a4n_domain::{
    DownstreamConflictStrategy,
    manga::{
        MangaBoundReferenceAsset, MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4,
        MangaPagePromptRecord, MangaReferenceBindingSummary,
    },
};

pub(crate) fn manga_page_generation_layout(
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPageGenerationLayoutV3, String> {
    a4n_manga_core::manga_page_generation_layout(
        prompt.id,
        prompt.page_number,
        prompt.layout.as_ref(),
    )
}

pub(crate) fn manga_page_pipeline_layout(
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPageGenerationLayoutV4, String> {
    a4n_manga_core::manga_page_pipeline_layout(
        prompt.id,
        prompt.page_number,
        prompt.layout.as_ref(),
    )
}

fn bind_reference_asset_in_bindings(
    bindings: &mut [MangaReferenceBindingSummary],
    character_name: &str,
    bound_asset: MangaBoundReferenceAsset,
) -> Result<(), String> {
    for (index, binding) in bindings.iter_mut().enumerate() {
        let current_character_name = binding.character_name.trim();
        if current_character_name.is_empty() {
            return Err(format!(
                "整页漫画提示词 reference_bindings[{index}].character_name 必须是非空字符串"
            ));
        }
        if current_character_name != character_name {
            continue;
        }
        binding.bound_asset = Some(bound_asset);
        return Ok(());
    }

    Err(format!("整页漫画提示词不存在角色参考图: {character_name}"))
}

/// 将参考素材绑定进整页漫画 layout 的指定角色。
pub(crate) fn bind_reference_asset_in_layout(
    mut layout: MangaPageGenerationLayoutV3,
    character_name: &str,
    bound_asset: MangaBoundReferenceAsset,
) -> Result<MangaPageGenerationLayoutV3, String> {
    bind_reference_asset_in_bindings(&mut layout.reference_bindings, character_name, bound_asset)?;
    Ok(layout)
}

/// 将参考素材绑定进新流水线 layout，同时保留页面 checkpoint。
pub(crate) fn bind_reference_asset_in_pipeline_layout(
    mut layout: MangaPageGenerationLayoutV4,
    character_name: &str,
    bound_asset: MangaBoundReferenceAsset,
) -> Result<MangaPageGenerationLayoutV4, String> {
    bind_reference_asset_in_bindings(&mut layout.reference_bindings, character_name, bound_asset)?;
    Ok(layout)
}

/// 确定整页漫画提示词变更前的下游图片处理。
pub(crate) fn resolve_manga_prompt_downstream_conflict(
    page_prompt: &MangaPagePromptRecord,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Option<i64>, String> {
    let Some(image_id) = page_prompt.generated_image_id else {
        return Ok(None);
    };
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "整页漫画提示词 #{} 已派生成图片 #{image_id}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源提示词",
            page_prompt.id
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(None),
        DownstreamConflictStrategy::DeleteDownstream => Ok(Some(image_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::manga::{
        MangaPageGenerationLayoutV4, MangaPageGenerationSchemaVersion, MangaReferenceAssetKind,
        MangaReferenceBindingSummary,
    };
    use serde_json::json;

    #[test]
    fn bind_updates_existing_character() {
        let layout = layout_fixture(vec![MangaReferenceBindingSummary {
            character_name: "hero".to_string(),
            bound_asset: None,
        }]);
        let bound = MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let updated = bind_reference_asset_in_layout(layout, "hero", bound).unwrap();
        assert_eq!(
            updated.reference_bindings[0]
                .bound_asset
                .as_ref()
                .expect("bound asset")
                .asset_id,
            7
        );
    }

    #[test]
    fn bind_rejects_empty_character_and_unknown_character() {
        let bound = || MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let err = bind_reference_asset_in_layout(
            layout_fixture(vec![MangaReferenceBindingSummary {
                character_name: "".to_string(),
                bound_asset: None,
            }]),
            "hero",
            bound(),
        )
        .unwrap_err();
        assert!(err.contains("character_name 必须是非空字符串"));
        let err = bind_reference_asset_in_layout(
            layout_fixture(vec![MangaReferenceBindingSummary {
                character_name: "other".to_string(),
                bound_asset: None,
            }]),
            "hero",
            bound(),
        )
        .unwrap_err();
        assert_eq!(err, "整页漫画提示词不存在角色参考图: hero");
    }

    #[test]
    fn bind_updates_v4_without_downgrading_pipeline() {
        let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(json!({
            "schema_version": "manga_page_generation_v4",
            "page_id": "page-0001",
            "page_number": 1,
            "page_plan_ref": null,
            "pipeline": {
                "schema_version": "manga_page_pipeline_state_v1",
                "page_id": "page-0001",
                "status": "pending",
                "stage": "adaptation",
                "attempt": 1,
                "source_fingerprint": "source",
                "stage_fingerprints": {},
                "page_content": null,
                "page_adaptation": null,
                "storyboard": null,
                "drawing_prompt": null,
                "image": null,
                "error": null
            },
            "reference_bindings": [{"character_name": "hero", "bound_asset": null}],
            "prompt_fingerprint": {},
            "source": {
                "root_project_id": 33,
                "source_chapter_id": 22,
                "source_snapshot_id": null,
                "source_snapshot_chapter_id": null,
                "source_revision_key": "content",
                "content_sha256": "content-sha256",
                "adaptation_plan_id": 71,
                "adaptation_plan_revision": 2,
                "adaptation_plan_sha256": "plan-sha256"
            }
        }))
        .expect("valid v4 layout fixture");
        let bound = MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };

        let updated = bind_reference_asset_in_pipeline_layout(layout, "hero", bound).unwrap();

        assert_eq!(updated.schema_version, MangaPageGenerationSchemaVersion::V4);
        assert_eq!(updated.pipeline.page_id, "page-0001");
        assert_eq!(
            updated.reference_bindings[0]
                .bound_asset
                .as_ref()
                .unwrap()
                .asset_id,
            7
        );
    }

    fn layout_fixture(
        reference_bindings: Vec<MangaReferenceBindingSummary>,
    ) -> MangaPageGenerationLayoutV3 {
        serde_json::from_value(json!({
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
            "reference_bindings": reference_bindings,
            "prompt_fingerprint": {"hash": "sha256"},
            "source": {
                "root_project_id": 33,
                "source_chapter_id": 22,
                "source_snapshot_id": null,
                "source_snapshot_chapter_id": null,
                "source_revision_key": "chapter:22:revision:1",
                "content_sha256": "content-sha256",
                "adaptation_plan_id": 71,
                "adaptation_plan_revision": 2,
                "adaptation_plan_sha256": "plan-sha256"
            }
        }))
        .expect("valid layout fixture")
    }
}
