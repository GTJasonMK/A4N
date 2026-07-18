use std::path::Path;

pub(super) const CONTRACT_BOUND_PROMPT_PATHS: &[&str] = &[
    "novel/04_writing/writing.md",
    "novel/05_analysis/content_optimization.md",
    "novel/05_analysis/content_optimization_chat.md",
    "novel/05_analysis/content_optimization_patch.md",
    "novel/05_analysis/chapter_fact_extraction.md",
    "novel/05_analysis/chapter_summary_synthesis.md",
    "novel/05_analysis/chapter_character_state.md",
    "novel/05_analysis/chapter_foreshadowing_events.md",
    "novel/05_analysis/chapter_screenplay_skeleton.md",
    "novel/05_analysis/chapter_screenplay_layout_unit.md",
    "novel/05_analysis/chapter_screenplay_unit_adaptation.md",
    "novel/05_analysis/chapter_shooting_design_unit.md",
    "novel/05_analysis/object_discovery.md",
    "novel/05_analysis/scene_profile_candidates.md",
    "novel/05_analysis/scene_occurrence_extraction.md",
    "novel/05_analysis/scene_transition_extraction.md",
    "novel/05_analysis/scene_entity_state_extraction.md",
    "novel/06_manga/manga_character_binding.md",
    "novel/06_manga/manga_speech_selection.md",
    "novel/06_manga/manga_visual_anchor_extraction.md",
    "novel/06_manga/manga_item_extraction.md",
    "novel/06_manga/manga_chapter_comic_synthesis.md",
    "novel/06_manga/manga_page_planning.md",
    "novel/06_manga/manga_page_drawing_prompt.md",
    "novel/07_protagonist/character_portrait_generation.md",
    "novel/08_video/chapter_video_builder_rules.md",
    "novel/08_video/chapter_video_segment_plan.md",
    "novel/08_video/chapter_video_provider_prompt.md",
    "novel/08_video/chapter_video_first_frame_provider_prompt.md",
    "novel/08_video/chapter_video_first_frame_prompts.md",
    "novel/08_video/chapter_video_first_frame_video_prompt.md",
    "novel/08_video/chapter_video_frame_prompts.md",
    "novel/08_video/chapter_video_prompt.md",
    "novel/08_video/chapter_video_chat_prompt.md",
    "novel/08_video/chapter_video_storyboard_prompts.md",
    "novel/08_video/chapter_video_storyboard_video_prompt.md",
    "novel/08_video/chapter_video_prompt_targeted_repair.md",
    "novel/09_galgame/script_adaptation.md",
    "novel/09_galgame/character_staging.md",
    "novel/09_galgame/visual_planning.md",
    "novel/09_galgame/branch_design.md",
];

pub(in crate::state) fn sync_contract_bound_prompts(
    source_dir: &Path,
    dest_dir: &Path,
) -> Result<(), String> {
    let mut changed = Vec::new();
    for rel_path in CONTRACT_BOUND_PROMPT_PATHS {
        let source_path = source_dir.join(rel_path);
        if !source_path.is_file() {
            continue;
        }
        let dest_path = dest_dir.join(rel_path);
        let source_content = std::fs::read(&source_path)
            .map_err(|err| format!("读取契约提示词失败：{}：{err}", source_path.display()))?;
        let needs_update = match std::fs::read(&dest_path) {
            Ok(dest_content) => dest_content != source_content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
            Err(err) => {
                return Err(format!(
                    "读取用户契约提示词失败：{}：{err}",
                    dest_path.display()
                ));
            }
        };
        if !needs_update {
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建契约提示词目录失败：{}：{err}", parent.display()))?;
        }
        write_file_atomically(&dest_path, &source_content)?;
        changed.push(*rel_path);
    }
    if !changed.is_empty() {
        tracing::info!(
            prompts = %changed.join("、"),
            source = ?source_dir,
            dest = ?dest_dir,
            "已同步后端契约绑定 prompts"
        );
    }
    Ok(())
}

fn write_file_atomically(path: &Path, content: &[u8]) -> Result<(), String> {
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, content)
        .map_err(|err| format!("写入临时文件失败：{}：{err}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        let primary = format!(
            "替换文件失败：{} -> {}：{err}",
            temp_path.display(),
            path.display()
        );
        crate::file_utils::error_with_rollback_file_cleanup(
            primary,
            std::slice::from_ref(&temp_path),
            "临时文件",
        )
    })
}
