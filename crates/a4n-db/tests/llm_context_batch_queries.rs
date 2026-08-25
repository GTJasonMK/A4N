use a4n_core::models::generation::llm_context_pack;
use a4n_db::llm_context::{
    RecordOrReuseContextPackInput, record_or_reuse_context_pack_references,
    record_or_reuse_context_packs,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;

fn input(project_id: i64, pack_kind: &str) -> RecordOrReuseContextPackInput {
    RecordOrReuseContextPackInput {
        project_id,
        root_project_id: project_id,
        scope_kind: "project".to_string(),
        workflow: "batch-test".to_string(),
        pack_kind: pack_kind.to_string(),
        source_refs_json: Some(json!({"source": pack_kind})),
        config_hash: "config-v1".to_string(),
        content_sha256: format!("sha-{pack_kind}"),
        canonical_content_json: json!({"kind": pack_kind}),
        canonical_content_text: format!("{{\"kind\":\"{pack_kind}\"}}"),
        compression_level: "full_raw".to_string(),
        token_estimate: 3,
    }
}

#[tokio::test]
async fn context_pack_batch_reads_and_reuses_identity_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("上下文包批量读取 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("project must be created");
    let inputs = vec![
        input(project.id, "blueprint"),
        input(project.id, "parts"),
        input(project.id, "blueprint"),
    ];

    let first = record_or_reuse_context_packs(&db, inputs.clone())
        .await
        .expect("batch insert must succeed");
    let second = record_or_reuse_context_packs(&db, inputs)
        .await
        .expect("batch reuse must succeed");

    assert_eq!(
        first.iter().map(|row| row.id).collect::<Vec<_>>(),
        second.iter().map(|row| row.id).collect::<Vec<_>>()
    );
    assert_eq!(first[0].id, first[2].id);
    let references = record_or_reuse_context_pack_references(
        &db,
        vec![input(project.id, "blueprint"), input(project.id, "parts")],
    )
    .await
    .expect("reference batch reuse must succeed");
    assert_eq!(
        references.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![first[0].id, first[1].id]
    );
    assert!(references.iter().all(|row| row.token_estimate == 3));
    let count = llm_context_pack::Entity::find()
        .filter(llm_context_pack::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .expect("context pack count must succeed");
    assert_eq!(count, 2);
}
