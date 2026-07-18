use a4n_core::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use a4n_llm::structured_stream::StructuredLlmProgress;

pub fn chapter_analysis_root_field_progress(
    item_label: &'static str,
    milestones: &'static [FieldMilestone],
    span: ProgressSpan,
) -> StructuredLlmProgress {
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "",
            item_label,
            expected_items: None,
            milestones,
        },
        span,
    }
}
