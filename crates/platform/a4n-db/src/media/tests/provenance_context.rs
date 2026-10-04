use a4n_db_models::models::{
    chapter::chapter,
    generation::{generation_spec, prompt_artifact},
    project::{novel_project, project_source_snapshot},
};
use a4n_domain::media::{CreateMediaAssetRecordInput, MediaProvenanceRecordInput};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;

use crate::media::{MediaDbError, create_media_asset_with_provenance_record};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project_with_chapter(
    db: &impl crate::DbExecutor,
    title: &str,
) -> novel_project::Model {
    let project = crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("他推开门。".to_string()),
        word_count: Set(5),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    project
}

async fn video_project(
    db: &impl crate::DbExecutor,
    user_id: i64,
    title: &str,
) -> novel_project::Model {
    let root = text_project_with_chapter(db, title).await;
    crate::project::create_derived_project_record(
        db,
        user_id,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: format!("{title}视频"),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap()
}

async fn generation_spec_for_snapshot(
    db: &impl crate::DbExecutor,
    project: &novel_project::Model,
    snapshot_id: i64,
) -> generation_spec::Model {
    let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
        .one(db)
        .await
        .unwrap()
        .expect("source snapshot must exist");
    let now = Utc::now().naive_utc();
    generation_spec::ActiveModel {
        project_id: Set(project.id),
        root_project_id: Set(project.root_project_id.unwrap()),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        target_kind: Set("final_assembly".to_string()),
        target_entity_kind: Set("assembly".to_string()),
        target_entity_id: Set(None),
        status: Set(crate::generation::SPEC_STATUS_DRAFT.to_string()),
        source_version_hash: Set(snapshot.source_version_hash),
        params_json: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn generation_spec(
    db: &impl crate::DbExecutor,
    project: &novel_project::Model,
) -> generation_spec::Model {
    generation_spec_for_snapshot(db, project, project.source_snapshot_id.unwrap()).await
}

async fn additional_snapshot(
    db: &impl crate::DbExecutor,
    root_project_id: i64,
) -> project_source_snapshot::Model {
    crate::project::create_source_snapshot_in_record(
        db,
        root_project_id,
        crate::project::DerivedChapterScope {
            start_chapter: 1,
            end_chapter: 1,
        },
        Utc::now().naive_utc(),
    )
    .await
    .unwrap()
}

async fn prompt_artifact_for_spec(
    db: &impl crate::DbExecutor,
    spec_id: i64,
) -> prompt_artifact::Model {
    let now = Utc::now().naive_utc();
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(spec_id),
        prompt_kind: Set("chat_video_prompt".to_string()),
        prompt_text: Set("自然语言视频提示词".to_string()),
        renderer_version: Set("test".to_string()),
        edited_by_user: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

fn media_input(project: &novel_project::Model) -> CreateMediaAssetRecordInput {
    CreateMediaAssetRecordInput {
        project_id: project.id,
        root_project_id: project.root_project_id.unwrap(),
        source_snapshot_id: project.source_snapshot_id,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "generated".to_string(),
        file_path: None,
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: None,
        checksum: None,
        status: "pending".to_string(),
        error_message: None,
        provenance: MediaProvenanceRecordInput {
            generation_spec_id: None,
            prompt_artifact_id: None,
            provider_job_id: None,
            source_media_asset_id: None,
            source_external_path: None,
            imported_from_user: false,
            request_snapshot_json: None,
        },
    }
}

#[tokio::test]
async fn create_rejects_generation_spec_from_another_project_family() {
    let db = setup_db().await;
    let first = video_project(&db, 1, "根文本A").await;
    let second = video_project(&db, 1, "根文本B").await;
    let second_spec = generation_spec(&db, &second).await;
    let mut input = media_input(&first);
    input.provenance.generation_spec_id = Some(second_spec.id);

    let error = create_media_asset_with_provenance_record(&db, input)
        .await
        .expect_err("media provenance must not reference another project family");

    assert!(matches!(
        error,
        MediaDbError::InvalidArgument(message) if message.contains("不属于媒体资产项目族")
    ));
}

#[tokio::test]
async fn create_rejects_generation_spec_from_another_source_snapshot() {
    let db = setup_db().await;
    let project = video_project(&db, 1, "快照生成意图").await;
    let other_snapshot = additional_snapshot(&db, project.root_project_id.unwrap()).await;
    let other_spec = generation_spec_for_snapshot(&db, &project, other_snapshot.id).await;
    let mut input = media_input(&project);
    input.provenance.generation_spec_id = Some(other_spec.id);

    let error = create_media_asset_with_provenance_record(&db, input)
        .await
        .expect_err("generation spec snapshot must match media snapshot");

    assert!(matches!(
        error,
        MediaDbError::InvalidArgument(message)
            if message.contains("source_snapshot_id 与媒体资产不一致")
    ));
}

#[tokio::test]
async fn create_accepts_prompt_artifact_without_repeated_generation_spec_id() {
    let db = setup_db().await;
    let project = video_project(&db, 1, "提示词来源").await;
    let spec = generation_spec(&db, &project).await;
    let artifact = prompt_artifact_for_spec(&db, spec.id).await;
    let mut input = media_input(&project);
    input.provenance.prompt_artifact_id = Some(artifact.id);

    let asset = crate::media::create_media_asset_with_provenance_read_model(&db, input)
        .await
        .unwrap();

    assert!(asset.id > 0);
    assert_eq!(asset.project_id, project.id);
    assert_eq!(asset.root_project_id, project.root_project_id.unwrap());
    assert_eq!(asset.source_snapshot_id, project.source_snapshot_id);
}

#[tokio::test]
async fn create_rejects_prompt_artifact_from_another_source_snapshot() {
    let db = setup_db().await;
    let project = video_project(&db, 1, "提示词快照").await;
    let other_snapshot = additional_snapshot(&db, project.root_project_id.unwrap()).await;
    let other_spec = generation_spec_for_snapshot(&db, &project, other_snapshot.id).await;
    let artifact = prompt_artifact_for_spec(&db, other_spec.id).await;
    let mut input = media_input(&project);
    input.provenance.prompt_artifact_id = Some(artifact.id);

    let error = create_media_asset_with_provenance_record(&db, input)
        .await
        .expect_err("prompt artifact snapshot must match media snapshot");

    assert!(matches!(
        error,
        MediaDbError::InvalidArgument(message)
            if message.contains("source_snapshot_id 与媒体资产不一致")
    ));
}
