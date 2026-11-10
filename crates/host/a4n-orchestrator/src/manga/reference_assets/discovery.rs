use std::collections::{HashMap, HashSet};

use a4n_domain::{
    manga::{
        MANGA_PAGE_REFERENCE_DISCOVERY_V1, MANGA_PAGE_REFERENCE_IMAGE_LIMIT,
        MANGA_PAGE_REFERENCE_REQUIREMENT_LIMIT,
        MANGA_REFERENCE_DESCRIPTION_MAX_CHARS as MAX_REFERENCE_DESCRIPTION_CHARS,
        MANGA_REFERENCE_TARGET_NAME_MAX_CHARS,
        MANGA_REFERENCE_USAGE_MAX_CHARS as MAX_REFERENCE_USAGE_CHARS,
        MangaPageReferenceDiscoveryV1, MangaPageReferenceImageV1, MangaPageReferenceNeedV1,
        MangaPageReferenceTargetKind, MangaReferenceAssetKind, MangaReferenceBindingSummary,
    },
    media::GeneratedImageRecord,
};
use a4n_error::AppError;
use serde::Deserialize;
use serde_json::{Value, json};

const MAX_REFERENCE_CANDIDATES: usize = 48;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReferenceDiscoveryResponse {
    #[schemars(extend("const"=MANGA_PAGE_REFERENCE_DISCOVERY_V1))]
    schema_version: String,
    #[schemars(range(min = 1))]
    page_number: u32,
    /// 同类型、同名称的需求只能出现一次，视觉描述与连续性用途合并到同一项。
    #[schemars(length(max=MANGA_PAGE_REFERENCE_REQUIREMENT_LIMIT))]
    requirements: Vec<MangaPageReferenceNeedV1>,
    #[schemars(length(max=MANGA_PAGE_REFERENCE_IMAGE_LIMIT))]
    references: Vec<ReferenceSelection>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReferenceSelection {
    target_kind: MangaPageReferenceTargetKind,
    #[schemars(length(min=1,max=MANGA_REFERENCE_TARGET_NAME_MAX_CHARS),regex(pattern=r"\S"))]
    target_name: String,
    asset_kind: MangaReferenceAssetKind,
    #[schemars(range(min = 1))]
    asset_id: i64,
    #[schemars(length(min=1,max=MAX_REFERENCE_USAGE_CHARS),regex(pattern=r"\S"))]
    usage: String,
}

pub(in crate::manga) fn manga_reference_discovery_output_schema() -> Value {
    a4n_domain::json_contract::typed_output_schema::<ReferenceDiscoveryResponse>()
}

#[cfg(test)]
#[test]
fn model_contract_matrix_raw_reference_response() {
    let raw = json!({"schema_version":MANGA_PAGE_REFERENCE_DISCOVERY_V1,"page_number":1,"requirements":[],"references":[]});
    let parsed = parse_manga_page_reference_discovery(&raw, 1, &HashMap::new()).unwrap();
    let validator = a4n_domain::json_contract::OutputSchemaValidator::new(
        &manga_reference_discovery_output_schema(),
    )
    .unwrap();
    assert_eq!(
        crate::manga::pipeline::json::output_schema("manga_page_reference_discovery").unwrap(),
        manga_reference_discovery_output_schema()
    );
    validator.validate(&raw).unwrap();
    assert!(
        validator
            .validate(&serde_json::to_value(parsed).unwrap())
            .is_err(),
        "normalized storage data must not become the model output schema"
    );
    for field in [
        "schema_version",
        "page_number",
        "requirements",
        "references",
    ] {
        let mut missing = raw.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            parse_manga_page_reference_discovery(&missing, 1, &HashMap::new()).is_err(),
            "{field}"
        );
    }
}

#[derive(Debug, Clone)]
pub(in crate::manga) struct ReferenceCandidate {
    pub(in crate::manga) asset_kind: MangaReferenceAssetKind,
    pub(in crate::manga) asset_id: i64,
    pub(in crate::manga) image_path: String,
    pub(in crate::manga) description: String,
}

fn reference_image_candidate(
    image: &GeneratedImageRecord,
    current_page_image_id: Option<i64>,
) -> Option<ReferenceCandidate> {
    if image.id <= 0 || image.file_path.trim().is_empty() || current_page_image_id == Some(image.id)
    {
        return None;
    }
    Some(ReferenceCandidate {
        asset_kind: MangaReferenceAssetKind::GeneratedImage,
        asset_id: image.id,
        image_path: image.file_path.clone(),
        description: image_description(image),
    })
}

pub(in crate::manga) fn manga_page_reference_discovery_candidates(
    images: &[GeneratedImageRecord],
    current_page_image_id: Option<i64>,
    bindings: &[MangaReferenceBindingSummary],
) -> (
    Value,
    HashMap<(MangaReferenceAssetKind, i64), ReferenceCandidate>,
) {
    let user_asset_keys = bindings
        .iter()
        .flat_map(|binding| {
            binding.user_added_assets.iter().chain(
                binding
                    .bound_asset
                    .iter()
                    .filter(|asset| asset.asset_kind == MangaReferenceAssetKind::GeneratedImage),
            )
        })
        .map(|asset| (asset.asset_kind, asset.asset_id))
        .collect::<HashSet<_>>();
    let mut candidates = HashMap::new();
    let mut items = Vec::new();

    for image in images {
        if user_asset_keys.contains(&(MangaReferenceAssetKind::GeneratedImage, image.id)) {
            continue;
        }
        let Some(candidate) = reference_image_candidate(image, current_page_image_id) else {
            continue;
        };
        if candidate.description.is_empty() {
            continue;
        }
        let key = (MangaReferenceAssetKind::GeneratedImage, image.id);
        if candidates.contains_key(&key) {
            continue;
        }
        items.push(json!({
            "asset_kind": "generated_image",
            "asset_id": image.id,
            "image_type": image.image_type,
            "description": &candidate.description,
            "is_favorite": image.is_favorite,
        }));
        candidates.insert(key, candidate);
        if items.len() == MAX_REFERENCE_CANDIDATES {
            break;
        }
    }

    (Value::Array(items), candidates)
}

/// Stored bindings use the authorized asset scope, independent of model candidate limits and deduplication.
pub(in crate::manga) fn validate_saved_manga_page_reference_discovery(
    discovery: &MangaPageReferenceDiscoveryV1,
    images: &[GeneratedImageRecord],
    current_page_image_id: Option<i64>,
) -> Result<(), AppError> {
    let candidates = images
        .iter()
        .filter(|image| {
            discovery.references.iter().any(|reference| {
                reference.asset.asset_kind == MangaReferenceAssetKind::GeneratedImage
                    && reference.asset.asset_id == image.id
            })
        })
        .filter_map(|image| reference_image_candidate(image, current_page_image_id))
        .map(|candidate| ((candidate.asset_kind, candidate.asset_id), candidate))
        .collect();
    validate_manga_page_reference_discovery_candidates(discovery, &candidates)
}

fn validate_manga_page_reference_discovery_candidates(
    discovery: &MangaPageReferenceDiscoveryV1,
    candidates: &HashMap<(MangaReferenceAssetKind, i64), ReferenceCandidate>,
) -> Result<(), AppError> {
    discovery
        .validate_for_write()
        .map_err(AppError::InvalidArgument)?;
    for (index, reference) in discovery.references.iter().enumerate() {
        let candidate = candidates
            .get(&(reference.asset.asset_kind, reference.asset.asset_id))
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "references[{index}] 引用了当前页面不可用的素材 {:?}#{}",
                    reference.asset.asset_kind, reference.asset.asset_id
                ))
            })?;
        if reference.asset.image_path != candidate.image_path {
            return Err(AppError::InvalidArgument(format!(
                "references[{index}].asset.image_path 与素材记录不一致"
            )));
        }
    }
    Ok(())
}

pub(in crate::manga) fn parse_manga_page_reference_discovery(
    value: &Value,
    page_number: u32,
    candidates: &HashMap<(MangaReferenceAssetKind, i64), ReferenceCandidate>,
) -> Result<MangaPageReferenceDiscoveryV1, AppError> {
    let response: ReferenceDiscoveryResponse =
        serde_json::from_value(value.clone()).map_err(|error| {
            AppError::InvalidArgument(format!("manga_page_reference_discovery 输出无效: {error}"))
        })?;
    if response.schema_version != MANGA_PAGE_REFERENCE_DISCOVERY_V1 {
        return Err(AppError::InvalidArgument(format!(
            "manga_page_reference_discovery.schema_version 必须是 {MANGA_PAGE_REFERENCE_DISCOVERY_V1}"
        )));
    }
    if response.page_number != page_number {
        return Err(AppError::InvalidArgument(format!(
            "manga_page_reference_discovery.page_number 必须等于 {page_number}"
        )));
    }
    let mut references = Vec::with_capacity(response.references.len());
    for (index, selected) in response.references.into_iter().enumerate() {
        let key = (selected.asset_kind, selected.asset_id);
        let Some(candidate) = candidates.get(&key) else {
            return Err(AppError::InvalidArgument(format!(
                "manga_page_reference_discovery.references[{index}] 引用了不可用素材 {:?}#{}",
                selected.asset_kind, selected.asset_id
            )));
        };
        references.push(MangaPageReferenceImageV1 {
            asset: a4n_domain::manga::MangaBoundReferenceAsset {
                asset_kind: candidate.asset_kind,
                asset_id: candidate.asset_id,
                image_path: candidate.image_path.clone(),
            },
            target_kind: selected.target_kind,
            target_name: selected.target_name.trim().to_string(),
            description: candidate.description.clone(),
            usage: selected.usage.trim().to_string(),
        });
    }
    let discovery = MangaPageReferenceDiscoveryV1 {
        schema_version: MANGA_PAGE_REFERENCE_DISCOVERY_V1.to_string(),
        requirements: response.requirements,
        references,
    };
    discovery
        .validate_for_write()
        .map_err(AppError::InvalidArgument)?;
    Ok(discovery)
}

fn image_description(image: &GeneratedImageRecord) -> String {
    let prompt = image.prompt.trim();
    let description = if prompt.is_empty() {
        image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("description"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
    } else {
        prompt
    };
    description
        .chars()
        .take(MAX_REFERENCE_DESCRIPTION_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::collections::HashMap;

    use a4n_domain::manga::{
        MANGA_PAGE_REFERENCE_DISCOVERY_V1, MangaBoundReferenceAsset, MangaPageReferenceDiscoveryV1,
        MangaPageReferenceImageV1, MangaPageReferenceTargetKind, MangaReferenceAssetKind,
    };

    use super::{ReferenceCandidate, validate_manga_page_reference_discovery_candidates};

    fn discovery(asset_id: i64, image_path: &str) -> MangaPageReferenceDiscoveryV1 {
        MangaPageReferenceDiscoveryV1 {
            schema_version: MANGA_PAGE_REFERENCE_DISCOVERY_V1.into(),
            requirements: Vec::new(),
            references: vec![MangaPageReferenceImageV1 {
                asset: MangaBoundReferenceAsset {
                    asset_kind: MangaReferenceAssetKind::GeneratedImage,
                    asset_id,
                    image_path: image_path.into(),
                },
                target_kind: MangaPageReferenceTargetKind::Object,
                target_name: "旧怀表".into(),
                description: "黄铜表壳".into(),
                usage: "保持外观".into(),
            }],
        }
    }

    fn candidates() -> HashMap<(MangaReferenceAssetKind, i64), ReferenceCandidate> {
        HashMap::from([(
            (MangaReferenceAssetKind::GeneratedImage, 7),
            ReferenceCandidate {
                asset_kind: MangaReferenceAssetKind::GeneratedImage,
                asset_id: 7,
                image_path: "images/watch.png".into(),
                description: "旧怀表".into(),
            },
        )])
    }

    #[test]
    fn reference_discovery_rejects_unavailable_assets_and_forged_paths() {
        assert!(
            validate_manga_page_reference_discovery_candidates(
                &discovery(7, "images/watch.png"),
                &candidates(),
            )
            .is_ok()
        );
        assert!(
            validate_manga_page_reference_discovery_candidates(
                &discovery(8, "images/watch.png"),
                &candidates(),
            )
            .is_err()
        );
        assert!(
            validate_manga_page_reference_discovery_candidates(
                &discovery(7, "/private/other-project.png"),
                &candidates(),
            )
            .is_err()
        );
    }

    #[test]
    fn model_reference_selection_uses_the_shared_target_capacity_contract() {
        let candidates = (1..=3)
            .map(|id| {
                (
                    (MangaReferenceAssetKind::GeneratedImage, id),
                    ReferenceCandidate {
                        asset_kind: MangaReferenceAssetKind::GeneratedImage,
                        asset_id: id,
                        image_path: format!("images/watch-{id}.png"),
                        description: "黄铜表壳".into(),
                    },
                )
            })
            .collect();
        let mut response = json!({"schema_version":MANGA_PAGE_REFERENCE_DISCOVERY_V1,"page_number":1,"requirements":[],"references":[]});
        for id in 1..=3 {
            response["references"].as_array_mut().unwrap().push(json!({
                "asset_kind":"generated_image","asset_id":id,"target_kind":"object","target_name":"旧怀表","usage":"保持外观"
            }));
            let result = super::parse_manga_page_reference_discovery(&response, 1, &candidates);
            if id <= 2 {
                result.unwrap();
            } else {
                assert!(result.unwrap_err().to_string().contains("最多绑定两张"));
            }
        }
        response["references"] = json!([]);
        let need = json!({"target_kind":"object","target_name":"旧怀表","description":"黄铜表壳","usage":"保持外观"});
        response["requirements"] = json!([need.clone(), need]);
        assert!(
            super::parse_manga_page_reference_discovery(&response, 1, &candidates)
                .unwrap_err()
                .to_string()
                .contains("不唯一")
        );
    }

    #[test]
    fn saved_references_are_independent_of_model_candidate_limits_and_user_deduplication() {
        let images = (1..=49)
            .map(|id| a4n_domain::media::GeneratedImageRecord {
                id,
                project_id: 1,
                image_type: "chapter_reference".into(),
                file_path: format!("images/watch-{id}.png"),
                prompt: "黄铜怀表".into(),
                model: "fixture".into(),
                width: 1024,
                height: 1024,
                file_size: None,
                ai_score: None,
                is_favorite: false,
                chapter_id: Some(2),
                panel_id: None,
                metadata_json: None,
                created_at: Default::default(),
            })
            .collect::<Vec<_>>();
        let saved = discovery(49, "images/watch-49.png");
        let (available, candidates) =
            super::manga_page_reference_discovery_candidates(&images, None, &[]);
        assert_eq!(available.as_array().unwrap().len(), 48);
        assert!(!candidates.contains_key(&(MangaReferenceAssetKind::GeneratedImage, 49)));
        super::validate_saved_manga_page_reference_discovery(&saved, &images, None).unwrap();
        let binding = a4n_domain::manga::MangaReferenceBindingSummary {
            character_name: "hero".into(),
            bound_asset: None,
            user_added_assets: vec![saved.references[0].asset.clone()],
        };
        let (available, _) =
            super::manga_page_reference_discovery_candidates(&images[48..], None, &[binding]);
        assert!(available.as_array().unwrap().is_empty());
        super::validate_saved_manga_page_reference_discovery(&saved, &images, None).unwrap();
        assert!(
            super::validate_saved_manga_page_reference_discovery(&saved, &images, Some(49))
                .is_err()
        );
        assert!(super::validate_saved_manga_page_reference_discovery(&saved, &[], None).is_err());
        assert!(
            super::validate_saved_manga_page_reference_discovery(
                &discovery(49, "forged.png"),
                &images,
                None
            )
            .is_err()
        );
        let raw = json!({"schema_version":MANGA_PAGE_REFERENCE_DISCOVERY_V1,"page_number":1,"requirements":[],
            "references":[{"asset_kind":"generated_image","asset_id":49,"target_kind":"object","target_name":"旧怀表","usage":"保持外观"}]});
        assert!(super::parse_manga_page_reference_discovery(&raw, 1, &candidates).is_err());
    }
}
