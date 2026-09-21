use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::galgame::GalgameLine;

pub(in crate::galgame) struct GalgamePortraitAssetDraft {
    pub(in crate::galgame) canonical_key: String,
    pub(in crate::galgame) profile_id: i64,
    pub(in crate::galgame) variant_key: String,
    pub(in crate::galgame) expression: String,
    pub(in crate::galgame) pose: String,
    pub(in crate::galgame) position_samples: BTreeSet<String>,
    pub(in crate::galgame) source_line_ids: BTreeSet<i64>,
    pub(in crate::galgame) text_anchor_ids: BTreeSet<i64>,
}

impl GalgamePortraitAssetDraft {
    pub(in crate::galgame) fn add_line_sample(
        &mut self,
        line: &GalgameLine,
        scene_anchor_ids: &BTreeSet<i64>,
    ) {
        self.source_line_ids.insert(line.id);
        if let Some(position) = line.position {
            self.position_samples.insert(position.to_string());
        }
        if let Some(text_anchor_id) = line.text_anchor_id {
            self.text_anchor_ids.insert(text_anchor_id);
        } else {
            self.text_anchor_ids
                .extend(scene_anchor_ids.iter().copied());
        }
    }

    pub(in crate::galgame) fn params_json(&self) -> Value {
        json!({
            "canonical_key": self.canonical_key.clone(),
            "asset_kind": "portrait_expression",
            "character_profile_id": self.profile_id,
            "expression": self.expression.clone(),
            "pose": self.pose.clone(),
            "position_samples": self.position_samples.iter().cloned().collect::<Vec<_>>(),
            "source_line_ids": self.source_line_ids.iter().copied().collect::<Vec<_>>(),
        })
    }
}

pub(in crate::galgame) struct GalgamePortraitVariant {
    pub(in crate::galgame) key: String,
    pub(in crate::galgame) expression: String,
    pub(in crate::galgame) pose: String,
}

pub(in crate::galgame) fn portrait_variant(line: &GalgameLine) -> Option<GalgamePortraitVariant> {
    let expression = line
        .expression
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    let pose = line
        .pose
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    Some(GalgamePortraitVariant {
        key: format!("expression:{expression}:pose:{pose}"),
        expression: expression.to_string(),
        pose: pose.to_string(),
    })
}
