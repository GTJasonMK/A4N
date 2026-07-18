use crate::state::ServerState;
use a4n_core::event::channel::TaskType;
use serde_json::Value;

use super::super::super::{
    ApiError, command_input, finish_web_task, outline_records_actions, register_web_task,
    sanitize_optional_task_id, to_value, web_progress_reporter,
};
use super::task::outline_web_generation_command;

outline_web_generation_command!(
    generate_parts,
    outline_records_actions::GeneratePartsInput,
    outline_records_actions::generate_parts_task_scope,
    "启动分部大纲生成",
    "分部大纲已生成",
    outline_records_actions::generate_parts,
);

outline_web_generation_command!(
    regenerate_parts,
    outline_records_actions::RegeneratePartsInput,
    outline_records_actions::regenerate_parts_task_scope,
    "启动分部大纲重生成",
    "已重新生成所有部",
    outline_records_actions::regenerate_parts,
);

outline_web_generation_command!(
    regenerate_last_part,
    outline_records_actions::RegenerateLastPartInput,
    outline_records_actions::regenerate_last_part_task_scope,
    "正在重新生成最后一部",
    "最后一部已重新生成",
    outline_records_actions::regenerate_last_part,
);

outline_web_generation_command!(
    regenerate_part_outline,
    outline_records_actions::RegeneratePartOutlineInput,
    outline_records_actions::regenerate_part_outline_task_scope,
    "正在重新生成指定分部",
    "指定分部已重新生成",
    outline_records_actions::regenerate_part_outline,
);

outline_web_generation_command!(
    continue_part_outlines,
    outline_records_actions::ContinuePartOutlinesInput,
    outline_records_actions::continue_part_outlines_task_scope,
    "正在续作分部大纲",
    "分部大纲续作完成",
    outline_records_actions::continue_part_outlines,
);

outline_web_generation_command!(
    generate_part_chapter_outlines,
    outline_records_actions::GeneratePartChapterOutlinesInput,
    outline_records_actions::generate_part_chapter_outlines_task_scope,
    "启动本部章节大纲生成",
    "本部章节大纲已生成",
    outline_records_actions::generate_part_chapter_outlines,
);

outline_web_generation_command!(
    batch_generate_part_outlines,
    outline_records_actions::BatchGeneratePartOutlinesInput,
    outline_records_actions::batch_generate_part_outlines_task_scope,
    "启动批量章节大纲生成",
    "批量章节大纲已生成",
    outline_records_actions::batch_generate_part_outlines,
);

outline_web_generation_command!(
    apply_beat_mapping,
    outline_records_actions::ApplyBeatMappingInput,
    outline_records_actions::apply_beat_mapping_task_scope,
    "正在映射节拍表",
    "节拍表映射完成",
    outline_records_actions::apply_beat_mapping,
);

outline_web_generation_command!(
    generate_chapter_outlines_by_count,
    outline_records_actions::GenerateChapterOutlinesByCountInput,
    outline_records_actions::generate_chapter_outlines_by_count_task_scope,
    "启动章节大纲生成",
    "章节大纲已生成",
    outline_records_actions::generate_chapter_outlines_by_count,
);

outline_web_generation_command!(
    regenerate_chapter_outline,
    outline_records_actions::RegenerateChapterOutlineInput,
    outline_records_actions::regenerate_chapter_outline_task_scope,
    "正在重新生成章节大纲",
    "章节大纲已重新生成",
    outline_records_actions::regenerate_chapter_outline,
);
