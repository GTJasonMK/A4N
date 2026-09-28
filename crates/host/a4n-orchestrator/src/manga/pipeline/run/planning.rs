mod finalization;
mod request;

use crate::media_authoring::authoring_error;
use a4n_api_contract::manga::MangaPipelineStopAfter;
use a4n_db::media_authoring as records;
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentStatus,
};
use a4n_error::AppError;

use self::finalization::finalize_planning;
use self::request::generate_planning_trace;
use super::contracts::{MangaExtractionRun, MangaPlanningRun, MangaRunContext};

pub(super) async fn run_planning(
    context: &MangaRunContext<'_>,
    extraction: MangaExtractionRun,
) -> Result<MangaPlanningRun, AppError> {
    let db = context.service.database.connection();
    let snapshot_id = context.adaptation.record.source_snapshot_id;
    let inputs = AuthoringDocumentInputs {
        sources: context.adaptation.record.inputs.sources.clone(),
        video_reference: None,
        dependencies: vec![context.adaptation.record.reference()],
    };
    let previous = records::editable_authoring_document(
        db,
        context.project_id,
        snapshot_id,
        Some(context.chapter_id),
        AuthoringDocumentKind::MangaPagePlan,
    )
    .await
    .map_err(authoring_error)?;
    if context.stop_after != Some(MangaPipelineStopAfter::PagePlanning)
        && let Some(record) = previous.as_ref().filter(|record| record.inputs == inputs)
    {
        let plan = super::super::json::parse_manga_page_plan(
            &record.document,
            &context.adaptation,
            "采用已保存分页分镜",
        )?;
        if record.status == AuthoringDocumentStatus::Draft {
            records::adopt_authoring_document(db, context.project_id, record.id, record.revision)
                .await
                .map_err(authoring_error)?;
        }
        context
            .advance_stage(super::super::super::MangaPipelineStage::DrawingPrompt)
            .await?;
        return Ok(MangaPlanningRun {
            extraction,
            plan,
            page_plan_ref: record.reference(),
        });
    }
    let (plan_trace, plan) = generate_planning_trace(context, &extraction).await?;
    let draft = previous.filter(|record| record.status == AuthoringDocumentStatus::Draft);
    let record = records::save_authoring_document(
        db,
        records::SaveAuthoringDocument {
            project_id: context.project_id,
            source_snapshot_id: snapshot_id,
            source_chapter_id: Some(context.chapter_id),
            kind: AuthoringDocumentKind::MangaPagePlan,
            record_id: draft.as_ref().map(|record| record.id),
            expected_revision: draft.map(|record| record.revision),
            inputs,
            document: serde_json::to_value(&plan)?,
        },
    )
    .await
    .map_err(authoring_error)?;
    if context.stop_after != Some(MangaPipelineStopAfter::PagePlanning) {
        records::adopt_authoring_document(db, context.project_id, record.id, record.revision)
            .await
            .map_err(authoring_error)?;
    }
    finalize_planning(context, extraction, plan_trace, plan, record.reference()).await
}
