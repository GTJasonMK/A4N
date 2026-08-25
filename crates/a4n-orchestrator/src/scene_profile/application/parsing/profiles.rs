use super::super::evidence::resolve_scene_text_anchor_texts;
use super::super::types::{SceneProfileEvidenceRefs, SceneProfileSyncRequestContext};
use super::validation::validate_id_in_set;
use crate::scene_profile::{SCENE_TYPES, SceneProfileDraft, normalize_scene_type_value};
use a4n_core::json_contract::{
    ensure_object, optional_positive_i64, positive_i64_array, reject_unknown_fields,
    required_array, required_array_value, required_string, string_array,
};
use a4n_core::profile_text::normalize_importance_value;
use serde_json::Value;
use std::collections::HashSet;

pub(super) struct ParsedSceneProfiles {
    pub(super) drafts: Vec<SceneProfileDraft>,
    pub(super) evidence_refs: Vec<SceneProfileEvidenceRefs>,
}

pub(super) fn parse_scene_profiles(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<ParsedSceneProfiles, String> {
    let scenes = required_array(root, "scenes", "root")?;
    let mut refs = HashSet::new();
    let mut drafts = Vec::with_capacity(scenes.len());
    let mut evidence_refs = Vec::with_capacity(scenes.len());
    for (index, scene) in scenes.iter().enumerate() {
        let path = format!("scenes[{index}]");
        ensure_object(scene, &path)?;
        reject_unknown_fields(
            scene,
            &path,
            &[
                "scene_ref",
                "scene_profile_id",
                "name",
                "aliases",
                "scene_type",
                "importance",
                "parent_scene_id",
                "base_description",
                "visual_description",
                "spatial_layout",
                "atmosphere",
                "entry_points",
                "rules",
                "evidence_anchor_ids",
            ],
        )?;
        let scene_ref = required_string(scene, "scene_ref", &path)?;
        if !refs.insert(scene_ref.clone()) {
            return Err(format!("{path}.scene_ref 与前序场景重复: {scene_ref}"));
        }
        let scene_profile_id = optional_positive_i64(scene, "scene_profile_id", &path)?;
        if let Some(id) = scene_profile_id {
            validate_id_in_set(
                id,
                &context.existing_scene_ids,
                &format!("{path}.scene_profile_id"),
            )?;
        }
        let parent_scene_id = optional_positive_i64(scene, "parent_scene_id", &path)?;
        if let Some(id) = parent_scene_id {
            validate_id_in_set(
                id,
                &context.existing_scene_ids,
                &format!("{path}.parent_scene_id"),
            )?;
        }
        let first_seen_chapter = context.chapter.chapter_number;
        let last_seen_chapter = context.chapter.chapter_number;
        let evidence_anchor_ids = positive_i64_array(scene, "evidence_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.evidence_anchor_ids"),
            &evidence_anchor_ids,
            context,
        )?;
        evidence_refs.push(SceneProfileEvidenceRefs {
            scene_ref: scene_ref.clone(),
            evidence_anchor_ids,
        });
        let name = required_string(scene, "name", &path)?;
        let base_description = required_string(scene, "base_description", &path)?;
        let visual_description = required_string(scene, "visual_description", &path)?;
        let spatial_layout = required_string(scene, "spatial_layout", &path)?;
        let atmosphere = required_string(scene, "atmosphere", &path)?;
        let image_prompt_seed = scene_image_prompt_seed(
            &name,
            &base_description,
            &visual_description,
            &spatial_layout,
            &atmosphere,
        );
        drafts.push(SceneProfileDraft {
            scene_ref,
            scene_profile_id,
            name,
            aliases: string_array(scene, "aliases", &path, false)?,
            scene_type: scene_type_string(scene, "scene_type", &path)?,
            importance: importance_string(scene, "importance", &path)?,
            parent_scene_id,
            base_description,
            visual_description,
            spatial_layout,
            atmosphere,
            entry_points: required_array_value(scene, "entry_points", &path)?,
            rules: required_array_value(scene, "rules", &path)?,
            first_seen_chapter,
            last_seen_chapter,
            evidence,
            image_prompt_seed,
        });
    }
    Ok(ParsedSceneProfiles {
        drafts,
        evidence_refs,
    })
}

fn scene_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_TYPES.join("/")
        )
    })
}

fn importance_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_importance_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：primary/secondary/minor",
            text
        )
    })
}

fn scene_image_prompt_seed(
    name: &str,
    base_description: &str,
    visual_description: &str,
    spatial_layout: &str,
    atmosphere: &str,
) -> String {
    format!(
        "{name}，{base_description}，{visual_description}，空间布局：{spatial_layout}，氛围光线：{atmosphere}，可复用场景素材设定图"
    )
}
