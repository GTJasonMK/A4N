use a4n_runtime::host_api::outline_records as outline_actions;
use outline_actions::{
    ContinuePartOutlinesInput, GeneratePartsInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput,
};

outline_preview_command!(
    preview_generate_parts_request,
    GeneratePartsInput,
    outline_actions::preview_generate_parts_request,
);

outline_preview_command!(
    preview_regenerate_parts_request,
    RegeneratePartsInput,
    outline_actions::preview_regenerate_parts_request,
);

outline_preview_command!(
    preview_regenerate_last_part_request,
    RegenerateLastPartInput,
    outline_actions::preview_regenerate_last_part_request,
);

outline_preview_command!(
    preview_continue_part_outlines_request,
    ContinuePartOutlinesInput,
    outline_actions::preview_continue_part_outlines_request,
);

outline_preview_command!(
    preview_regenerate_part_outline_request,
    RegeneratePartOutlineInput,
    outline_actions::preview_regenerate_part_outline_request,
);
