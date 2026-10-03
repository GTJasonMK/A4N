use std::collections::BTreeMap;

use a4n_api_contract::media_authoring::{
    AuthoringDocumentView, MediaAuthoringArtifactKind, MediaAuthoringArtifactView,
};
use a4n_db::media_authoring::AuthoringArtifactSource;
use a4n_domain::media_authoring::{
    AdaptationTarget, AuthoringDocumentKind, AuthoringDocumentRef, AuthoringDocumentStatus,
};
use a4n_error::AppError;

impl crate::OrchestratorContext {
    pub(super) async fn authoring_artifact_views(
        &self,
        project_id: i64,
        target: AdaptationTarget,
        snapshot_id: i64,
        chapter_id: Option<i64>,
        documents: &[AuthoringDocumentView],
    ) -> Result<Vec<MediaAuthoringArtifactView>, AppError> {
        let sources = a4n_db::media_authoring::authoring_artifact_sources(
            self.database.connection(),
            project_id,
            target,
            chapter_id,
        )
        .await
        .map_err(super::authoring_error)?;
        Ok(artifact_views(sources, snapshot_id, documents))
    }
}

fn artifact_views(
    sources: Vec<AuthoringArtifactSource>,
    snapshot_id: i64,
    documents: &[AuthoringDocumentView],
) -> Vec<MediaAuthoringArtifactView> {
    let current = |id| {
        documents.iter().find(|view| {
            view.record.id == id
                && view.record.status == AuthoringDocumentStatus::Adopted
                && !view.stale
        })
    };
    let matches = |reference: &AuthoringDocumentRef| {
        current(reference.id).is_some_and(|view| view.record.reference() == *reference)
    };
    let mut result = Vec::new();
    let mut chapters = BTreeMap::new();
    for source in sources {
        match source {
            AuthoringArtifactSource::MangaPage {
                chapter_id,
                source,
                page_plan_ref,
            } => {
                let entry = chapters.entry(chapter_id).or_insert((0, false));
                entry.0 += 1;
                entry.1 |= source.source_snapshot_id != Some(snapshot_id)
                    || !matches(&AuthoringDocumentRef {
                        id: source.adaptation_plan_id,
                        revision: source.adaptation_plan_revision,
                        sha256: source.adaptation_plan_sha256,
                    })
                    || page_plan_ref
                        .as_ref()
                        .is_none_or(|reference| !matches(reference));
            }
            AuthoringArtifactSource::GalgameScript {
                source_snapshot_id,
                branch_plan_id,
                scene_plan_ids,
            } => {
                let branch_stale = match branch_plan_id {
                    Some(id) => current(id).is_none(),
                    None => documents.iter().any(|view| {
                        view.record.kind == AuthoringDocumentKind::GalgameBranchPlan
                            && view.record.status == AuthoringDocumentStatus::Adopted
                    }),
                };
                result.push(MediaAuthoringArtifactView {
                    kind: MediaAuthoringArtifactKind::GalgameScript,
                    source_chapter_id: None,
                    count: scene_plan_ids.len(),
                    stale: source_snapshot_id != snapshot_id
                        || branch_stale
                        || scene_plan_ids
                            .iter()
                            .flatten()
                            .any(|id| current(*id).is_none()),
                });
            }
        }
    }
    result.extend(chapters.into_iter().map(|(chapter_id, (count, stale))| {
        MediaAuthoringArtifactView {
            kind: MediaAuthoringArtifactKind::MangaPages,
            source_chapter_id: Some(chapter_id),
            count,
            stale,
        }
    }));
    result
}
