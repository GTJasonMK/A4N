#[test]
fn parse_optimization_issues_accepts_canonical_array_only() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("canonical issues should parse");

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, "issue-001");
    assert_eq!(issues[0].tool, "edit_article");
    assert_eq!(issues[0].description, "动机不足");
    assert_eq!(issues[0].severity, "major");

    let wrapped = parse_optimization_issues(&json!({
        "issues": [{"id": "issue-001", "tool": "edit_article", "arguments": {}}]
    }))
    .expect_err("wrapped issues are not the persisted contract");
    assert!(wrapped.to_string().contains("issues 必须是数组"));

    let missing_arguments = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article"
    }]))
    .expect_err("arguments are required");
    assert!(missing_arguments.to_string().contains("缺少 arguments"));

    let missing_severity = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足"
    }]))
    .expect_err("severity is required by the persisted issue contract");
    assert!(missing_severity.to_string().contains("severity"));

    let unknown_field = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足",
        "severity": "major",
        "legacy": true
    }]))
    .expect_err("unknown issue fields must fail");
    assert!(unknown_field.to_string().contains("未知字段 legacy"));
}

#[test]
fn materialize_optimization_issue_evidence_uses_source_span_id() {
    let source_context = json!({"current_chapter": {
        "chapter_number": 1,
        "content": "前文出现。决定进入旧城。",
        "source_spans": [
            {"id": "ch1:s1:f1", "text": "前文出现", "paragraph_index": 1},
            {"id": "ch1:s2:f1", "text": "决定进入旧城", "paragraph_index": 1}
        ]
    }});
    let markers = crate::optimize::runtime_tools::current_chapter_source_markers(
        &source_context["current_chapter"],
    )
    .unwrap();
    let evidence_marker = markers.marker_for("ch1:s2:f1").unwrap();
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": evidence_marker,
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("canonical model issues should parse before materialization");
    let materialized = materialize_optimization_issue_evidence(
        issues,
        &source_context,
        &json!({
            "current_chapter": {"source_spans": [
                {"id": "ch1:s2:f1", "text": "决定进入旧城"}
            ]}
        }),
    )
    .expect("source span id should be materialized");

    assert_eq!(materialized[0].arguments["evidence"], "决定进入旧城");
    assert!(materialized[0].arguments.get("evidence_span_id").is_none());
}

#[test]
fn materialize_optimization_issue_evidence_rejects_legacy_evidence() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence": "模型自由抄写的证据",
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should not validate model boundary evidence");
    let err = materialize_optimization_issue_evidence(
        issues,
        &json!({"current_chapter": {
            "chapter_number": 1,
            "content": "决定进入旧城。"
        }}),
        &json!({
            "current_chapter": {"source_spans": []}
        }),
    )
    .expect_err("legacy evidence must be rejected at model boundary");

    assert!(err.to_string().contains("arguments.evidence 已废弃"));
}

#[test]
fn materialize_optimization_issue_evidence_rejects_unknown_argument_fields() {
    let source_context = json!({"current_chapter": {
        "chapter_number": 1,
        "content": "决定进入旧城。"
    }});
    let markers = crate::optimize::runtime_tools::current_chapter_source_markers(
        &source_context["current_chapter"],
    )
    .unwrap();
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": markers.marker_for("ch1:s1:f1").unwrap(),
            "goal": "补足行动动机",
            "old_text": "模型不应在 issue 阶段输出补丁字段"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should parse issue before materialization");
    let err = materialize_optimization_issue_evidence(
        issues,
        &source_context,
        &json!({"current_chapter": {"source_spans": []}}),
    )
    .expect_err("issue arguments must reject unknown model-owned fields");

    assert!(err.to_string().contains("arguments 包含未知字段 old_text"));
}

#[test]
fn materialize_optimization_issue_evidence_rejects_unknown_span_id() {
    let source_context = json!({"current_chapter": {
        "chapter_number": 1,
        "content": "已有文本。未知引用。",
        "source_spans": [
            {"id": "ch1:s1:f1", "text": "已有文本", "paragraph_index": 1},
            {"id": "ch1:s2:f1", "text": "未知引用", "paragraph_index": 1}
        ]
    }});
    let markers = crate::optimize::runtime_tools::current_chapter_source_markers(
        &source_context["current_chapter"],
    )
    .unwrap();
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": markers.marker_for("ch1:s2:f1").unwrap(),
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should accept issue before materialization");
    let err = materialize_optimization_issue_evidence(
        issues,
        &source_context,
        &json!({"current_chapter": {"source_spans": [
            {"id": "ch1:s1:f1", "text": "已有文本"}
        ]}}),
    )
    .expect_err("unknown span id must fail visibly");

    assert!(err.to_string().contains("evidence_span_id 不在"));
}
