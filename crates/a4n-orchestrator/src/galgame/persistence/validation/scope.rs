use std::collections::{BTreeMap, BTreeSet};

use a4n_core::{
    error::AppError,
    models::{
        character::protagonist_profile,
        project::{source_snapshot_chapter, text_anchor},
    },
};
use a4n_db::galgame::GalgamePersistValidationScopeReadModel;
use serde_json::Value;

use crate::galgame::{GalgameScriptPersistPlan, OrchestratorContext};

pub(in crate::galgame) struct GalgamePersistValidationScope {
    pub(in crate::galgame) source_snapshot_chapters: BTreeMap<i64, source_snapshot_chapter::Model>,
    pub(in crate::galgame) speaker_profiles: BTreeMap<i64, protagonist_profile::Model>,
    pub(in crate::galgame) text_anchors: BTreeMap<i64, text_anchor::Model>,
}

impl GalgamePersistValidationScope {
    pub(in crate::galgame::persistence) async fn load(
        service: &OrchestratorContext,
        plan: &GalgameScriptPersistPlan,
    ) -> Result<Self, AppError> {
        let mut source_snapshot_chapter_ids = BTreeSet::new();
        let mut speaker_profile_ids = BTreeSet::new();
        let mut text_anchor_ids = BTreeSet::new();

        for scene in &plan.scenes {
            if scene.source_snapshot_chapter_id > 0 {
                source_snapshot_chapter_ids.insert(scene.source_snapshot_chapter_id);
            }
            collect_participant_profile_ids(&scene.participants_json, &mut speaker_profile_ids);
            for line in &scene.lines {
                if let Some(speaker_profile_id) = line.speaker_profile_id
                    && speaker_profile_id > 0
                {
                    speaker_profile_ids.insert(speaker_profile_id);
                }
                if let Some(text_anchor_id) = line.text_anchor_id
                    && text_anchor_id > 0
                {
                    text_anchor_ids.insert(text_anchor_id);
                }
            }
        }

        let read_model = a4n_db::galgame::galgame_persist_validation_scope_read_model(
            service.database.connection(),
            &source_snapshot_chapter_ids.into_iter().collect::<Vec<_>>(),
            &speaker_profile_ids.into_iter().collect::<Vec<_>>(),
            &text_anchor_ids.into_iter().collect::<Vec<_>>(),
        )
        .await
        .map_err(AppError::Database)?;
        Ok(Self::from_read_model(read_model))
    }

    pub(in crate::galgame) fn from_read_model(
        read_model: GalgamePersistValidationScopeReadModel,
    ) -> Self {
        Self {
            source_snapshot_chapters: read_model
                .source_snapshot_chapters
                .into_iter()
                .map(|chapter| (chapter.id, chapter))
                .collect(),
            speaker_profiles: read_model
                .speaker_profiles
                .into_iter()
                .map(|profile| (profile.id, profile))
                .collect(),
            text_anchors: read_model
                .text_anchors
                .into_iter()
                .map(|anchor| (anchor.id, anchor))
                .collect(),
        }
    }
}

fn collect_participant_profile_ids(value: &Value, profile_ids: &mut BTreeSet<i64>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        if let Some(profile_id) = object
            .get("character_profile_id")
            .and_then(Value::as_i64)
            .filter(|profile_id| *profile_id > 0)
        {
            profile_ids.insert(profile_id);
        }
    }
}
