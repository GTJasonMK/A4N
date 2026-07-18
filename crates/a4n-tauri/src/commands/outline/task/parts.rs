use a4n_runtime::host_api::outline_records as outline_actions;
use outline_actions::{
    ContinuePartOutlinesInput, GeneratePartsInput, PartOutlineSummary, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput,
};

outline_generation_command!(
    generate_parts,
    GeneratePartsInput,
    Vec<PartOutlineSummary>,
    outline_actions::generate_parts_task_scope,
    |_input| "启动分部大纲生成",
    "outline_generate_failed",
    "分部大纲已生成",
    outline_actions::generate_parts,
);

outline_preview_command!(
    preview_generate_parts_request,
    GeneratePartsInput,
    outline_actions::preview_generate_parts_request,
);

outline_generation_command!(
    regenerate_parts,
    RegeneratePartsInput,
    Vec<PartOutlineSummary>,
    outline_actions::regenerate_parts_task_scope,
    |_input| "启动分部大纲重生成",
    "outline_regenerate_failed",
    "已重新生成所有部",
    outline_actions::regenerate_parts,
);

outline_preview_command!(
    preview_regenerate_parts_request,
    RegeneratePartsInput,
    outline_actions::preview_regenerate_parts_request,
);

outline_generation_command!(
    regenerate_last_part,
    RegenerateLastPartInput,
    PartOutlineSummary,
    outline_actions::regenerate_last_part_task_scope,
    |_input| "正在重新生成最后一部",
    "outline_regenerate_last_failed",
    "最后一部已重新生成",
    outline_actions::regenerate_last_part,
);

outline_preview_command!(
    preview_regenerate_last_part_request,
    RegenerateLastPartInput,
    outline_actions::preview_regenerate_last_part_request,
);

outline_generation_command!(
    continue_part_outlines,
    ContinuePartOutlinesInput,
    PartOutlineSummary,
    outline_actions::continue_part_outlines_task_scope,
    |input| format!("正在续作第 {} 部", input.part_number),
    "outline_continue_part_failed",
    "分部续作完成",
    outline_actions::continue_part_outlines,
);

outline_preview_command!(
    preview_continue_part_outlines_request,
    ContinuePartOutlinesInput,
    outline_actions::preview_continue_part_outlines_request,
);

outline_generation_command!(
    regenerate_part_outline,
    RegeneratePartOutlineInput,
    PartOutlineSummary,
    outline_actions::regenerate_part_outline_task_scope,
    |_input| "正在重新生成指定分部",
    "outline_part_regenerate_failed",
    "指定分部已重新生成",
    outline_actions::regenerate_part_outline,
);

outline_preview_command!(
    preview_regenerate_part_outline_request,
    RegeneratePartOutlineInput,
    outline_actions::preview_regenerate_part_outline_request,
);
