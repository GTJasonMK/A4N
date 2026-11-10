//! 漫画页面 layout JSON 纯字段规则。

use a4n_domain::manga::{
    MangaBoundReferenceAsset, MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4,
    MangaPagePromptRecord, MangaReferenceAssetKind, MangaReferenceBindingAction,
    MangaReferenceBindingSummary,
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

fn mutate_reference_asset_in_bindings(
    bindings: &mut [MangaReferenceBindingSummary],
    character_name: &str,
    asset_kind: MangaReferenceAssetKind,
    asset_id: i64,
    action: MangaReferenceBindingAction,
    asset: Option<MangaBoundReferenceAsset>,
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
        match action {
            MangaReferenceBindingAction::AddUserImage => {
                let asset = asset.ok_or_else(|| "添加用户参考图时必须提供素材".to_string())?;
                let is_system_asset = binding.bound_asset.as_ref().is_some_and(|current| {
                    current.asset_kind == asset.asset_kind && current.asset_id == asset.asset_id
                });
                if !is_system_asset
                    && !binding.user_added_assets.iter().any(|current| {
                        current.asset_kind == asset.asset_kind && current.asset_id == asset.asset_id
                    })
                {
                    binding.user_added_assets.push(asset);
                }
            }
            MangaReferenceBindingAction::RemoveUserImage => {
                if binding.bound_asset.as_ref().is_some_and(|current| {
                    current.asset_kind == asset_kind && current.asset_id == asset_id
                }) {
                    if binding.bound_asset.as_ref().is_some_and(|current| {
                        current.asset_kind == MangaReferenceAssetKind::CharacterPortrait
                    }) {
                        return Err(format!("角色 {character_name} 的系统参考图不能移除"));
                    }
                    binding.bound_asset = None;
                }
                binding.user_added_assets.retain(|current| {
                    current.asset_kind != asset_kind || current.asset_id != asset_id
                });
            }
        }
        return Ok(());
    }

    Err(format!("整页漫画提示词不存在角色参考图: {character_name}"))
}

/// 将参考素材绑定进新流水线 layout，同时保留页面 checkpoint。
pub(crate) fn mutate_reference_asset_in_pipeline_layout(
    mut layout: MangaPageGenerationLayoutV4,
    character_name: &str,
    asset_kind: MangaReferenceAssetKind,
    asset_id: i64,
    action: MangaReferenceBindingAction,
    asset: Option<MangaBoundReferenceAsset>,
) -> Result<MangaPageGenerationLayoutV4, String> {
    mutate_reference_asset_in_bindings(
        &mut layout.reference_bindings,
        character_name,
        asset_kind,
        asset_id,
        action,
        asset,
    )?;
    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::manga::{
        MangaPageGenerationLayoutV4, MangaPageGenerationSchemaVersion, MangaPagePipelineStage,
        MangaReferenceAssetKind, MangaReferenceBindingSummary,
    };
    use serde_json::json;

    #[test]
    fn add_user_reference_to_existing_character() {
        let layout = layout_fixture(vec![MangaReferenceBindingSummary {
            character_name: "hero".to_string(),
            bound_asset: None,
            user_added_assets: Vec::new(),
        }]);
        let bound = MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let updated = mutate_reference_asset_in_pipeline_layout(
            layout,
            "hero",
            MangaReferenceAssetKind::GeneratedImage,
            7,
            MangaReferenceBindingAction::AddUserImage,
            Some(bound),
        )
        .unwrap();
        assert_eq!(updated.reference_bindings[0].bound_asset, None);
        assert_eq!(
            updated.reference_bindings[0].user_added_assets[0].asset_id,
            7
        );
    }

    #[test]
    fn user_reference_mutation_rejects_empty_and_unknown_characters() {
        let bound = || MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let err = mutate_reference_asset_in_pipeline_layout(
            layout_fixture(vec![MangaReferenceBindingSummary {
                character_name: "".to_string(),
                bound_asset: None,
                user_added_assets: Vec::new(),
            }]),
            "hero",
            MangaReferenceAssetKind::GeneratedImage,
            7,
            MangaReferenceBindingAction::AddUserImage,
            Some(bound()),
        )
        .unwrap_err();
        assert!(err.contains("character_name 必须是非空字符串"));
        let err = mutate_reference_asset_in_pipeline_layout(
            layout_fixture(vec![MangaReferenceBindingSummary {
                character_name: "other".to_string(),
                bound_asset: None,
                user_added_assets: Vec::new(),
            }]),
            "hero",
            MangaReferenceAssetKind::GeneratedImage,
            7,
            MangaReferenceBindingAction::AddUserImage,
            Some(bound()),
        )
        .unwrap_err();
        assert_eq!(err, "整页漫画提示词不存在角色参考图: hero");
    }

    #[test]
    fn system_portrait_cannot_be_removed_as_user_reference() {
        let err = mutate_reference_asset_in_pipeline_layout(
            layout_fixture(vec![MangaReferenceBindingSummary {
                character_name: "hero".to_string(),
                bound_asset: Some(MangaBoundReferenceAsset {
                    asset_kind: MangaReferenceAssetKind::CharacterPortrait,
                    asset_id: 7,
                    image_path: "hero.png".to_string(),
                }),
                user_added_assets: Vec::new(),
            }]),
            "hero",
            MangaReferenceAssetKind::CharacterPortrait,
            7,
            MangaReferenceBindingAction::RemoveUserImage,
            None,
        )
        .expect_err("系统参考图不能作为用户参考图移除");
        assert_eq!(err, "角色 hero 的系统参考图不能移除");
    }

    #[test]
    fn bind_updates_v4_without_downgrading_pipeline() {
        let mut layout: MangaPageGenerationLayoutV4 = serde_json::from_value(json!({
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
        layout.pipeline.stage = MangaPagePipelineStage::DrawingPrompt;
        let bound = MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };

        let updated = mutate_reference_asset_in_pipeline_layout(
            layout,
            "hero",
            MangaReferenceAssetKind::GeneratedImage,
            7,
            MangaReferenceBindingAction::AddUserImage,
            Some(bound),
        )
        .unwrap();

        assert_eq!(updated.schema_version, MangaPageGenerationSchemaVersion::V4);
        assert_eq!(updated.pipeline.page_id, "page-0001");
        assert!(updated.reference_bindings[0].bound_asset.is_none());
        assert_eq!(
            updated.reference_bindings[0].user_added_assets[0].asset_id,
            7
        );
    }

    fn layout_fixture(
        reference_bindings: Vec<MangaReferenceBindingSummary>,
    ) -> MangaPageGenerationLayoutV4 {
        let mut layout = a4n_manga_core::manga_page_pipeline_layout_from_value(
            1,
            &json!({
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
            }),
        )
        .expect("valid layout fixture");
        layout.reference_bindings = reference_bindings;
        layout
    }
}
