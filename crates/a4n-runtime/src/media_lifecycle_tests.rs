use std::{path::Path, time::Duration};

use a4n_config::app_config::{AppConfig, SecurityConfig};
use a4n_db_models::models::project::project_member;
use a4n_error::AppError;
use a4n_orchestrator::media::{
    ImageImportApplicationPort, ImagePublicationRecoveryPort, PreparedImportedImageUpload,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use sha2::{Digest, Sha256};

use crate::{
    AppState, host_api::auth_records::authorize_media_file_for_user, state::RuntimeProfile,
};

async fn state() -> (
    tempfile::TempDir,
    AppState,
    sea_orm::DatabaseConnection,
    i64,
    i64,
) {
    let (db, database_url) = a4n_migration::test_support::connect_test_database_with_url().await;
    // A real bundled prompt registry is available beside this isolated data directory.
    let directory = tempfile::Builder::new()
        .prefix(".a4n-media-test-")
        .tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .unwrap();
    let data_dir = directory.path().to_path_buf();
    let config = AppConfig {
        database_url,
        cache_dir: data_dir.join("cache"),
        temp_dir: data_dir.join("tmp"),
        log_dir: data_dir.join("logs"),
        data_dir,
        security: SecurityConfig {
            jwt_secret: Some("isolated-media-test-secret".to_string()),
        },
        ..Default::default()
    };
    let state = AppState::new_with_profile(config, RuntimeProfile::Minimal)
        .await
        .unwrap();
    let owner = a4n_db::auth::create_user(
        &db,
        "media-owner",
        "owner@example.invalid",
        "test-password",
        "user",
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: "media test".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    (directory, state, db, owner.id, project.id)
}

fn prepared_image_upload(project_id: i64) -> PreparedImportedImageUpload {
    let decoded = a4n_media::decode_uploaded_image_data(
        "test.png",
        "image/png",
        TEST_IMAGE_BASE64,
        50 * 1024 * 1024,
    )
    .unwrap();
    PreparedImportedImageUpload {
        project_id,
        image_type: "imported".to_string(),
        prompt: "test".to_string(),
        model: "external_import".to_string(),
        chapter_id: None,
        panel_id: None,
        metadata_json: None,
        decoded,
    }
}

const TEST_IMAGE_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

#[tokio::test]
async fn media_access_checks_membership_and_the_resolved_project_or_theme() {
    let (_directory, state, db, owner_id, project_id) = state().await;
    let other = a4n_db::auth::create_user(
        &db,
        "media-other",
        "other@example.invalid",
        "test-password",
        "user",
    )
    .await
    .unwrap();
    let path = format!("projects/{project_id}/images/private.png");
    let absolute = state.data_dir().join(&path);
    std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
    std::fs::write(&absolute, b"private media").unwrap();
    assert_eq!(
        authorize_media_file_for_user(&state, owner_id, Path::new(&path))
            .await
            .unwrap(),
        absolute.canonicalize().unwrap()
    );
    assert!(matches!(
        authorize_media_file_for_user(&state, other.id, Path::new(&path)).await,
        Err(AppError::Forbidden(_))
    ));

    let now = chrono::Utc::now().naive_utc();
    let member = project_member::ActiveModel {
        project_id: Set(project_id),
        user_id: Set(other.id),
        role: Set("viewer".to_string()),
        invited_by_user_id: Set(owner_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert!(
        authorize_media_file_for_user(&state, other.id, Path::new(&path))
            .await
            .is_ok()
    );
    project_member::Entity::delete_by_id(member.id)
        .exec(&db)
        .await
        .unwrap();
    assert!(matches!(
        authorize_media_file_for_user(&state, other.id, Path::new(&path)).await,
        Err(AppError::Forbidden(_))
    ));

    for (theme_owner, allowed) in [(Some(owner_id), false), (None, true)] {
        let theme = a4n_db::config::create_theme(
            &db,
            theme_owner,
            "media theme",
            a4n_domain::ThemeMode::Light,
            a4n_domain::ThemeDocument::default(),
        )
        .await
        .unwrap();
        let theme_path = format!("themes/{}/background.png", theme.id);
        let absolute = state.data_dir().join(&theme_path);
        std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        std::fs::write(absolute, b"theme").unwrap();
        assert_eq!(
            authorize_media_file_for_user(&state, other.id, Path::new(&theme_path))
                .await
                .is_ok(),
            allowed
        );
    }

    #[cfg(unix)]
    {
        let own_project = a4n_db::project::create_project_record(
            &db,
            a4n_db::project::CreateProjectRecordInput {
                user_id: other.id,
                title: "other".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let link = format!("projects/{}/images/link.png", own_project.id);
        std::fs::create_dir_all(state.data_dir().join(&link).parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&absolute, state.data_dir().join(&link)).unwrap();
        assert!(matches!(
            authorize_media_file_for_user(&state, other.id, Path::new(&link)).await,
            Err(AppError::Forbidden(_))
        ));
        let traversal = format!(
            "projects/{}/../{project_id}/images/private.png",
            own_project.id
        );
        assert!(matches!(
            authorize_media_file_for_user(&state, other.id, Path::new(&traversal)).await,
            Err(AppError::Forbidden(_))
        ));
    }
}

#[tokio::test]
async fn media_recovery_waits_for_publication_and_recovers_a_cancelled_writer() {
    let (_directory, state, _db, user_id, project_id) = state().await;
    let prepared = prepared_image_upload(project_id);
    let bytes = prepared.decoded.bytes.clone();
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let final_path = format!("projects/{project_id}/images/live.png");
    let (publication, guard) = state
        .image_import_application()
        .create_imported_publication_intent(
            user_id,
            &prepared,
            "live-publication".to_string(),
            format!("projects/{project_id}/images/live.part"),
            final_path.clone(),
            sha256.clone(),
        )
        .await
        .unwrap();
    let recovery_state = state.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    let mut recovery = tokio::spawn(async move {
        started.send(()).unwrap();
        crate::cleanup_outbox::recover_cleanup_outbox(&recovery_state).await;
    });
    waiting.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut recovery)
            .await
            .is_err()
    );
    let path = state.data_dir().join(final_path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    state
        .image_import_application()
        .publish_imported_image(
            user_id,
            project_id,
            &publication.publication_id,
            bytes.len() as i64,
            sha256,
        )
        .await
        .unwrap();
    drop(guard);
    tokio::time::timeout(Duration::from_secs(5), recovery)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        state
            .image_publication_recovery_application()
            .image_publication_by_id(&publication.publication_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        "published"
    );
    assert!(path.is_file());

    let abandoned = prepared_image_upload(project_id);
    let writer_state = state.clone();
    let (started, writing) = tokio::sync::oneshot::channel();
    let writer = tokio::spawn(async move {
        let (_publication, _guard) = writer_state
            .image_import_application()
            .create_imported_publication_intent(
                user_id,
                &abandoned,
                "abandoned-publication".to_string(),
                format!("projects/{project_id}/images/abandoned.part"),
                format!("projects/{project_id}/images/abandoned.png"),
                format!("{:x}", Sha256::digest(&abandoned.decoded.bytes)),
            )
            .await
            .unwrap();
        started.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    writing.await.unwrap();
    writer.abort();
    assert!(writer.await.unwrap_err().is_cancelled());
    tokio::time::timeout(
        Duration::from_secs(5),
        crate::cleanup_outbox::recover_cleanup_outbox(&state),
    )
    .await
    .unwrap();
    assert_eq!(
        state
            .image_publication_recovery_application()
            .image_publication_by_id("abandoned-publication")
            .await
            .unwrap()
            .unwrap()
            .status,
        "reclaimed"
    );
}

#[tokio::test]
async fn media_recovery_retains_published_record_when_final_file_facts_drift() {
    let (_directory, state, _db, user_id, project_id) = state().await;
    let prepared = prepared_image_upload(project_id);
    let bytes = prepared.decoded.bytes.clone();
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let final_path = format!("projects/{project_id}/images/drifted.png");
    let (publication, guard) = state
        .image_import_application()
        .create_imported_publication_intent(
            user_id,
            &prepared,
            "drifted-publication".to_string(),
            format!("projects/{project_id}/images/drifted.part"),
            final_path.clone(),
            sha256.clone(),
        )
        .await
        .unwrap();
    let path = state.data_dir().join(&final_path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let image = state
        .image_import_application()
        .publish_imported_image(
            user_id,
            project_id,
            &publication.publication_id,
            bytes.len() as i64,
            sha256,
        )
        .await
        .unwrap();
    drop(guard);

    std::fs::write(&path, b"drift").unwrap();
    crate::cleanup_outbox::recover_cleanup_outbox(&state).await;

    let recovered = state
        .image_publication_recovery_application()
        .image_publication_by_id(&publication.publication_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.status, "published");
    assert_eq!(recovered.image_id, Some(image.id));
    assert_eq!(
        state
            .image_import_application()
            .imported_image_summary(image.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        image.id
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"drift");
}
