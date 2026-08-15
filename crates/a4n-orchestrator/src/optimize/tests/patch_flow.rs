use super::*;
use a4n_api_contract::optimize::OptimizationPatchOperation;

#[test]
fn apply_optimization_patches_rejects_stale_hash() {
    let content = "第一段正文。主角收到旧信，决定进入旧城。";
    let patch = OptimizationPatch {
        patch_id: "patch-001".to_string(),
        issue_id: Some("issue-plot-1".to_string()),
        operation: OptimizationPatchOperation::Replace,
        old_text: "主角收到旧信，决定进入旧城".to_string(),
        new_text: "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城".to_string(),
        reason: "补足动机".to_string(),
        risk: Some("low".to_string()),
    };

    let err = apply_optimization_patches_to_content(content, "stale", &[patch])
        .expect_err("stale hash must fail");

    assert!(err.to_string().contains("hash"));
}

#[test]
fn apply_optimization_patches_requires_unique_old_text() {
    let content = "旧城。旧城。";
    let patch = OptimizationPatch {
        patch_id: "patch-001".to_string(),
        issue_id: None,
        operation: OptimizationPatchOperation::Replace,
        old_text: "旧城".to_string(),
        new_text: "长安旧城".to_string(),
        reason: "明确地点".to_string(),
        risk: Some("low".to_string()),
    };
    let hash = optimization_content_hash(content);

    let err = apply_optimization_patches_to_content(content, &hash, &[patch])
        .expect_err("non-unique old_text must fail");

    assert!(err.to_string().contains("必须唯一匹配"));
}

#[test]
fn apply_optimization_patches_rejects_noop_replace() {
    let content = "甲。乙。";
    let hash = optimization_content_hash(content);
    let patch = OptimizationPatch {
        patch_id: "patch-noop".to_string(),
        issue_id: None,
        operation: OptimizationPatchOperation::Replace,
        old_text: "乙。".to_string(),
        new_text: "乙。".to_string(),
        reason: "无变化".to_string(),
        risk: Some("low".to_string()),
    };

    let err = apply_optimization_patches_to_content(content, &hash, &[patch])
        .expect_err("noop replace must fail");

    assert!(err.to_string().contains("不能与 old_text 相同"));
}

#[test]
fn apply_optimization_patches_supports_delete_and_insert_operations() {
    let content = "甲。乙。丙。";
    let hash = optimization_content_hash(content);
    let patches = vec![
        OptimizationPatch {
            patch_id: "patch-delete".to_string(),
            issue_id: None,
            operation: OptimizationPatchOperation::Delete,
            old_text: "乙。".to_string(),
            new_text: String::new(),
            reason: "删除冗余句".to_string(),
            risk: Some("low".to_string()),
        },
        OptimizationPatch {
            patch_id: "patch-before".to_string(),
            issue_id: None,
            operation: OptimizationPatchOperation::InsertBefore,
            old_text: "丙。".to_string(),
            new_text: "转折。".to_string(),
            reason: "补充转折".to_string(),
            risk: Some("low".to_string()),
        },
        OptimizationPatch {
            patch_id: "patch-after".to_string(),
            issue_id: None,
            operation: OptimizationPatchOperation::InsertAfter,
            old_text: "甲。".to_string(),
            new_text: "铺垫。".to_string(),
            reason: "补充铺垫".to_string(),
            risk: Some("low".to_string()),
        },
    ];

    let result = apply_optimization_patches_to_content(content, &hash, &patches)
        .expect("delete and insert patches should apply");

    assert_eq!(result, "甲。铺垫。转折。丙。");
}

#[test]
fn apply_optimization_patches_rejects_invalid_new_text_for_operation() {
    let content = "甲。乙。";
    let hash = optimization_content_hash(content);
    let delete_patch = OptimizationPatch {
        patch_id: "patch-delete".to_string(),
        issue_id: None,
        operation: OptimizationPatchOperation::Delete,
        old_text: "乙。".to_string(),
        new_text: "不应存在".to_string(),
        reason: "删除冗余句".to_string(),
        risk: Some("low".to_string()),
    };
    let insert_patch = OptimizationPatch {
        patch_id: "patch-insert".to_string(),
        issue_id: None,
        operation: OptimizationPatchOperation::InsertAfter,
        old_text: "甲。".to_string(),
        new_text: String::new(),
        reason: "补充铺垫".to_string(),
        risk: Some("low".to_string()),
    };

    let delete_err =
        apply_optimization_patches_to_content(content, &hash, std::slice::from_ref(&delete_patch))
            .expect_err("delete with new_text must fail");
    let insert_err = apply_optimization_patches_to_content(content, &hash, &[insert_patch])
        .expect_err("insert without new_text must fail");

    assert!(delete_err.to_string().contains("new_text 必须为空字符串"));
    assert!(insert_err.to_string().contains("new_text 不能为空"));
}
