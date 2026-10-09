use a4n_db::media_authoring::{
    SaveAuthoringDocument, adopt_authoring_document, save_authoring_document,
};
use a4n_db_models::models::project::source_snapshot_chapter;
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentRecord, AuthoringSourceRef,
    legacy::{HistoricalGalgameScene, historical_galgame_unit},
};
use sea_orm::DatabaseConnection;
use serde_json::json;

pub async fn adopt_historical_game_plan(
    db: &DatabaseConnection,
    project_id: i64,
    source: &source_snapshot_chapter::Model,
) -> AuthoringDocumentRecord {
    let units = (1..=2)
        .map(|id| {
            historical_galgame_unit(HistoricalGalgameScene {
                id,
                title: format!("已有场景 {id}"),
                location: None,
                participants: Vec::new(),
                lines: Vec::new(),
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let input = SaveAuthoringDocument {
        project_id,
        source_snapshot_id: source.snapshot_id,
        source_chapter_id: Some(source.source_chapter_id),
        kind: AuthoringDocumentKind::AdaptationPlan,
        record_id: None,
        expected_revision: None,
        inputs: AuthoringDocumentInputs {
            sources: vec![AuthoringSourceRef {
                root_project_id: source.root_project_id,
                source_chapter_id: source.source_chapter_id,
                source_snapshot_id: source.snapshot_id,
                source_snapshot_chapter_id: source.id,
                content_sha256: source.content_hash.clone(),
            }],
            video_reference: None,
            dependencies: Vec::new(),
            preprocessing_fingerprint: None,
        },
        document: json!({"schema_version":"adaptation_plan_v1", "target":"galgame", "intention":"保留已有场景单元", "decisions":[], "units":units}),
    };
    let doc = save_authoring_document(db, input).await.unwrap();
    adopt_authoring_document(db, project_id, doc.id, doc.revision)
        .await
        .unwrap()
}
