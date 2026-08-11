use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use a4n_rag_core::vector_store::VectorStore;

use super::profile::RuntimeProfile;
use super::prompt_engine::{ensure_portable_prompts_dir, sync_contract_bound_prompts};
use super::rag::DisabledVectorStore;

fn unique_temp_dir(label: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("a4n_{label}_{}_{}", std::process::id(), now))
}

fn write_valid_prompt_dir(dir: &Path) {
    fs::create_dir_all(dir).expect("create prompt dir");
    fs::write(
        dir.join("existing.md"),
        "---\nname: existing\n---\nExisting",
    )
    .expect("write prompt");
    fs::write(
        dir.join("_registry.yaml"),
        r#"version: "1.0"
prompts:
  existing:
    path: "existing.md"
    title: "Existing"
    category: "writing"
    description: "Existing prompt"
    tags: ["test"]
    version: "1.0"
    used_by: []
    dependencies: []
categories:
  writing:
    label: "写作"
    order: 1
"#,
    )
    .expect("write registry");
}

#[test]
fn runtime_profile_parse_accepts_implemented_profiles() {
    assert_eq!(RuntimeProfile::parse("full").unwrap(), RuntimeProfile::Full);
    assert_eq!(
        RuntimeProfile::parse(" minimal ").unwrap(),
        RuntimeProfile::Minimal
    );
    assert_eq!(
        RuntimeProfile::parse("sidecar").unwrap(),
        RuntimeProfile::Sidecar
    );

    let err = RuntimeProfile::parse("unknown").expect_err("unknown profile should fail");
    assert!(err.to_string().contains("可选值: full, minimal, sidecar"));
}

#[test]
fn runtime_profile_default_matches_compiled_capabilities() {
    #[cfg(feature = "embedded-capabilities")]
    assert_eq!(RuntimeProfile::default_for_build(), RuntimeProfile::Full);

    #[cfg(not(feature = "embedded-capabilities"))]
    assert_eq!(RuntimeProfile::default_for_build(), RuntimeProfile::Sidecar);

    let error = RuntimeProfile::parse(" ").expect_err("blank profile must fail explicitly");
    assert!(error.to_string().contains("A4N_RUNTIME_PROFILE 不能为空"));
}

#[tokio::test]
async fn disabled_vector_store_keeps_reads_explicit_and_drop_idempotent() {
    let store = DisabledVectorStore::new(RuntimeProfile::Minimal);

    let err = store
        .metadata_by_ids("missing", &["missing".to_string()])
        .await
        .expect_err("disabled RAG must fail visibly");

    assert!(err.to_string().contains("RAG 能力未启用"));
    assert!(err.to_string().contains("runtime profile=minimal"));
    store
        .drop_collection("missing")
        .await
        .expect("disabled RAG has no collection to delete");
}

#[cfg(not(all(
    feature = "embedded-prompt-git",
    feature = "embedded-llm-provider",
    feature = "local-embedding",
    feature = "embedded-rag-index",
    feature = "embedded-media-providers"
)))]
#[test]
fn full_profile_requires_all_embedded_capabilities_before_startup() {
    let error = RuntimeProfile::Full
        .validate_build_capabilities()
        .expect_err("full profile must reject an incomplete embedded build");
    let message = error.to_string();

    assert!(message.contains("full runtime profile 要求完整 embedded capabilities"));
    assert!(message.contains("--features embedded-capabilities"));
}

#[test]
fn existing_user_prompts_sync_failure_is_not_silently_ignored() {
    let root = unique_temp_dir("prompt_sync_failure");
    let data_dir = root.join("data");
    let portable_dir = data_dir.join("prompts");
    let bundled_dir = root.join("prompts");
    write_valid_prompt_dir(&portable_dir);
    fs::create_dir_all(&bundled_dir).expect("create bundled dir");
    fs::write(bundled_dir.join("_registry.yaml"), "prompts: [").expect("write bad registry");

    let result = ensure_portable_prompts_dir(&data_dir);

    fs::remove_dir_all(&root).ok();
    let err = result.expect_err("sync failure should stop prompt engine startup");
    assert!(err.contains("同步随包新增 prompts 失败"));
    assert!(err.contains("解析提示词注册表失败"));
}

#[test]
fn contract_bound_sync_overwrites_stale_backend_prompts() {
    let root = unique_temp_dir("contract_prompt_sync");
    let source_dir = root.join("bundled");
    let dest_dir = root.join("data").join("prompts");

    let paths = [
        "novel/04_writing/writing.md",
        "novel/05_analysis/chapter_analysis_session.md",
        "novel/05_analysis/scene_profile_sync_session.md",
        "novel/07_protagonist/character_sync_session.md",
        "novel/09_galgame/galgame_pipeline_session.md",
    ];
    for rel_path in paths {
        let source_path = source_dir.join(rel_path);
        let dest_path = dest_dir.join(rel_path);
        fs::create_dir_all(source_path.parent().expect("source parent"))
            .expect("create source parent");
        fs::create_dir_all(dest_path.parent().expect("dest parent")).expect("create dest parent");
        fs::write(&source_path, format!("current:{rel_path}"))
            .expect("write bundled contract prompt");
        fs::write(&dest_path, format!("stale:{rel_path}")).expect("write stale contract prompt");
    }

    sync_contract_bound_prompts(&source_dir, &dest_dir).expect("sync contract prompts");

    for rel_path in paths {
        assert_eq!(
            fs::read_to_string(dest_dir.join(rel_path)).expect("read synced contract prompt"),
            format!("current:{rel_path}")
        );
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn portable_prompt_sync_replaces_stale_contract_registry_dependencies() {
    let root = unique_temp_dir("contract_registry_sync");
    let data_dir = root.join("data");
    let portable_dir = data_dir.join("prompts");
    let bundled_dir = root.join("prompts");
    fs::create_dir_all(&portable_dir).expect("create portable prompt dir");
    fs::create_dir_all(&bundled_dir).expect("create bundled prompt dir");

    let prompt_paths = [
        "novel/05_analysis/chapter_screenplay_skeleton.md",
        "novel/05_analysis/chapter_screenplay_layout_unit.md",
        "novel/05_analysis/chapter_screenplay_unit_adaptation.md",
        "novel/05_analysis/chapter_shooting_design_unit.md",
        "novel/08_video/chapter_video_segment_plan.md",
    ];
    for path in prompt_paths {
        let prompt_path = bundled_dir.join(path);
        fs::create_dir_all(prompt_path.parent().expect("prompt parent"))
            .expect("create bundled prompt parent");
        fs::write(prompt_path, "---\nname: test\n---\ntest").expect("write bundled prompt");
    }

    fs::write(
        bundled_dir.join("_registry.yaml"),
        r#"version: "1.0"
prompts:
  chapter_screenplay_skeleton:
    path: "novel/05_analysis/chapter_screenplay_skeleton.md"
    title: "章节叙事骨架"
    category: "analysis"
    description: "布局"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: []
  chapter_screenplay_layout_unit:
    path: "novel/05_analysis/chapter_screenplay_layout_unit.md"
    title: "叙事单元布局细化"
    category: "analysis"
    description: "布局细化"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_screenplay_skeleton"]
  chapter_screenplay_unit_adaptation:
    path: "novel/05_analysis/chapter_screenplay_unit_adaptation.md"
    title: "叙事单元剧本改编"
    category: "analysis"
    description: "改编"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_screenplay_layout_unit"]
  chapter_shooting_design_unit:
    path: "novel/05_analysis/chapter_shooting_design_unit.md"
    title: "拍摄设计单元"
    category: "analysis"
    description: "镜头设计"
    tags: ["shot"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_screenplay_unit_adaptation"]
  chapter_video_segment_plan:
    path: "novel/08_video/chapter_video_segment_plan.md"
    title: "章节视频片段规划"
    category: "analysis"
    description: "片段规划"
    tags: ["video"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_shooting_design_unit"]
categories:
  analysis:
    label: "分析"
    order: 1
"#,
    )
    .expect("write bundled registry");

    fs::write(
        portable_dir.join("_registry.yaml"),
        r#"version: "1.0"
prompts:
  chapter_screenplay_skeleton:
    path: "novel/05_analysis/chapter_screenplay_skeleton.md"
    title: "章节叙事骨架"
    category: "analysis"
    description: "布局"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: []
  chapter_screenplay_layout_unit:
    path: "novel/05_analysis/chapter_screenplay_layout_unit.md"
    title: "叙事单元布局细化"
    category: "analysis"
    description: "布局细化"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_screenplay_skeleton"]
  chapter_screenplay_unit_adaptation:
    path: "novel/05_analysis/chapter_screenplay_unit_adaptation.md"
    title: "叙事单元剧本改编"
    category: "analysis"
    description: "改编"
    tags: ["screenplay"]
    version: "1.0"
    used_by: []
    dependencies: ["chapter_screenplay_layout_unit"]
  chapter_shooting_design_unit:
    path: "novel/05_analysis/chapter_shooting_design_unit.md"
    title: "拍摄设计单元"
    category: "analysis"
    description: "旧镜头设计"
    tags: ["shot"]
    version: "0.9"
    used_by: []
    dependencies: ["obsolete_prompt_contract"]
  chapter_video_segment_plan:
    path: "novel/08_video/chapter_video_segment_plan.md"
    title: "章节视频片段规划"
    category: "analysis"
    description: "旧片段规划"
    tags: ["video"]
    version: "0.9"
    used_by: []
    dependencies: ["obsolete_prompt_contract"]
categories:
  analysis:
    label: "分析"
    order: 1
"#,
    )
    .expect("write portable registry");

    let synced_dir = ensure_portable_prompts_dir(&data_dir).expect("sync portable prompts");
    let registry = a4n_prompt::registry::PromptRegistry::load_from_dir(&synced_dir)
        .expect("load synced registry");
    assert!(registry.validate(&synced_dir).is_empty());
    assert_eq!(
        registry
            .get("chapter_shooting_design_unit")
            .expect("design prompt")
            .dependencies,
        vec!["chapter_screenplay_unit_adaptation"]
    );
    assert_eq!(
        registry
            .get("chapter_video_segment_plan")
            .expect("segment plan prompt")
            .dependencies,
        vec!["chapter_shooting_design_unit"]
    );

    fs::remove_dir_all(&root).ok();
}
