use super::super::contracts::{ASSEMBLY_PUSH_STATUS_REJECTED, AssemblyClip, AssemblyPushRow};

pub(in crate::actions::video_records) fn active_assembly_push_source_ids(
    pushes: &[AssemblyPushRow],
) -> Vec<i64> {
    pushes
        .iter()
        .filter(|push| push.status != ASSEMBLY_PUSH_STATUS_REJECTED)
        .flat_map(|push| push.source_video_ids.iter().copied())
        .collect()
}

pub(in crate::actions::video_records) fn selected_assembly_push_source_clips(
    pending_clips: &[AssemblyClip],
    end_video_id: Option<i64>,
) -> Result<Vec<AssemblyClip>, String> {
    if pending_clips.is_empty() {
        return Err("没有新的后续成片片段可推送".to_string());
    }
    if let Some(end_video_id) = end_video_id {
        let end_index = pending_clips
            .iter()
            .position(|clip| clip.video_id == end_video_id)
            .ok_or_else(|| format!("推送结束片段 #{end_video_id} 不在可推送后续片段中"))?;
        return Ok(pending_clips[..=end_index].to_vec());
    }
    Ok(pending_clips.to_vec())
}

pub(in crate::actions::video_records::assembly) fn latest_non_rejected_push(
    pushes: &[AssemblyPushRow],
) -> Option<&AssemblyPushRow> {
    pushes
        .iter()
        .rev()
        .find(|push| push.status != ASSEMBLY_PUSH_STATUS_REJECTED)
}

pub(in crate::actions::video_records) fn ensure_active_pushes_match_clip_prefix(
    clips: &[AssemblyClip],
    active_source_ids: &[i64],
) -> Result<(), String> {
    if active_source_ids.len() > clips.len() {
        return Err("已推送片段数量超过当前成片片段数量，请检查成片列表".to_string());
    }
    for (index, source_id) in active_source_ids.iter().enumerate() {
        let clip = &clips[index];
        if clip.video_id != *source_id {
            return Err(format!(
                "已推送片段与当前成片列表不连续：第 {} 个已推送源片段应为 #{}，当前为 #{}",
                index + 1,
                source_id,
                clip.video_id
            ));
        }
    }
    Ok(())
}
