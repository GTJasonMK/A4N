use super::package::build_webgal_package;
use a4n_core::models::{
    character::character_portrait,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    media::{media_asset, media_binding},
};
use a4n_orchestrator::galgame::{
    GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, GalgameMediaBindingSummary, GalgameSceneDetail,
    GalgameScriptDetail, GalgameSpeakerProfile, GalgameTextAnchorSummary, SCENE_ENTRY_BRANCH,
};
use a4n_orchestrator::media::{
    MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_CG,
    MEDIA_BINDING_ROLE_SFX, MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE, MEDIA_ORIGIN_IMPORTED,
    MEDIA_STATUS_COMPLETED,
};
use chrono::NaiveDateTime;
use serde_json::json;
use std::io::{Cursor, Read};
use std::path::Path;
use tempfile::tempdir;
use zip::ZipArchive;

#[test]
fn webgal_package_writes_scene_scripts_and_bound_media() {
    let temp = tempdir().expect("temp dir");
    write_fixture_file(temp.path(), "projects/10/images/bg.png", b"bg-bytes");
    write_fixture_file(temp.path(), "projects/10/images/cg.jpg", b"cg-bytes");
    write_fixture_file(temp.path(), "projects/10/audio/bgm.mp3", b"bgm-bytes");
    write_fixture_file(temp.path(), "projects/10/audio/sfx.ogg", b"sfx-bytes");
    let mut detail = webgal_fixture_detail(vec![
        media_summary(
            1,
            11,
            GALGAME_ENTITY_SCENE,
            100,
            MEDIA_BINDING_ROLE_BACKGROUND,
            "projects/10/images/bg.png",
        ),
        media_summary(
            2,
            12,
            GALGAME_ENTITY_LINE,
            1000,
            MEDIA_BINDING_ROLE_CG,
            "projects/10/images/cg.jpg",
        ),
        media_summary(
            3,
            13,
            GALGAME_ENTITY_LINE,
            1000,
            MEDIA_BINDING_ROLE_BGM,
            "projects/10/audio/bgm.mp3",
        ),
        media_summary(
            4,
            14,
            GALGAME_ENTITY_LINE,
            1000,
            MEDIA_BINDING_ROLE_SFX,
            "projects/10/audio/sfx.ogg",
        ),
    ]);
    detail.scenes[0].lines[0].presentation_json = json!({"transition": "cut"});

    let bytes = build_webgal_package(temp.path(), &detail, &[]).expect("webgal package");
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    assert_zip_text_contains(
        &mut zip,
        "game/config.txt",
        &["Game_name:测试 Galgame;", "Game_key:a4n_galgame_1;"],
    );
    assert_zip_text_contains(
        &mut zip,
        "game/scene/start.txt",
        &["changeScene:scene_001_100.txt;"],
    );
    assert_zip_text_contains(
        &mut zip,
        "game/scene/scene_001_100.txt",
        &[
            "changeBg:background_100_11.png -next;",
            "bgm:bgm_1000_13.mp3;",
            "playEffect:sfx_1000_14.ogg -next;",
            "changeBg:cg_1000_12.jpg -duration=0 -enterDuration=0 -exitDuration=0 -next;",
            "unlockCg:cg_1000_12.jpg -name=第一场 #1 CG;",
            "林晓雨:你好；世界 下一行;",
            "雨停了;",
            "choose:继续 回想:scene_002_200.txt|留在原地:scene_002_200.txt;",
            "changeScene:scene_002_200.txt;",
        ],
    );
    assert_zip_text_contains(&mut zip, "game/scene/scene_002_200.txt", &["第二场开始;"]);
    assert_zip_bytes(
        &mut zip,
        "game/background/background_100_11.png",
        b"bg-bytes",
    );
    assert_zip_bytes(&mut zip, "game/background/cg_1000_12.jpg", b"cg-bytes");
    assert_zip_bytes(&mut zip, "game/bgm/bgm_1000_13.mp3", b"bgm-bytes");
    assert_zip_bytes(&mut zip, "game/vocal/sfx_1000_14.ogg", b"sfx-bytes");
}

#[test]
fn webgal_package_writes_matching_character_figures() {
    let temp = tempdir().expect("temp dir");
    write_fixture_file(
        temp.path(),
        "projects/1/portraits/smile.png",
        b"figure-bytes",
    );
    let mut detail = webgal_fixture_detail(Vec::new());
    detail.scenes[0].lines[0].expression = Some("smile".to_string());
    detail.scenes[0].lines[0].pose = Some("standing".to_string());
    detail.scenes[0].lines[0].position = Some("left".to_string());
    let portraits = vec![portrait_model(
        50,
        1,
        Some("smile"),
        Some("standing"),
        false,
        "projects/1/portraits/smile.png",
    )];

    let bytes = build_webgal_package(temp.path(), &detail, &portraits).expect("webgal package");
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    assert_zip_text_contains(
        &mut zip,
        "game/scene/scene_001_100.txt",
        &[
            "changeFigure:figure_1_50.png -left -next;",
            "林晓雨:你好；世界 下一行;",
        ],
    );
    assert_zip_bytes(&mut zip, "game/figure/figure_1_50.png", b"figure-bytes");
}
