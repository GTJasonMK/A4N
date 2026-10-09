use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{
    media_authoring::{
        AdaptationBeat, AdaptationBeatKind, AdaptationBeatRef, AdaptationImportance,
        AdaptationOrigin, AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget,
        AdaptationUnit,
    },
    shooting_script::{ChapterShootingScriptV2, ShootingCoverageV1},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyShootingBeatRef {
    pub scene_index: u32,
    pub shot_index: u32,
    pub beat_index: u32,
}

pub struct LegacyShootingIdentity<'a> {
    pub id: i64,
    pub revision: i32,
    pub sha256: &'a str,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: &'a str,
    pub source_content_sha256: &'a str,
}

pub fn adaptation_from_shooting_script(
    script: &ChapterShootingScriptV2,
    target: AdaptationTarget,
) -> Result<
    (
        AdaptationPlanV1,
        BTreeMap<LegacyShootingBeatRef, AdaptationBeatRef>,
    ),
    String,
> {
    script.validate().map_err(|error| error.to_string())?;
    let mut mapping = BTreeMap::new();
    let mut units = Vec::with_capacity(script.scenes.len());
    for scene in &script.scenes {
        let mut beats = Vec::new();
        for shot in &scene.shots {
            for beat in &shot.performance_beats {
                let id = serde_json::to_string(&(&shot.shot_id, &beat.beat_id))
                    .map_err(|error| error.to_string())?;
                let reference = AdaptationBeatRef {
                    unit_id: scene.scene_identity.clone(),
                    beat_id: id.clone(),
                };
                let old = LegacyShootingBeatRef {
                    scene_index: scene.scene_index,
                    shot_index: shot.shot_index,
                    beat_index: beat.beat_index,
                };
                if mapping.insert(old, reference).is_some() {
                    return Err("历史拍摄节拍身份重复，不能转换改编来源".into());
                }
                beats.push(AdaptationBeat {
                    id,
                    kind: if beat.dialogue_intent.is_some() {
                        AdaptationBeatKind::Dialogue
                    } else if beat.reaction.is_some() {
                        AdaptationBeatKind::Reaction
                    } else if !beat.knowledge_delta.is_empty() {
                        AdaptationBeatKind::Revelation
                    } else if beat.spatial_change.is_some() {
                        AdaptationBeatKind::Transition
                    } else {
                        AdaptationBeatKind::Action
                    },
                    is_climax: beat.required
                        && (!beat.knowledge_delta.is_empty() || beat.relationship_delta.is_some()),
                    origin: if beat.source_span_ids.is_empty() {
                        AdaptationOrigin::Historical
                    } else {
                        AdaptationOrigin::Adapted
                    },
                    importance: if beat.required || shot.coverage == ShootingCoverageV1::MustShow {
                        AdaptationImportance::Required
                    } else if shot.coverage == ShootingCoverageV1::ShouldShow {
                        AdaptationImportance::Important
                    } else {
                        AdaptationImportance::Supporting
                    },
                    action: beat.visible_action.clone(),
                    reaction: beat.reaction.clone(),
                    dialogue_intent: beat.dialogue_intent.clone(),
                    entry_state: beat.entry_state.clone(),
                    exit_state: beat.exit_state.clone(),
                    source_span_ids: beat.source_span_ids.clone(),
                });
            }
        }
        units.push(AdaptationUnit {
            id: scene.scene_identity.clone(),
            origin: if scene.source_span_ids.is_empty() {
                AdaptationOrigin::Historical
            } else {
                AdaptationOrigin::Adapted
            },
            title: scene.title.clone(),
            location: scene.location.clone(),
            participants: scene.participants.clone(),
            dramatic_question: scene.dramatic_question.clone(),
            objective: scene.scene_objective.clone(),
            obstacle: scene.scene_obstacle.clone(),
            entry_state: scene.entry_state.clone(),
            exit_state: scene.exit_state.clone(),
            source_span_ids: scene.source_span_ids.clone(),
            beats,
        });
    }
    Ok((
        AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target,
            page_count: None,
            intention: "保留迁移前已经采用的叙事取舍，后续在目标项目独立改编".into(),
            decisions: Vec::new(),
            units,
        },
        mapping,
    ))
}
