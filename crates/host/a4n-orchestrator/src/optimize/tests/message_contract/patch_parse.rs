#[test]
fn parse_content_optimization_patch_response_accepts_empty_patch_list() {
    let proposal = super::super::patch::parse_content_optimization_patch_response(
        1,
        2,
        3,
        "hash-001",
        "正文内容",
        r#"{"summary":"没有找到可安全唯一定位的局部修改。","patches":[]}"#,
    )
    .expect("empty patch proposal should be a valid no-op proposal");

    assert_eq!(proposal.summary, "没有找到可安全唯一定位的局部修改。");
    assert!(proposal.patches.is_empty());
}

#[test]
fn parse_content_optimization_patch_response_rejects_unknown_patch_field() {
    let err = super::super::patch::parse_content_optimization_patch_response(
        1,
        2,
        3,
        "hash-001",
        "主角突然转向。",
        r#"{"summary":"补足动机","patches":[{"patch_id":"patch-001","issue_id":"issue-001","operation":"replace","old_text":"主角突然转向。","new_text":"主角想到旧信中的线索，突然转向。","reason":"补足动机","risk":"low","legacy":true}]}"#,
    )
    .expect_err("unknown patch fields must fail");

    assert!(err.to_string().contains("unknown field `legacy`"));
}
