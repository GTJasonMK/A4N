use std::collections::{BTreeMap, BTreeSet};

use a4n_error::AppError;

use crate::galgame::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_TARGET_PORTRAIT_EXPRESSION, GalgameLine,
};
use a4n_db::project::ProjectFamilyContext;

use super::super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft, GalgameSpecInput};
use super::super::drafts::{
    GalgamePortraitAssetDraft, galgame_asset_plan_item_draft, portrait_variant,
};
use super::required_json_string;

pub(super) fn collect_portrait_asset_sample(
    context: &ProjectFamilyContext,
    source_snapshot_id: i64,
    scene_title: &str,
    line: &GalgameLine,
    scene_anchor_ids: &BTreeSet<i64>,
    portrait_drafts: &mut BTreeMap<(i64, String), GalgamePortraitAssetDraft>,
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
        .entry((source_snapshot_id, canonical_key.clone()))
        .or_insert_with(|| GalgamePortraitAssetDraft {
            canonical_key,
            profile_id,
            variant_key: variant.key,
            expression: variant.expression,
            pose: variant.pose,
            position_samples: BTreeSet::new(),
            source_line_ids: BTreeSet::new(),
            text_anchor_ids: BTreeSet::new(),
            source_labels: BTreeSet::new(),
        });
    draft.source_labels.insert(scene_title.to_string());
    draft.add_line_sample(line, scene_anchor_ids);
}

pub(super) fn portrait_asset_plan_drafts(
    context: &ProjectFamilyContext,
    portrait_drafts: BTreeMap<(i64, String), GalgamePortraitAssetDraft>,
) -> Result<Vec<GalgameAssetPlanItemDraft>, AppError> {
    portrait_drafts
        .into_iter()
        .map(|((source_snapshot_id, _), draft)| {
            let params_json = draft.params_json();
            let canonical_key = required_json_string(&params_json, "canonical_key")?;
            galgame_asset_plan_item_draft(
                GalgameAssetPlanGroup::PortraitExpression,
                GalgameSpecInput {
                    canonical_key,
                    project_id: context.project.id,
                    root_project_id: context.root_project.id,
                    source_snapshot_id,
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
                draft
                    .source_labels
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" · "),
            )
        })
        .collect()
}
