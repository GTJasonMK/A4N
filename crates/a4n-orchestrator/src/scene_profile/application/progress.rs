use super::types::SceneProfileStageProgressSpec;
use a4n_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
pub(crate) use a4n_scene_host_contract::SceneProfileSyncProgress;

pub(crate) use a4n_scene_host_contract::SCENE_PROFILE_SYNC_PROGRESS_TOTAL;
pub(super) const SCENE_PROFILE_SYNC_CHAPTER_START: u32 = 5;
pub(super) const SCENE_PROFILE_SYNC_CHAPTER_END: u32 = 88;
pub(super) const SCENE_PROFILE_SYNC_RAG_PROGRESS: u32 = 94;

pub(super) const SCENE_CANDIDATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "场景引用", 1),
    FieldMilestone::new("name", "场景名称", 1),
    FieldMilestone::new("scene_type", "场景类型", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("base_description", "基础描述", 1),
    FieldMilestone::new("visual_description", "视觉描述", 1),
    FieldMilestone::new("spatial_layout", "空间布局", 1),
    FieldMilestone::new("atmosphere", "氛围", 1),
    FieldMilestone::new("evidence_anchor_ids", "证据锚点", 1),
];
pub(super) const SCENE_OCCURRENCE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("occurrence_ref", "出场引用", 1),
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("summary", "出场摘要", 1),
    FieldMilestone::new("characters", "出场角色", 1),
    FieldMilestone::new("objects", "出场物体", 1),
    FieldMilestone::new("entry_state", "进入状态", 1),
    FieldMilestone::new("exit_state", "离开状态", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
pub(super) const SCENE_TRANSITION_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("from_scene_ref", "起点场景", 1),
    FieldMilestone::new("to_scene_ref", "目标场景", 1),
    FieldMilestone::new("movement_type", "移动类型", 1),
    FieldMilestone::new("route_description", "路线描述", 1),
    FieldMilestone::new("participants", "参与角色", 1),
    FieldMilestone::new("object_refs", "关联物体", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
pub(super) const SCENE_ENTITY_STATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("occurrence_ref", "关联出场", 1),
    FieldMilestone::new("entity_type", "实体类型", 1),
    FieldMilestone::new("entity_id", "实体 ID", 1),
    FieldMilestone::new("state_key", "状态类型", 1),
    FieldMilestone::new("state_before", "变化前状态", 1),
    FieldMilestone::new("state_after", "变化后状态", 1),
    FieldMilestone::new("action_description", "动作描述", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];

pub(super) fn subspan(
    span: ProgressSpan,
    start_units: u32,
    end_units: u32,
    total_units: u32,
) -> ProgressSpan {
    let total_units = total_units.max(1);
    let start_units = start_units.min(total_units);
    let end_units = end_units.min(total_units).max(start_units);
    let width = span.end() - span.start();
    let mapped_start = u64::from(width) * u64::from(start_units) / u64::from(total_units);
    let mapped_end = u64::from(width) * u64::from(end_units) / u64::from(total_units);
    ProgressSpan::new(
        span.start()
            + u32::try_from(mapped_start)
                .expect("scene subspan start is bounded by the u32 span width"),
        span.start()
            + u32::try_from(mapped_end)
                .expect("scene subspan end is bounded by the u32 span width"),
    )
}

pub(super) fn scene_stage_progress_spec(
    root_array_key: &'static str,
    item_label: &'static str,
    milestones: &'static [FieldMilestone],
    span: ProgressSpan,
) -> SceneProfileStageProgressSpec {
    SceneProfileStageProgressSpec {
        structured: StructuredJsonProgressSpec {
            root_array_key,
            item_label,
            expected_items: None,
            milestones,
        },
        span,
    }
}

pub(super) fn report_scene_profile_sync_progress(
    progress: Option<&SceneProfileSyncProgress<'_>>,
    current: u32,
    message: impl AsRef<str>,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, SCENE_PROFILE_SYNC_PROGRESS_TOTAL, message.as_ref());
        progress.check_cancelled(cancelled_message)?;
    }
    Ok(())
}
