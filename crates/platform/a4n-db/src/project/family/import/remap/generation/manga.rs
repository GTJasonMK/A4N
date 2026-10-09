use a4n_domain::{
    manga::{
        MangaBoundReferenceAsset, MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4,
        MangaPageImageArtifactV1, MangaPagePromptSourceSummary, MangaPageReferenceDiscoveryV1,
        MangaPipelineArtifactStage, MangaReferenceAssetKind, MangaReferenceBindingSummary,
    },
    media_authoring::AuthoringDocumentRef,
};
use serde_json::Value;

use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, map_optional, map_required,
};

pub(in crate::project::family::import) fn remap_manga_page_layout(
    value: &Value,
    maps: &ImportIdMaps,
    expected_page_number: u32,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    match value.get("schema_version").and_then(Value::as_str) {
        Some("manga_page_generation_v4") => {
            let mut layout: MangaPageGenerationLayoutV4 = serde_json::from_value(value.clone())
                .map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
                })?;
            layout.validate(expected_page_number).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
            })?;
            remap_manga_page_references(
                &mut layout.source,
                &mut layout.page_plan_ref,
                &mut layout.reference_bindings,
                maps,
                label,
            )?;
            remap_manga_page_pipeline(&mut layout, maps, label)?;
            layout.validate(expected_page_number).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!(
                    "重映射后的 {label} 无效: {error}"
                ))
            })?;
            serde_json::to_value(layout).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
            })
        }
        Some("manga_page_generation_v3") => {
            let mut layout: MangaPageGenerationLayoutV3 = serde_json::from_value(value.clone())
                .map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
                })?;
            layout.validate(expected_page_number).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
            })?;
            remap_manga_page_references(
                &mut layout.source,
                &mut layout.page_plan_ref,
                &mut layout.reference_bindings,
                maps,
                label,
            )?;
            layout.validate(expected_page_number).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!(
                    "重映射后的 {label} 无效: {error}"
                ))
            })?;
            serde_json::to_value(layout).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
            })
        }
        schema => Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} schema_version 不支持: {}",
            schema.unwrap_or("<missing>")
        ))),
    }
}

pub(in crate::project::family::import) fn remap_manga_pipeline_artifact_payload(
    stage: MangaPipelineArtifactStage,
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    match stage {
        MangaPipelineArtifactStage::ReferenceDiscovery => {
            let mut discovery: MangaPageReferenceDiscoveryV1 =
                serde_json::from_value(value.clone()).map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
                })?;
            for (index, reference) in discovery.references.iter_mut().enumerate() {
                remap_manga_reference_asset(
                    &mut reference.asset,
                    maps,
                    &format!("{label}.references[{index}].asset"),
                )?;
            }
            serde_json::to_value(discovery).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
            })
        }
        MangaPipelineArtifactStage::ReferenceBindings => {
            remap_manga_reference_bindings_value(value, maps, label)
        }
        MangaPipelineArtifactStage::Image => {
            let mut image: MangaPageImageArtifactV1 = serde_json::from_value(value.clone())
                .map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
                })?;
            image.workflow_run_id = None;
            image.generated_image_id = map_required(
                &maps.generated_image_ids,
                image.generated_image_id,
                &format!("{label}.generated_image_id"),
            )?;
            serde_json::to_value(image).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
            })
        }
        _ => Ok(value.clone()),
    }
}

fn remap_manga_page_references(
    source: &mut MangaPagePromptSourceSummary,
    page_plan_ref: &mut Option<AuthoringDocumentRef>,
    reference_bindings: &mut [MangaReferenceBindingSummary],
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    remap_manga_page_source(source, maps, label)?;
    if let Some(reference) = page_plan_ref {
        let old = reference.id;
        reference.id = map_required(&maps.authoring_document_ids, old, "漫画分页计划")?;
        reference.sha256 = maps
            .authoring_document_sha256s
            .get(&old)
            .cloned()
            .ok_or_else(|| ProjectFamilyImportError::invalid_package("漫画分页计划哈希映射缺失"))?;
    }
    remap_manga_reference_bindings(reference_bindings, maps, label)
}

fn remap_manga_page_pipeline(
    layout: &mut MangaPageGenerationLayoutV4,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    if let Some(discovery) = layout.pipeline.reference_discovery.as_mut() {
        for (index, reference) in discovery.references.iter_mut().enumerate() {
            remap_manga_reference_asset(
                &mut reference.asset,
                maps,
                &format!("{label}.pipeline.reference_discovery.references[{index}].asset"),
            )?;
        }
    }
    if let Some(image) = layout.pipeline.image.as_mut() {
        image.workflow_run_id = None;
        image.generated_image_id = map_required(
            &maps.generated_image_ids,
            image.generated_image_id,
            &format!("{label}.pipeline.image.generated_image_id"),
        )?;
    }
    Ok(())
}

pub(in crate::project::family::import) fn remap_manga_reference_bindings_value(
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut bindings: Vec<MangaReferenceBindingSummary> = serde_json::from_value(value.clone())
        .map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
        })?;
    remap_manga_reference_bindings(&mut bindings, maps, label)?;
    serde_json::to_value(bindings).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
    })
}

fn remap_manga_page_source(
    source: &mut MangaPagePromptSourceSummary,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let old_chapter_id = source.source_chapter_id;
    let old_snapshot_id = source.source_snapshot_id;
    let old_snapshot_chapter_id = source.source_snapshot_chapter_id;
    let expected_revision_key = match (old_snapshot_id, old_snapshot_chapter_id) {
        (None, None) => format!("live:{old_chapter_id}:{}", source.content_sha256),
        (Some(snapshot_id), Some(snapshot_chapter_id)) => format!(
            "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{}",
            source.content_sha256
        ),
        _ => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "{label}.source 的 source_snapshot_id 与 source_snapshot_chapter_id 必须同时存在或同时为空"
            )));
        }
    };
    if source.source_revision_key != expected_revision_key {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "{label}.source.source_revision_key 与来源身份不一致"
        )));
    }

    source.root_project_id = map_required(
        &maps.project_ids,
        source.root_project_id,
        &format!("{label}.source.root_project_id"),
    )?;
    source.source_chapter_id = map_required(
        &maps.chapter_ids,
        old_chapter_id,
        &format!("{label}.source.source_chapter_id"),
    )?;
    source.source_snapshot_id = map_optional(
        &maps.source_snapshot_ids,
        old_snapshot_id,
        &format!("{label}.source.source_snapshot_id"),
    )?;
    source.source_snapshot_chapter_id = map_optional(
        &maps.source_snapshot_chapter_ids,
        old_snapshot_chapter_id,
        &format!("{label}.source.source_snapshot_chapter_id"),
    )?;
    let old_plan_id = source.adaptation_plan_id;
    source.adaptation_plan_id = map_required(
        &maps.authoring_document_ids,
        old_plan_id,
        &format!("{label}.source.adaptation_plan_id"),
    )?;
    source.adaptation_plan_sha256 = maps
        .authoring_document_sha256s
        .get(&old_plan_id)
        .cloned()
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "{label}.source.adaptation_plan_sha256 引用了导入包外或尚未导入的改编计划: {old_plan_id}"
            ))
        })?;
    source.source_revision_key =
        match (source.source_snapshot_id, source.source_snapshot_chapter_id) {
            (None, None) => format!(
                "live:{}:{}",
                source.source_chapter_id, source.content_sha256
            ),
            (Some(snapshot_id), Some(snapshot_chapter_id)) => format!(
                "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{}",
                source.content_sha256
            ),
            _ => unreachable!("source snapshot identity was validated before remapping"),
        };
    Ok(())
}

fn remap_manga_reference_bindings(
    bindings: &mut [MangaReferenceBindingSummary],
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    for (index, binding) in bindings.iter_mut().enumerate() {
        if binding.character_name.trim().is_empty() {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "{label}.reference_bindings[{index}].character_name 不能为空"
            )));
        }
        if let Some(asset) = &mut binding.bound_asset {
            remap_manga_reference_asset(
                asset,
                maps,
                &format!("{label}.reference_bindings[{index}].bound_asset"),
            )?;
        }
        for (user_index, asset) in binding.user_added_assets.iter_mut().enumerate() {
            remap_manga_reference_asset(
                asset,
                maps,
                &format!("{label}.reference_bindings[{index}].user_added_assets[{user_index}]"),
            )?;
        }
    }
    Ok(())
}

fn remap_manga_reference_asset(
    asset: &mut MangaBoundReferenceAsset,
    maps: &ImportIdMaps,
    path: &str,
) -> Result<(), ProjectFamilyImportError> {
    let (ids, paths, kind) = match asset.asset_kind {
        MangaReferenceAssetKind::CharacterPortrait => (
            &maps.character_portrait_ids,
            &maps.character_portrait_paths,
            "character_portrait",
        ),
        MangaReferenceAssetKind::GeneratedImage => (
            &maps.generated_image_ids,
            &maps.generated_image_paths,
            "generated_image",
        ),
    };
    let old_id = asset.asset_id;
    asset.asset_id = map_required(ids, old_id, &format!("{path}.asset_id"))?;
    asset.image_path = paths.get(&old_id).cloned().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!(
            "{path} 引用了导入包外或尚未导入的 {kind} 文件: {old_id}"
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remap_manga_page_layout_preserves_v4_intermediate_pipeline() {
        let mut maps = ImportIdMaps::default();
        maps.project_ids.insert(1, 10);
        maps.chapter_ids.insert(2, 20);
        maps.authoring_document_ids.insert(3, 30);
        maps.authoring_document_sha256s
            .insert(3, "remapped-plan-sha256".into());

        let layout = json!({
            "schema_version": "manga_page_generation_v4",
            "page_id": "page-0001",
            "page_number": 1,
            "page_plan_ref": null,
            "pipeline": {
                "schema_version": "manga_page_pipeline_state_v1",
                "page_id": "page-0001",
                "status": "pending",
                "stage": "content",
                "attempt": 1,
                "source_fingerprint": "content-sha256",
                "stage_fingerprints": {},
                "page_content": {
                    "summary": "本页内容",
                    "source_span_ids": [],
                    "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                    "action": "角色抬头",
                    "reaction": null,
                    "dialogue_intent": null,
                    "entry_state": "平静",
                    "exit_state": "紧张"
                },
                "page_adaptation": null,
                "storyboard": null,
                "drawing_prompt": null,
                "image": null,
                "error": null
            },
            "reference_bindings": [],
            "prompt_fingerprint": {},
            "source": {
                "root_project_id": 1,
                "source_chapter_id": 2,
                "source_snapshot_id": null,
                "source_snapshot_chapter_id": null,
                "source_revision_key": "live:2:content-sha256",
                "content_sha256": "content-sha256",
                "adaptation_plan_id": 3,
                "adaptation_plan_revision": 1,
                "adaptation_plan_sha256": "plan-sha256"
            }
        });

        let remapped = remap_manga_page_layout(&layout, &maps, 1, "fixture").unwrap();

        assert_eq!(remapped["schema_version"], "manga_page_generation_v4");
        assert_eq!(remapped["source"]["root_project_id"], 10);
        assert_eq!(remapped["source"]["source_chapter_id"], 20);
        assert_eq!(remapped["source"]["adaptation_plan_id"], 30);
        assert_eq!(
            remapped["source"]["source_revision_key"],
            "live:20:content-sha256"
        );
        assert_eq!(remapped["pipeline"]["stage"], "content");
        assert_eq!(remapped["pipeline"]["page_content"]["summary"], "本页内容");
    }
}
