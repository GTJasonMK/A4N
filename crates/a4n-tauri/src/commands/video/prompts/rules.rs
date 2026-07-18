use a4n_runtime::host_api::video_prompts::ChapterVideoPromptSummary;

pub(super) fn frame_prompt_generation_success_summary(
    rows: &[ChapterVideoPromptSummary],
) -> String {
    format!(
        "已生成 {} 条帧提示词记录",
        generated_frame_prompt_record_count(rows)
    )
}

fn generated_frame_prompt_record_count(rows: &[ChapterVideoPromptSummary]) -> usize {
    rows.iter()
        .filter(|row| is_generated_frame_prompt_type(&row.prompt_type))
        .count()
}

fn is_generated_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt"
    )
}

#[cfg(test)]
mod tests {
    use super::is_generated_frame_prompt_type;

    #[test]
    fn frame_prompt_success_count_excludes_upstream_records() {
        let prompt_types = [
            "frame_segment_plan",
            "frame_prompt",
            "first_frame_prompt",
            "chat_video_frame_prompt",
            "video_prompt",
        ];
        let count = prompt_types
            .iter()
            .filter(|prompt_type| is_generated_frame_prompt_type(prompt_type))
            .count();

        assert_eq!(count, 3);
    }
}
