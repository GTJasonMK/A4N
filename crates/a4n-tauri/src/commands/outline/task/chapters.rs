use a4n_runtime::host_api::outline_records as outline_actions;
use outline_actions::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlineSummary,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput,
    RegenerateChapterOutlineInput,
};

outline_generation_command!(
    regenerate_chapter_outline,
    RegenerateChapterOutlineInput,
    ChapterOutlineSummary,
    outline_actions::regenerate_chapter_outline_task_scope,
    |_input| "启动章节大纲重生成",
    "outline_chapter_regenerate_failed",
    "章节大纲已重新生成",
    outline_actions::regenerate_chapter_outline,
);

outline_preview_command!(
    preview_regenerate_chapter_outline_request,
    RegenerateChapterOutlineInput,
    outline_actions::preview_regenerate_chapter_outline_request,
);

outline_generation_command!(
    generate_part_chapter_outlines,
    GeneratePartChapterOutlinesInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::generate_part_chapter_outlines_task_scope,
    |_input| "启动本部章节大纲生成",
    "outline_part_chapters_generate_failed",
    "本部章节大纲已生成",
    outline_actions::generate_part_chapter_outlines,
);

outline_preview_command!(
    preview_generate_part_chapter_outlines_request,
    GeneratePartChapterOutlinesInput,
    outline_actions::preview_generate_part_chapter_outlines_request,
);

outline_generation_command!(
    batch_generate_part_outlines,
    BatchGeneratePartOutlinesInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::batch_generate_part_outlines_task_scope,
    |_input| "启动批量章节大纲生成",
    "outline_batch_chapters_generate_failed",
    "批量章节大纲已生成",
    outline_actions::batch_generate_part_outlines,
);

outline_preview_command!(
    preview_batch_generate_part_outlines_request,
    BatchGeneratePartOutlinesInput,
    outline_actions::preview_batch_generate_part_outlines_request,
);

outline_generation_command!(
    apply_beat_mapping,
    ApplyBeatMappingInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::apply_beat_mapping_task_scope,
    |_input| "启动节拍映射",
    "outline_beat_mapping_failed",
    "节拍映射已应用",
    outline_actions::apply_beat_mapping,
);

outline_preview_command!(
    preview_apply_beat_mapping_request,
    ApplyBeatMappingInput,
    outline_actions::preview_apply_beat_mapping_request,
);

outline_generation_command!(
    generate_chapter_outlines_by_count,
    GenerateChapterOutlinesByCountInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::generate_chapter_outlines_by_count_task_scope,
    |_input| "启动章节大纲追加生成",
    "outline_chapters_generate_failed",
    "章节大纲已生成",
    outline_actions::generate_chapter_outlines_by_count,
);

outline_preview_command!(
    preview_generate_chapter_outlines_by_count_request,
    GenerateChapterOutlinesByCountInput,
    outline_actions::preview_generate_chapter_outlines_by_count_request,
);
