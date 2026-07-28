use a4n_runtime::host_api::outline_records as outline_actions;
use outline_actions::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, GenerateChapterOutlinesByCountInput,
    GeneratePartChapterOutlinesInput, RegenerateChapterOutlineInput,
};

outline_preview_command!(
    preview_regenerate_chapter_outline_request,
    RegenerateChapterOutlineInput,
    outline_actions::preview_regenerate_chapter_outline_request,
);

outline_preview_command!(
    preview_generate_part_chapter_outlines_request,
    GeneratePartChapterOutlinesInput,
    outline_actions::preview_generate_part_chapter_outlines_request,
);

outline_preview_command!(
    preview_batch_generate_part_outlines_request,
    BatchGeneratePartOutlinesInput,
    outline_actions::preview_batch_generate_part_outlines_request,
);

outline_preview_command!(
    preview_apply_beat_mapping_request,
    ApplyBeatMappingInput,
    outline_actions::preview_apply_beat_mapping_request,
);

outline_preview_command!(
    preview_generate_chapter_outlines_by_count_request,
    GenerateChapterOutlinesByCountInput,
    outline_actions::preview_generate_chapter_outlines_by_count_request,
);
