use std::collections::{HashMap, HashSet};

use super::super::TeraPromptEngine;
use super::support::make_entry;

#[test]
fn test_validate_variables() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{{ foo }} {{ bar }} {{ baz }}")])
            .unwrap();

    let mut provided = HashSet::new();
    provided.insert("foo".to_string());
    provided.insert("bar".to_string());

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["baz"]);
}

#[test]
fn test_validate_variables_with_filter_expression() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{{ name | upper }}")]).unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["name"]);
}

#[test]
fn validate_variables_reports_if_condition_variables() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{% if user %}你好{% endif %}")])
            .unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["user"]);
}

#[test]
fn defined_guard_makes_only_its_branch_variable_optional() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry(
        "test",
        "{% if optimization.direction is defined %}{% if optimization.direction %}{{ optimization.direction }}{% endif %}{% endif %} {{ required.value }}",
    )])
    .unwrap();
    let provided = HashSet::new();

    assert_eq!(
        engine.validate_variables("test", &provided).unwrap(),
        vec!["required"]
    );
    assert_eq!(
        crate::variables::template_variable_access_paths(
            engine.get("test").unwrap().content.as_str()
        )
        .unwrap(),
        vec![
            vec!["optimization".to_string(), "direction".to_string()],
            vec!["required".to_string(), "value".to_string()]
        ]
    );
}

#[test]
fn validate_variables_uses_for_loop_container_not_loop_local() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry(
        "test",
        "{% for item in items %}{{ loop.index }} {{ item.name }}{% endfor %}",
    )])
    .unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["items"]);

    let mut provided = HashSet::new();
    provided.insert("items".to_string());
    let missing = engine.validate_variables("test", &provided).unwrap();
    assert!(missing.is_empty());
}

#[test]
fn template_variable_access_paths_include_nested_external_fields() {
    let paths = crate::variables::template_variable_access_paths(
        "{% if scene.location %}{{ scene.title }}{% endif %} {{ line.text }}",
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![
            vec!["line".to_string(), "text".to_string()],
            vec!["scene".to_string(), "location".to_string()],
            vec!["scene".to_string(), "title".to_string()],
        ]
    );
}

/// 集成测试：从真实仓库 prompts/ 目录加载当前提示词集合，
/// 验证递归扫描、frontmatter 解析、@include 展开、registry 严格注入全链路。
#[test]
fn test_load_real_prompts_dir() {
    // 仓库根目录的 prompts/，相对于 crates/prompt/a4n-prompt
    let prompts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts");
    assert!(prompts_dir.is_dir(), "prompts 目录不存在：{prompts_dir:?}");

    let engine = TeraPromptEngine::from_directory(&prompts_dir)
        .expect("from_directory 应能加载真实 prompts/");
    let all = engine.list_all();

    // 关键 prompt 必须存在（service 直接调用的）
    let names: std::collections::HashSet<String> = all.iter().map(|e| e.name.clone()).collect();
    for required in [
        "inspiration",
        "blueprint",
        "outline",
        "part_outline_single",
        "writing",
        "chapter_fact_extraction",
        "chapter_summary_synthesis",
        "chapter_scene_generation",
        "inspiration_project_context_guard",
        "chapter_screenplay_layout_unit",
        "chapter_screenplay_unit_adaptation",
        "manga_preprocessing_facts",
        "manga_preprocessing_speech",
        "manga_preprocessing_visuals",
        "manga_preprocessing_synthesis",
        "manga_page_slots",
        "manga_page_adaptation_plan",
        "manga_page_storyboard",
        "manga_page_drawing_prompt",
        "character_portrait_generation",
        "character_discovery",
        "character_conflict_check",
        "blueprint_optimization",
    ] {
        assert!(
            names.contains(required),
            "缺少必需的 prompt: {required}（实际：{:?}）",
            names
        );
    }

    // 至少一个 prompt 应有 used_by（registry 元数据）
    let with_used_by = all.iter().filter(|e| !e.used_by.is_empty()).count();
    assert!(
        with_used_by >= 6,
        "应有 ≥6 个 prompt 通过 registry 注入 used_by，实际：{with_used_by}"
    );

    // _partials/ 内的文件不应作为顶层 entry 暴露
    assert!(
        !names.contains("json_only_rule") && !names.contains("_json_only_rule"),
        "_partials 内的文件不应作为顶层 prompt 暴露"
    );

    let portrait_prompt = engine.get("character_portrait_generation").unwrap().content;
    assert!(portrait_prompt.contains("中文 prompt"));
    assert!(!portrait_prompt.contains("英文 prompt"));

    let manga_storyboard_prompt = engine.get("manga_page_storyboard").unwrap().content;
    assert!(manga_storyboard_prompt.contains("\"layer_index\": 1"));
    assert!(manga_storyboard_prompt.contains("narration"));
    assert!(manga_storyboard_prompt.contains("text_layers[].speaker"));
    assert!(manga_storyboard_prompt.contains("JSON `null`"));
    assert!(manga_storyboard_prompt.contains("upper_right"));
    assert!(manga_storyboard_prompt.contains("8 个规范值"));

    let mut context_guard_vars = HashMap::new();
    context_guard_vars.insert(
        "project_context_json".to_string(),
        serde_json::json!("{\"title\":\"测试项目\"}"),
    );
    let context_guard = engine
        .render(
            "inspiration_project_context_guard",
            &context_guard_vars,
            None,
        )
        .unwrap();
    assert!(context_guard.contains("测试项目"));
    assert!(context_guard.contains("优先尊重"));

    let mut scene_vars = HashMap::new();
    scene_vars.insert("chapter_number".to_string(), serde_json::json!(3));
    scene_vars.insert("chapter_title".to_string(), serde_json::json!("雨夜"));
    scene_vars.insert("scene_number".to_string(), serde_json::json!(2));
    let scene_prompt = engine
        .render("chapter_scene_generation", &scene_vars, None)
        .unwrap();
    assert!(scene_prompt.contains("第 3 章《雨夜》"));
    assert!(scene_prompt.contains("第 2 个场景"));

    let conflict_prompt = engine.get("character_conflict_check").unwrap().content;
    assert!(conflict_prompt.contains("故事一致性检查专家"));
    assert!(conflict_prompt.contains("source_id"));

    for name in ["content_optimization", "content_optimization_chat"] {
        let content = engine.get(name).unwrap().content;
        if name == "content_optimization" {
            assert!(content.contains("参数结构"));
        } else {
            assert!(content.contains("schema 名称"));
        }
        assert!(!content.contains("符合对应 schema"));
    }

    for name in ["chapter_video_frame_prompts", "chapter_video_prompt"] {
        let content = engine.get(name).unwrap().content;
        assert!(content.contains("中文"), "{name} 应明确要求输出中文提示词");
        for forbidden in [
            "low quality",
            "no reference images",
            "no revision needed",
            "英文或中英混合",
            "camera 6 o'clock",
            "cinematic lighting",
            "sad、angry、tense、emotional",
        ] {
            assert!(
                !content.contains(forbidden),
                "{name} 仍包含旧英文提示词片段: {forbidden}"
            );
        }
    }
}
