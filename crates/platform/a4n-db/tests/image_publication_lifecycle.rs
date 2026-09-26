//! 图片发布 A1-A5 的数据库隔离故障矩阵。
//!
//! 每个测试通过 `connect_test_database` 使用独立 schema。文件写入、rename、真实
//! server/provider ACK、cleanup worker 的引用复核和物理 unlink 必须由运行时/端到端
//! 测试覆盖；本文件只验证数据库能证明的持久化不变量。

use a4n_db::cleanup_outbox::{
    CLEANUP_STATUS_COMPLETED, FileArtifactCleanupPayload, claim_file_artifact_cleanup,
    complete_claimed_cleanup, insert_file_artifact_cleanup_intent,
};
use a4n_db::media::filter_unreferenced_media_paths;
use a4n_db::profile_material::upsert_profile_primary_material;
use a4n_db::project::delete::delete_project_cascade_record;
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db::video::{
    create_image_publication_intent, delete_generated_image_record_with_cleanup,
    image_publications_for_recovery, image_publications_for_recovery_for_model,
    mark_image_publication_reclaimed, mark_image_publication_reclaiming, publish_image_publication,
};
use a4n_db_models::models::{
    ai::generated_image, media::image_publication, project::novel_project, task::cleanup_outbox,
};
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateImagePublicationIntentInput,
    IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
    IMAGE_PUBLICATION_STATUS_RECLAIMED, IMAGE_PUBLICATION_STATUS_RECLAIMING,
    ImagePublicationFileFacts,
};
use a4n_migration::test_support::connect_test_database;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

struct Fixture {
    db: sea_orm::DatabaseConnection,
    user_id: i64,
    project_id: i64,
    publication_id: String,
    staging_path: String,
    final_path: String,
    sha256: String,
}

impl Fixture {
    fn intent_input(&self) -> CreateImagePublicationIntentInput {
        CreateImagePublicationIntentInput {
            publication_id: self.publication_id.clone(),
            project_id: self.project_id,
            requested_by_user_id: self.user_id,
            staging_path: self.staging_path.clone(),
            final_path: self.final_path.clone(),
            image: CreateGeneratedImageRecordDraftInput {
                project_id: self.project_id,
                image_type: "imported".to_string(),
                file_path: self.final_path.clone(),
                prompt: "database publication fault matrix".to_string(),
                model: "test".to_string(),
                width: 1,
                height: 1,
                file_size: Some(4),
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: None,
            },
            expected_file_size: 4,
            expected_sha256: self.sha256.clone(),
            allow_pending_replacement: false,
        }
    }

    fn file_facts(&self) -> ImagePublicationFileFacts {
        ImagePublicationFileFacts {
            file_size: 4,
            sha256: self.sha256.clone(),
        }
    }
}

async fn fixture() -> Fixture {
    let db = connect_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        &db,
        &format!("image_publication_{suffix}"),
        &format!("image_publication_{suffix}@example.invalid"),
        "image-publication-test-password",
        "user",
    )
    .await
    .expect("图片发布测试用户必须创建成功");
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("图片发布故障矩阵 {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("图片发布测试项目必须创建成功");

    Fixture {
        db,
        user_id: user.id,
        project_id: project.id,
        publication_id: format!("image-publication-{suffix}"),
        staging_path: format!("projects/{}/images/.staging/{suffix}.png", project.id),
        final_path: format!("projects/{}/images/{suffix}.png", project.id),
        sha256: "a".repeat(64),
    }
}

#[tokio::test]
async fn recovery_cursor_visits_later_publications_and_wraps_without_changing_records() {
    let fixture = fixture().await;
    let mut published = Vec::new();
    for index in 0..3 {
        let mut input = fixture.intent_input();
        input.publication_id = format!("recovery-{index}");
        input.staging_path = format!("projects/{}/images/{index}.part", fixture.project_id);
        input.final_path = format!("projects/{}/images/{index}.png", fixture.project_id);
        input.image.file_path = input.final_path.clone();
        let publication = create_image_publication_intent(&fixture.db, input)
            .await
            .unwrap();
        publish_image_publication(
            &fixture.db,
            &publication.publication_id,
            fixture.file_facts(),
        )
        .await
        .unwrap();
        published.push(
            image_publication::Entity::find_by_id(&publication.publication_id)
                .one(&fixture.db)
                .await
                .unwrap()
                .unwrap(),
        );
    }

    let first = image_publications_for_recovery(&fixture.db, 2, None)
        .await
        .unwrap();
    assert_eq!(first, published[..2]);
    let second = image_publications_for_recovery(
        &fixture.db,
        2,
        Some(&first.last().unwrap().publication_id),
    )
    .await
    .unwrap();
    assert_eq!(second, published[2..]);
    let wrapped = image_publications_for_recovery(&fixture.db, 2, Some(&second[0].publication_id))
        .await
        .unwrap();
    assert_eq!(
        wrapped, first,
        "audit progress must not mutate publication facts/timestamps"
    );

    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .unwrap();
    let mixed = image_publications_for_recovery(
        &fixture.db,
        2,
        Some(&first.last().unwrap().publication_id),
    )
    .await
    .unwrap();
    assert_eq!(mixed, vec![pending, published[2].clone()]);
}

#[tokio::test]
async fn imported_recovery_uses_provenance_for_pending_and_published_images() {
    let fixture = fixture().await;
    for (index, (image_type, model)) in [
        ("project_reference", a4n_domain::EXTERNAL_IMPORT_MODEL),
        ("project_reference", a4n_domain::EXTERNAL_IMPORT_MODEL),
        ("imported", "test"),
        ("project_reference", "test"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = fixture.intent_input();
        input.publication_id = format!("provenance-{index}");
        input.staging_path = format!("projects/{}/images/{index}.part", fixture.project_id);
        input.final_path = format!("projects/{}/images/{index}.png", fixture.project_id);
        input.image.file_path = input.final_path.clone();
        input.image.image_type = image_type.to_string();
        input.image.model = model.to_string();
        let publication = create_image_publication_intent(&fixture.db, input)
            .await
            .unwrap();
        if index % 2 == 1 {
            publish_image_publication(
                &fixture.db,
                &publication.publication_id,
                fixture.file_facts(),
            )
            .await
            .unwrap();
        }
    }
    let publications = image_publications_for_recovery_for_model(
        &fixture.db,
        10,
        None,
        Some(a4n_domain::EXTERNAL_IMPORT_MODEL),
    )
    .await
    .unwrap();
    assert_eq!(publications.len(), 2);
    assert_eq!(publications[0].publication_id, "provenance-0");
    assert_eq!(publications[1].publication_id, "provenance-1");
}

#[tokio::test]
async fn publication_intent_replay_is_idempotent_and_conflicting_payload_is_rejected() {
    let fixture = fixture().await;
    let (first, second) = tokio::join!(
        create_image_publication_intent(&fixture.db, fixture.intent_input()),
        create_image_publication_intent(&fixture.db, fixture.intent_input()),
    );
    let first = first.expect("并发首次发布意图必须成功");
    let second = second.expect("并发重放发布意图必须成功");

    assert_eq!(first, second);
    assert_eq!(first.status, IMAGE_PUBLICATION_STATUS_PENDING);
    assert_eq!(first.version, 1);

    let mut conflicting = fixture.intent_input();
    conflicting.expected_sha256 = "b".repeat(64);
    let error = create_image_publication_intent(&fixture.db, conflicting)
        .await
        .expect_err("同一 publication_id 不得绑定另一个文件事实");
    assert!(error.to_string().contains("不同的图片发布意图"));

    let stored = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("发布意图查询必须成功")
        .expect("发布意图必须保留");
    assert_eq!(stored, first);
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(fixture.project_id))
            .all(&fixture.db)
            .await
            .expect("图片记录查询必须成功")
            .is_empty()
    );
}

#[tokio::test]
async fn image_generation_retry_replaces_only_a_pending_intent() {
    let fixture = fixture().await;
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &fixture.db,
        a4n_domain::chapter::ImportChapterContentInput {
            project_id: fixture.project_id,
            chapter_number: 9,
            content: "图片生成重试章节".into(),
            title: None,
        },
        "image publication retry test",
    )
    .await
    .unwrap();
    create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("首次图片生成发布意图必须成功");

    let mut retry = fixture.intent_input();
    retry.allow_pending_replacement = true;
    retry.staging_path = format!("projects/{}/images/.staging/retry.part", fixture.project_id);
    retry.final_path = format!("projects/{}/images/retry.webp", fixture.project_id);
    retry.image.file_path = retry.final_path.clone();
    retry.image.model = "provider-retry".to_string();
    retry.image.width = 2;
    retry.image.height = 2;
    retry.image.prompt = "retry prompt".to_string();
    retry.image.chapter_id = Some(chapter.id);
    retry.image.panel_id = Some("panel-2".to_string());
    retry.image.file_size = Some(5);
    retry.image.metadata_json = Some(json!({"generation": {"attempt": 2}}));
    retry.expected_file_size = 5;
    retry.expected_sha256 = "b".repeat(64);

    let replaced = create_image_publication_intent(&fixture.db, retry)
        .await
        .expect("pending 图片生成意图应允许替换本次输出事实");

    assert_eq!(replaced.publication_id, fixture.publication_id);
    assert_eq!(replaced.version, 2);
    assert_eq!(replaced.expected_file_size, 5);
    assert_eq!(replaced.expected_sha256, "b".repeat(64));
    assert_eq!(replaced.prompt, "retry prompt");
    assert_eq!(replaced.chapter_id, Some(chapter.id));
    assert_eq!(replaced.panel_id.as_deref(), Some("panel-2"));
    assert_eq!(
        replaced.final_path,
        format!("projects/{}/images/retry.webp", fixture.project_id)
    );
    assert_eq!(replaced.status, IMAGE_PUBLICATION_STATUS_PENDING);

    let strict_replay = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect_err("普通发布意图仍必须拒绝不同输出事实");
    assert!(strict_replay.to_string().contains("不同的图片发布意图"));
}

#[tokio::test]
async fn publish_failure_before_commit_rolls_back_image_and_keeps_pending_intent() {
    let fixture = fixture().await;
    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须先提交");

    let mut invalid_facts = fixture.file_facts();
    invalid_facts.file_size = 3;
    let error = publish_image_publication(&fixture.db, &fixture.publication_id, invalid_facts)
        .await
        .expect_err("文件事实不匹配不得提交发布");
    assert!(error.to_string().contains("文件事实与发布意图不一致"));

    let mut injected: image_publication::ActiveModel = pending.clone().into();
    injected.version = Set(i64::MAX);
    let injected = injected
        .update(&fixture.db)
        .await
        .expect("数据库应接受仍为正数的版本故障注入");
    let error =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect_err("版本溢出必须在事务提交前失败");
    assert!(error.to_string().contains("更新图片发布版本"));

    let stored = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("发布意图查询必须成功")
        .expect("失败后发布意图必须保留");
    assert_eq!(stored, injected);
    assert_eq!(stored.status, IMAGE_PUBLICATION_STATUS_PENDING);
    assert_eq!(stored.image_id, None);
    assert!(stored.file_size.is_none());
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(fixture.project_id))
            .all(&fixture.db)
            .await
            .expect("失败后图片查询必须成功")
            .is_empty()
    );
    let recovery = image_publications_for_recovery(&fixture.db, 10, None)
        .await
        .expect("失败后恢复查询必须成功");
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].publication_id, fixture.publication_id);
    assert_eq!(recovery[0].status, IMAGE_PUBLICATION_STATUS_PENDING);
}

#[tokio::test]
async fn published_replay_returns_same_image_and_keeps_published_identity_recoverable() {
    let fixture = fixture().await;
    create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");
    let first =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect("首次发布必须成功");
    let mut conflicting_facts = fixture.file_facts();
    conflicting_facts.file_size = 3;
    let error = publish_image_publication(&fixture.db, &fixture.publication_id, conflicting_facts)
        .await
        .expect_err("已发布重放不得接受另一组文件事实");
    assert!(error.to_string().contains("已发布意图不一致"));
    let replayed =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect("已发布重放必须返回既有图片");

    assert_eq!(replayed.id, first.id);
    assert_eq!(replayed.file_path, fixture.final_path);
    let publication = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("已发布意图查询必须成功")
        .expect("已发布意图必须存在");
    assert_eq!(publication.status, IMAGE_PUBLICATION_STATUS_PUBLISHED);
    assert_eq!(publication.version, 2);
    assert_eq!(publication.image_id, Some(first.id));
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(fixture.project_id))
            .all(&fixture.db)
            .await
            .expect("已发布图片查询必须成功")
            .len(),
        1
    );

    let recovery = image_publications_for_recovery(&fixture.db, 10, None)
        .await
        .expect("已发布恢复查询必须成功");
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].status, IMAGE_PUBLICATION_STATUS_PUBLISHED);
    assert_eq!(recovery[0].image_id, Some(first.id));
}

#[tokio::test]
async fn deleting_published_image_moves_publication_into_recovery() {
    let fixture = fixture().await;
    create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");
    let image =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect("图片必须先成功发布");

    delete_generated_image_record_with_cleanup(&fixture.db, image.id)
        .await
        .expect("删除图片必须成功");

    let publication = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("发布意图查询必须成功")
        .expect("发布意图必须保留");
    assert_eq!(publication.status, IMAGE_PUBLICATION_STATUS_RECLAIMING);
    assert_eq!(publication.version, 3);
    assert_eq!(
        publication.last_error.as_deref(),
        Some("generated_image 记录已删除")
    );
}

#[tokio::test]
async fn published_replay_rejects_drifted_image_identity_instead_of_acknowledging() {
    let fixture = fixture().await;
    create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");
    let image =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect("首次发布必须成功");

    let mut corrupted: generated_image::ActiveModel = image.clone().into();
    corrupted.file_size = Set(Some(3));
    corrupted
        .update(&fixture.db)
        .await
        .expect("测试必须能注入已发布图片的文件事实漂移");

    let error =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect_err("已发布图片身份漂移时不得确认 ACK");
    assert!(error.to_string().contains("校验已发布图片身份"));

    let publication = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("漂移后的发布意图查询必须成功")
        .expect("发布意图必须存在");
    assert_eq!(publication.status, IMAGE_PUBLICATION_STATUS_PUBLISHED);
    assert_eq!(publication.image_id, Some(image.id));
    assert_eq!(publication.file_size, Some(4));
    assert_eq!(publication.file_sha256, Some(fixture.sha256));
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(fixture.project_id))
            .all(&fixture.db)
            .await
            .expect("漂移后的图片查询必须成功")
            .len(),
        1
    );
}

#[tokio::test]
async fn reclaim_and_publish_are_fenced_by_publication_version() {
    let fixture = fixture().await;
    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");

    let stale_claim = mark_image_publication_reclaiming(
        &fixture.db,
        &fixture.publication_id,
        pending.version + 1,
        "过期回收 owner".to_string(),
    )
    .await
    .expect_err("未来版本不得取得回收权");
    assert!(stale_claim.to_string().contains("转入图片回收"));
    assert_eq!(
        image_publication::Entity::find_by_id(&fixture.publication_id)
            .one(&fixture.db)
            .await
            .expect("过期回收后查询必须成功")
            .expect("发布意图必须存在"),
        pending
    );

    let reclaiming = mark_image_publication_reclaiming(
        &fixture.db,
        &fixture.publication_id,
        pending.version,
        "文件未完成".to_string(),
    )
    .await
    .expect("当前版本必须取得回收权");
    assert_eq!(reclaiming.status, IMAGE_PUBLICATION_STATUS_RECLAIMING);
    assert_eq!(reclaiming.version, 2);

    let stale_completion = mark_image_publication_reclaimed(
        &fixture.db,
        &fixture.publication_id,
        pending.version,
        None,
    )
    .await
    .expect_err("旧回收 owner 不得完成新版本回收");
    assert!(stale_completion.to_string().contains("完成图片回收"));
    let late_publish =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect_err("回收权取得后晚到发布不得写入图片");
    assert!(late_publish.to_string().contains("发布图片"));

    let reclaimed = mark_image_publication_reclaimed(
        &fixture.db,
        &fixture.publication_id,
        reclaiming.version,
        Some("cleanup 已处理".to_string()),
    )
    .await
    .expect("当前回收版本必须完成");
    assert_eq!(reclaimed.status, IMAGE_PUBLICATION_STATUS_RECLAIMED);
    assert_eq!(reclaimed.version, 3);
    let replayed = mark_image_publication_reclaimed(
        &fixture.db,
        &fixture.publication_id,
        reclaiming.version,
        None,
    )
    .await
    .expect("已回收终态重放必须幂等");
    assert_eq!(replayed, reclaimed);
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(fixture.project_id))
            .all(&fixture.db)
            .await
            .expect("晚到发布后的图片查询必须成功")
            .is_empty()
    );
    assert!(
        image_publications_for_recovery(&fixture.db, 10, None)
            .await
            .expect("已回收恢复查询必须成功")
            .is_empty()
    );
}

#[tokio::test]
async fn staging_cleanup_intent_is_idempotent_and_cannot_retarget_published_file() {
    let fixture = fixture().await;
    create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");
    let image =
        publish_image_publication(&fixture.db, &fixture.publication_id, fixture.file_facts())
            .await
            .expect("图片必须先成功发布");

    let dedupe_key = format!("image-publication-staging:{}", fixture.publication_id);
    let staging_intent = insert_file_artifact_cleanup_intent(
        &fixture.db,
        dedupe_key.clone(),
        FileArtifactCleanupPayload::new(fixture.project_id, vec![fixture.staging_path.clone()]),
    )
    .await
    .expect("暂存文件 cleanup intent 必须写入");
    let replayed = insert_file_artifact_cleanup_intent(
        &fixture.db,
        dedupe_key.clone(),
        FileArtifactCleanupPayload::new(fixture.project_id, vec![fixture.staging_path.clone()]),
    )
    .await
    .expect("相同暂存 cleanup intent 必须幂等");
    assert_eq!(replayed.id, staging_intent.id);

    let retargeted = insert_file_artifact_cleanup_intent(
        &fixture.db,
        dedupe_key,
        FileArtifactCleanupPayload::new(fixture.project_id, vec![fixture.final_path.clone()]),
    )
    .await
    .expect_err("暂存 cleanup intent 不得被重放为最终文件删除");
    assert!(retargeted.to_string().contains("已绑定其他 payload"));

    let claimed = claim_file_artifact_cleanup(&fixture.db, Some(fixture.project_id))
        .await
        .expect("暂存 cleanup intent 必须可领取")
        .expect("暂存 cleanup intent 必须存在");
    assert_eq!(claimed.record.id, staging_intent.id);
    assert_eq!(claimed.payload.paths, vec![fixture.staging_path.clone()]);
    assert!(!claimed.payload.paths.contains(&fixture.final_path));
    let lease_token = claimed
        .record
        .lease_token
        .as_deref()
        .expect("cleanup claim 必须有 lease");
    complete_claimed_cleanup(&fixture.db, claimed.record.id, lease_token)
        .await
        .expect("cleanup intent 完成必须成功");

    let cleanup = cleanup_outbox::Entity::find_by_id(staging_intent.id)
        .one(&fixture.db)
        .await
        .expect("cleanup intent 查询必须成功")
        .expect("cleanup intent 必须保留审计记录");
    assert_eq!(cleanup.status, CLEANUP_STATUS_COMPLETED);
    let publication = image_publication::Entity::find_by_id(&fixture.publication_id)
        .one(&fixture.db)
        .await
        .expect("cleanup 后发布意图查询必须成功")
        .expect("cleanup 不得删除发布意图");
    assert_eq!(publication.status, IMAGE_PUBLICATION_STATUS_PUBLISHED);
    assert_eq!(publication.image_id, Some(image.id));
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&fixture.db)
            .await
            .expect("cleanup 后图片查询必须成功")
            .is_some()
    );
}

#[tokio::test]
async fn reclaim_transitions_remain_available_while_project_is_draining() {
    let fixture = fixture().await;
    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");

    let project = novel_project::Entity::find_by_id(fixture.project_id)
        .one(&fixture.db)
        .await
        .expect("项目查询必须成功")
        .expect("项目必须存在");
    let mut draining: novel_project::ActiveModel = project.into();
    draining.lifecycle_gate = Set("deleting".to_string());
    draining
        .update(&fixture.db)
        .await
        .expect("测试必须把项目置为排空状态");

    let reclaiming = mark_image_publication_reclaiming(
        &fixture.db,
        &fixture.publication_id,
        pending.version,
        "项目删除排空".to_string(),
    )
    .await
    .expect("项目排空期间仍必须能取得回收权");
    assert_eq!(reclaiming.status, IMAGE_PUBLICATION_STATUS_RECLAIMING);

    let reclaimed = mark_image_publication_reclaimed(
        &fixture.db,
        &fixture.publication_id,
        reclaiming.version,
        None,
    )
    .await
    .expect("项目排空期间仍必须能完成回收状态");
    assert_eq!(reclaimed.status, IMAGE_PUBLICATION_STATUS_RECLAIMED);
}

#[tokio::test]
async fn reclaim_transitions_remain_available_after_project_is_deleted() {
    let fixture = fixture().await;
    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("发布意图必须创建成功");

    let project = novel_project::Entity::find_by_id(fixture.project_id)
        .one(&fixture.db)
        .await
        .expect("项目查询必须成功")
        .expect("项目必须存在");
    let mut deleting: novel_project::ActiveModel = project.into();
    deleting.lifecycle_gate = Set("deleting".to_string());
    deleting
        .update(&fixture.db)
        .await
        .expect("测试必须把项目置为排空状态");
    delete_project_cascade_record(&fixture.db, fixture.project_id)
        .await
        .expect("项目必须完成级联删除");

    let reclaiming = mark_image_publication_reclaiming(
        &fixture.db,
        &fixture.publication_id,
        pending.version,
        "项目已删除，回收孤儿发布意图".to_string(),
    )
    .await
    .expect("项目删除后仍必须能取得回收权");
    assert_eq!(reclaiming.status, IMAGE_PUBLICATION_STATUS_RECLAIMING);

    let reclaimed = mark_image_publication_reclaimed(
        &fixture.db,
        &fixture.publication_id,
        reclaiming.version,
        None,
    )
    .await
    .expect("项目删除后仍必须能完成回收状态");
    assert_eq!(reclaimed.status, IMAGE_PUBLICATION_STATUS_RECLAIMED);
}

#[tokio::test]
async fn reclaim_keeps_shared_final_path_out_of_cleanup_candidates() {
    let fixture = fixture().await;
    let survivor_image =
        a4n_db::video::create_generated_image_record(&fixture.db, fixture.intent_input().image)
            .await
            .expect("共享路径的存活图片必须创建成功");

    let pending = create_image_publication_intent(&fixture.db, fixture.intent_input())
        .await
        .expect("待回收发布意图必须创建成功");
    let reclaiming = mark_image_publication_reclaiming(
        &fixture.db,
        &pending.publication_id,
        pending.version,
        "共享最终文件仍被引用".to_string(),
    )
    .await
    .expect("共享路径发布意图必须进入回收中");

    let candidates = filter_unreferenced_media_paths(
        &fixture.db,
        &[fixture.staging_path.clone(), fixture.final_path.clone()],
    )
    .await
    .expect("回收前引用复核必须成功");
    assert_eq!(
        candidates,
        vec![fixture.staging_path.clone()],
        "已发布图片仍引用最终文件时，cleanup 只能保留最终路径并回收暂存路径"
    );

    let reclaimed = mark_image_publication_reclaimed(
        &fixture.db,
        &pending.publication_id,
        reclaiming.version,
        Some("共享最终文件仍被引用，保留最终文件".to_string()),
    )
    .await
    .expect("引用复核后回收状态必须可完成");
    assert_eq!(reclaimed.status, IMAGE_PUBLICATION_STATUS_RECLAIMED);
    assert_eq!(
        reclaimed.last_error.as_deref(),
        Some("共享最终文件仍被引用，保留最终文件")
    );
    assert_eq!(survivor_image.file_path, fixture.final_path);
    assert!(
        generated_image::Entity::find_by_id(survivor_image.id)
            .one(&fixture.db)
            .await
            .expect("共享图片查询必须成功")
            .is_some(),
        "回收另一条发布意图不得删除仍被引用的图片记录"
    );
}

#[tokio::test]
async fn profile_primary_write_waits_for_image_delete_and_rechecks_target() {
    let fixture = fixture().await;
    let intent = fixture.intent_input();
    let image = a4n_db::video::create_generated_image_record(&fixture.db, intent.image)
        .await
        .expect("并发引用测试图片必须创建成功");

    let tx = fixture.db.begin().await.expect("删除事务必须开启");
    novel_project::Entity::find_by_id(fixture.project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .lock_exclusive()
        .into_tuple::<i64>()
        .one(&tx)
        .await
        .expect("删除事务必须持有项目媒体引用锁")
        .expect("测试项目必须存在");
    generated_image::Entity::delete_by_id(image.id)
        .exec(&tx)
        .await
        .expect("删除事务必须先删除图片记录");

    let db = fixture.db.clone();
    let project_id = fixture.project_id;
    let mut write = tokio::spawn(async move {
        upsert_profile_primary_material(&db, project_id, "scene", 1, image.id).await
    });
    assert!(
        tokio::time::timeout(Duration::from_secs(5), &mut write)
            .await
            .is_err(),
        "主素材写入必须等待持有项目锁的删除事务"
    );

    tx.commit().await.expect("删除事务必须提交");
    let error = write
        .await
        .expect("主素材写入任务必须结束")
        .expect_err("删除提交后不得把已删除图片写成主素材");
    assert!(error.to_string().contains("不存在或不属于项目"));
}
