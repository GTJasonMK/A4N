use crate::state::ServerState;
use serde_json::Value;

use super::super::super::{ApiError, command_input, outline_records_actions, to_value};
use super::task::outline_web_preview_command;

outline_web_preview_command!(
    preview_generate_parts_request,
    outline_records_actions::GeneratePartsInput,
    outline_records_actions::preview_generate_parts_request,
);

outline_web_preview_command!(
    preview_regenerate_parts_request,
    outline_records_actions::RegeneratePartsInput,
    outline_records_actions::preview_regenerate_parts_request,
);

outline_web_preview_command!(
    preview_regenerate_last_part_request,
    outline_records_actions::RegenerateLastPartInput,
    outline_records_actions::preview_regenerate_last_part_request,
);

outline_web_preview_command!(
    preview_regenerate_part_outline_request,
    outline_records_actions::RegeneratePartOutlineInput,
    outline_records_actions::preview_regenerate_part_outline_request,
);

outline_web_preview_command!(
    preview_continue_part_outlines_request,
    outline_records_actions::ContinuePartOutlinesInput,
    outline_records_actions::preview_continue_part_outlines_request,
);

outline_web_preview_command!(
    preview_generate_part_chapter_outlines_request,
    outline_records_actions::GeneratePartChapterOutlinesInput,
    outline_records_actions::preview_generate_part_chapter_outlines_request,
);

outline_web_preview_command!(
    preview_batch_generate_part_outlines_request,
    outline_records_actions::BatchGeneratePartOutlinesInput,
    outline_records_actions::preview_batch_generate_part_outlines_request,
);

outline_web_preview_command!(
    preview_apply_beat_mapping_request,
    outline_records_actions::ApplyBeatMappingInput,
    outline_records_actions::preview_apply_beat_mapping_request,
);

outline_web_preview_command!(
    preview_generate_chapter_outlines_by_count_request,
    outline_records_actions::GenerateChapterOutlinesByCountInput,
    outline_records_actions::preview_generate_chapter_outlines_by_count_request,
);

outline_web_preview_command!(
    preview_regenerate_chapter_outline_request,
    outline_records_actions::RegenerateChapterOutlineInput,
    outline_records_actions::preview_regenerate_chapter_outline_request,
);
