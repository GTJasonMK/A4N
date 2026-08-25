use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanRecordSummary, ChapterVisualPlanSourceSpanView, ChapterVisualPlanSourceView,
};
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;

pub use a4n_api_contract::chapter_visual_plan::{
    ApplyChapterVisualPlanProposalInput, ChapterVisualPlanProposalView,
    ChapterVisualPlanRecordInput, ChapterVisualPlanSetView, ChapterVisualPlanSourceInput,
    ChapterVisualPlanView, ConfirmedChapterVisualPlanView, GenerateChapterVisualPlanDraftInput,
    ProposeChapterVisualPlanRevisionInput, SaveChapterVisualPlanDraftInput,
};

pub(super) fn plan_view(value: ParsedChapterVisualPlan) -> ChapterVisualPlanView {
    ChapterVisualPlanView {
        record: ChapterVisualPlanRecordSummary {
            id: value.record.id,
            plan_version: value.record.plan_version,
            revision: value.record.revision,
            status: value.status,
            authoring_mode: value.authoring_mode,
            source_content_sha256: value.record.source_content_sha256,
            plan_sha256: value.record.plan_sha256,
            confirmed_at: value.record.confirmed_at.map(|value| value.to_string()),
        },
        plan: value.plan,
    }
}

pub(super) fn plan_set_view(
    value: a4n_orchestrator::chapter_visual_plan::ResolvedChapterVisualPlanSet,
) -> ChapterVisualPlanSetView {
    ChapterVisualPlanSetView {
        source: ChapterVisualPlanSourceView {
            chapter_number: value.source.chapter_number,
            title: value.source.title,
            content: value.source.content,
            content_sha256: value.source.content_sha256,
            source_spans: value
                .source_spans
                .into_iter()
                .map(|span| ChapterVisualPlanSourceSpanView {
                    id: span.id,
                    text: span.text,
                    paragraph_index: span.paragraph_index,
                })
                .collect(),
        },
        starter_plan: value.starter_plan,
        draft: value.plans.draft.map(plan_view),
        confirmed: value.plans.confirmed.map(plan_view),
    }
}
