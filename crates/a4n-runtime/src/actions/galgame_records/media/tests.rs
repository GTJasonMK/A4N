use super::*;
use a4n_core::models::media::{media_asset, media_binding};
use a4n_orchestrator::galgame::GalgameTargetContext;
use a4n_orchestrator::galgame::{GALGAME_ENTITY_SCENE, GalgameMediaBindingSummary};
use a4n_orchestrator::media::{
    MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_SFX,
    MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE, MEDIA_ORIGIN_IMPORTED, MEDIA_STATUS_COMPLETED,
};
use chrono::NaiveDateTime;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn imported_galgame_audio_persistence_keeps_output_and_reports_actual_size() {
    let data_dir = unique_temp_dir("a4n_galgame_audio_persist");

    let (path, relative_path, file_size) =
        persist_imported_galgame_audio(&data_dir, 7, "mp3", b"audio-bytes")
            .expect("persist imported Galgame audio");

    assert!(path.is_file());
    assert!(relative_path.starts_with("projects/7/audio/galgame_audio_"));
    assert!(relative_path.ends_with(".mp3"));
    assert_eq!(file_size, 11);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn galgame_media_asset_source_context_accepts_same_snapshot_without_chapter() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_chapter_id = None;
    asset.source_snapshot_chapter_id = None;

    validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect("same source snapshot can be bound without a chapter-specific asset context");
}

#[test]
fn galgame_media_asset_source_context_rejects_missing_snapshot() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_snapshot_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("Galgame media bindings must keep source snapshot context");

    assert!(err.contains("缺少 source_snapshot_id"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_mismatch() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_snapshot_id = Some(21);

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("Galgame media source snapshot must match the target script snapshot");

    assert!(err.contains("源快照与目标不一致"));
    assert!(err.contains("asset_source_snapshot_id=21"));
    assert!(err.contains("target_source_snapshot_id=20"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_chapter_mismatch() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_snapshot_chapter_id = Some(41);

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("chapter-scoped Galgame media must match the target scene chapter");

    assert!(err.contains("源快照章节与目标不一致"));
    assert!(err.contains("asset_source_snapshot_chapter_id=41"));
    assert!(err.contains("target_source_snapshot_chapter_id=40"));
}

#[test]
fn galgame_media_asset_source_context_rejects_partial_chapter_context() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_snapshot_chapter_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("source_chapter_id without source_snapshot_chapter_id must fail");

    assert!(err.contains("源章节上下文不完整"));
    assert!(err.contains("source_snapshot_chapter_id"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_chapter_without_source_chapter() {
    let mut asset = media_summary(
        1,
        11,
        GALGAME_ENTITY_SCENE,
        100,
        MEDIA_BINDING_ROLE_BACKGROUND,
        "projects/10/images/bg.png",
    )
    .asset;
    asset.source_chapter_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("source_snapshot_chapter_id without source_chapter_id must fail");

    assert!(err.contains("源章节上下文不完整"));
    assert!(err.contains("source_chapter_id"));
}

fn media_summary(
    binding_id: i64,
    asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    file_path: &str,
) -> GalgameMediaBindingSummary {
    let media_kind = if role == MEDIA_BINDING_ROLE_BGM || role == MEDIA_BINDING_ROLE_SFX {
        MEDIA_KIND_AUDIO
    } else {
        MEDIA_KIND_IMAGE
    };
    media_summary_with_kind(
        binding_id,
        asset_id,
        target_kind,
        target_id,
        role,
        media_kind,
        file_path,
    )
}

fn media_summary_with_kind(
    binding_id: i64,
    asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    media_kind: &str,
    file_path: &str,
) -> GalgameMediaBindingSummary {
    let now = fixed_time();
    GalgameMediaBindingSummary {
        binding: media_binding::Model {
            id: binding_id,
            media_asset_id: asset_id,
            target_kind: target_kind.to_string(),
            target_id,
            role: role.to_string(),
            created_at: now,
        },
        asset: media_asset::Model {
            id: asset_id,
            project_id: 10,
            root_project_id: 1,
            source_snapshot_id: Some(20),
            source_chapter_id: Some(30),
            source_snapshot_chapter_id: Some(40),
            media_kind: media_kind.to_string(),
            origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            file_path: Some(file_path.to_string()),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_secs: None,
            fps: None,
            file_size: None,
            checksum: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            created_at: now,
            updated_at: now,
        },
    }
}

fn background_target_context() -> GalgameTargetContext {
    GalgameTargetContext {
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: 20,
        source_chapter_id: 30,
        source_snapshot_chapter_id: 40,
        scene_id: 100,
        line_id: None,
    }
}

fn fixed_time() -> NaiveDateTime {
    chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("valid timestamp")
        .naive_utc()
}

fn validate_galgame_media_asset_source_context(
    asset: &media_asset::Model,
    context: &GalgameTargetContext,
) -> Result<(), String> {
    a4n_orchestrator::galgame::validate_galgame_media_asset_source_context(asset, context)
        .map_err(|error| error.to_string())
}
