use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use a4n_core::models::galgame::galgame_line;

use crate::galgame::{GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_TARGET_PORTRAIT_EXPRESSION};
use a4n_db::project::ProjectFamilyContext;

use super::super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft, GalgameSpecInput};
use super::super::drafts::{
    GalgamePortraitAssetDraft, galgame_asset_plan_item_draft, portrait_variant,
};
use super::required_json_string;

pub(super) fn collect_portrait_asset_sample(
    context: &ProjectFamilyContext,
    line: &galgame_line::Model,
    scene_anchor_ids: &BTreeSet<i64>,
    portrait_drafts: &mut BTreeMap<String, GalgamePortraitAssetDraft>,
) {
    let Some(profile_id) = line.speaker_profile_id else {
        return;
    };
    let Some(variant) = portrait_variant(line) else {
        return;
    };
    let canonical_key = format!(
        "galgame:{}:portrait:{}:{}",
        context.project.id, profile_id, variant.key
    );
    let draft = portrait_drafts
        .entry(canonical_key.clone())
        .or_insert_with(|| GalgamePortraitAssetDraft {
            canonical_key,
            profile_id,
            variant_key: variant.key,
            expression: variant.expression,
            pose: variant.pose,
            position_samples: BTreeSet::new(),
            source_line_ids: BTreeSet::new(),
            text_anchor_ids: BTreeSet::new(),
        });
    draft.add_line_sample(line, scene_anchor_ids);
}

pub(super) fn portrait_asset_plan_drafts(
    context: &ProjectFamilyContext,
    portrait_drafts: BTreeMap<String, GalgamePortraitAssetDraft>,
) -> Result<Vec<GalgameAssetPlanItemDraft>, AppError> {
    portrait_drafts
        .into_values()
        .map(|draft| {
            let params_json = draft.params_json();
            let canonical_key = required_json_string(&params_json, "canonical_key")?;
            galgame_asset_plan_item_draft(
                GalgameAssetPlanGroup::PortraitExpression,
                GalgameSpecInput {
                    canonical_key,
                    project_id: context.project.id,
                    root_project_id: context.root_project.id,
                    source_snapshot_id: context.source_snapshot.id,
                    source_chapter_id: None,
                    source_snapshot_chapter_id: None,
                    target_kind: GALGAME_TARGET_PORTRAIT_EXPRESSION,
                    target_entity_kind: GALGAME_ENTITY_CHARACTER_PROFILE,
                    target_entity_id: Some(draft.profile_id),
                    params_json,
                },
                &draft.text_anchor_ids,
                true,
                Some(draft.profile_id),
                format!("角色 {} 差分 {}", draft.profile_id, draft.variant_key),
            )
        })
        .collect()
}
