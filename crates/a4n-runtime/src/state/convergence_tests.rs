use std::path::{Path, PathBuf};

/// 禁止再把可迁移业务 service 挂回 AppState；共享运行时能力统一归 OrchestratorContext。
#[test]
fn appstate_service_bypass_fields_removed() {
    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    let app_state_body = app_src
        .split("pub struct AppState {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("AppState struct body");

    for field in FORBIDDEN_SERVICE_FIELDS {
        let needle = format!("pub {field}:");
        assert!(
            !app_state_body.contains(&needle),
            "AppState 不应再公开业务 service 旁路字段: {needle}"
        );
    }
    assert!(
        app_state_body.contains("pub orchestrator:"),
        "orchestrator 为收敛入口"
    );
    assert!(
        app_state_body.contains("pub runtime_profile:")
            && app_state_body.contains("pub managed_sidecars:"),
        "AppState 只额外持有 profile 与 managed-sidecar 生命周期"
    );

    let context_src = std::fs::read_to_string(orchestrator_context_rs_path())
        .expect("read orchestrator context.rs");
    let context_body = context_src
        .split("pub struct OrchestratorContext {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("OrchestratorContext struct body");
    assert!(
        context_body.contains("pub chapter:") && context_body.contains("pub(crate) rag_store:"),
        "chapter generation lock 与 RAG 基础设施应归 OrchestratorContext"
    );
    assert!(
        !context_src.contains("pub fn db(&self)")
            && !context_src.contains("pub fn app_config(&self)"),
        "OrchestratorContext 不应暴露零调用的数据库或配置旁路 facade"
    );
    let workflow_src = std::fs::read_to_string(orchestrator_workflow_rs_path())
        .expect("read orchestrator workflow.rs");
    assert!(
        !workflow_src.contains("mark_workflow_active_task")
            && !workflow_src.contains("clear_workflow_active_task_if_matches"),
        "workflow active_task_id 只能由持久队列状态同步，不应恢复手工旁路入口"
    );
    assert!(
        !app_src.contains("impl std::ops::DerefMut for AppState")
            && !app_src.contains("pub fn app_config(&self)"),
        "AppState 只读转发编排上下文，不应保留无调用的可变 Deref 或配置 facade"
    );
}

/// 运行时 action/workflow 不得再经 `state.<service>` 旁路（多行写法一并扫描）。
#[test]
fn runtime_actions_have_no_state_service_field_access() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    for path in walk_rs_files(&root) {
        if path.file_name().and_then(|n| n.to_str()) == Some("state.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read rs");
        let compact = collapse_ws(&text);
        for field in FORBIDDEN_SERVICE_FIELDS {
            if compact.contains(&format!("state .{field}"))
                || compact.contains(&format!("state.{field}"))
            {
                // 排除 `state.field_name` 更长标识符误匹配：要求 field 后非 [A-Za-z0-9_]
                if has_state_field_access(&compact, field) {
                    offenders.push(format!("{}: state.{}", path.display(), field));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "仍存在 state.<service> 旁路调用:\n{}",
        offenders.join("\n")
    );
}

const FORBIDDEN_SERVICE_FIELDS: &[&str] = &[
    "project",
    "object_profile",
    "scene_profile",
    "blueprint",
    "canvas",
    "outline",
    "portrait",
    "manga",
    "galgame",
    "image",
    "video",
    "optimize",
    "text_anchor",
    "media_asset",
    "source_sync",
];

fn app_state_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/state/app.rs")
}

fn orchestrator_context_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../a4n-orchestrator/src/context.rs")
}

fn orchestrator_workflow_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../a4n-orchestrator/src/workflow.rs")
}

fn collapse_ws(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn has_state_field_access(compact: &str, field: &str) -> bool {
    for prefix in ["state .", "state."] {
        let needle = format!("{prefix}{field}");
        let mut start = 0;
        while let Some(rel) = compact[start..].find(&needle) {
            let idx = start + rel + needle.len();
            let next = compact[idx..].chars().next();
            let ok = match next {
                None => true,
                Some(c) => !c.is_ascii_alphanumeric() && c != '_',
            };
            if ok {
                return true;
            }
            start = idx;
        }
    }
    false
}

fn walk_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&cur) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}
