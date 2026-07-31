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
        !app_state_body
            .lines()
            .any(|line| line.trim_start().starts_with("pub "))
            && app_state_body.contains("orchestrator:")
            && !app_state_body.contains("pub orchestrator:")
            && !app_state_body.contains("pub runtime_profile:")
            && !app_state_body.contains("pub managed_sidecars:")
            && app_state_body.contains("runtime_profile:")
            && app_state_body.contains("_managed_sidecars:"),
        "AppState 的 orchestrator、runtime profile 与 managed-sidecar 字段必须全部私有"
    );
    assert!(
        !app_src.contains("fn orchestrator(&self) -> &a4n_orchestrator::OrchestratorContext"),
        "runtime 不得恢复完整 orchestrator 上下文入口"
    );
    assert!(
        app_src.contains("pub(crate) fn runtime_profile(&self) -> RuntimeProfile"),
        "runtime profile 只能通过 crate 内只读窄方法访问"
    );

    let context_src = std::fs::read_to_string(orchestrator_context_rs_path())
        .expect("read orchestrator context.rs");
    let context_body = context_src
        .split("pub struct OrchestratorContext {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("OrchestratorContext struct body");
    for field in PRIVATE_CONTEXT_FIELDS {
        assert!(
            context_body.contains(&format!("{field}:"))
                && !context_body.contains(&format!("pub {field}:"))
                && !context_body.contains(&format!("pub(crate) {field}:")),
            "OrchestratorContext 字段必须保持私有: {field}"
        );
    }
    assert!(
        !context_body
            .lines()
            .any(|line| line.trim_start().starts_with("db:")
                || line.trim_start().starts_with("pub db:")
                || line.trim_start().starts_with("pub(crate) db:")),
        "OrchestratorContext 不得再持有 db 旁路字段；数据库只能由领域 owner 持有"
    );
    assert!(
        context_body.contains("chapter:") && context_body.contains("rag_store:"),
        "私有 chapter generation lock 与 RAG 基础设施应归 OrchestratorContext"
    );
    let context_owner_src = collapse_ws(&orchestrator_context_owner_sources());
    assert!(
        !context_owner_src.contains("fn db(&self)")
            && !context_owner_src.contains("fn app_config(&self)")
            && !context_owner_src.contains("fn config(&self)")
            && !context_owner_src.contains("fn event_bus(&self)")
            && !context_owner_src.contains("fn task_manager(&self)")
            && !context_owner_src.contains("fn snapshot_store(&self)")
            && !context_owner_src.contains("fn chapter_service(&self)")
            && !context_owner_src.contains("fn llm_provider(&self)")
            && !context_owner_src.contains("fn embedding_provider(&self)")
            && !context_owner_src.contains("fn prompt_engine(&self)")
            && !context_owner_src.contains("fn engine(&self)")
            && !context_owner_src.contains("fn task_queue(&self)")
            && !context_owner_src.contains("fn task_queue_handle(&self)")
            && !context_owner_src.contains("fn rag_store(&self)")
            && !context_owner_src.contains("fn vector_store(&self)")
            && !context_owner_src.contains("fn admin_application(&self)")
            && !context_owner_src.contains("fn admin_service(&self)")
            && !context_owner_src.contains("fn auth_application(&self)")
            && !context_owner_src.contains("fn auth_service(&self)")
            && !context_owner_src.contains("fn collaboration_application(&self)")
            && !context_owner_src.contains("fn collaboration_service(&self)")
            && !context_owner_src.contains("fn project_read_application(&self)")
            && !context_owner_src.contains("fn project_read_service(&self)")
            && !context_owner_src.contains("fn project_mutation_application(&self)")
            && !context_owner_src.contains("fn project_mutation_service(&self)")
            && !context_owner_src.contains("fn project_outline_records_application(&self)")
            && !context_owner_src.contains("fn project_outline_records_service(&self)")
            && !context_owner_src.contains("fn project_scene_records_application(&self)")
            && !context_owner_src.contains("fn project_scene_records_service(&self)")
            && !context_owner_src.contains("fn project_family_application(&self)")
            && !context_owner_src.contains("fn project_family_service(&self)")
            && !context_owner_src.contains("fn project_blueprint_application(&self)")
            && !context_owner_src.contains("fn project_blueprint_service(&self)")
            && !context_owner_src.contains("fn project_chapter_context_application(&self)")
            && !context_owner_src.contains("fn project_chapter_context_service(&self)")
            && !context_owner_src.contains("fn project_chapter_records_application(&self)")
            && !context_owner_src.contains("fn project_chapter_records_service(&self)")
            && !context_owner_src.contains("fn project_dependency_graph_application(&self)")
            && !context_owner_src.contains("fn project_dependency_graph_service(&self)")
            && !context_owner_src.contains("fn project_duplication_application(&self)")
            && !context_owner_src.contains("fn project_duplication_service(&self)")
            && !context_owner_src.contains("fn project_status_application(&self)")
            && !context_owner_src.contains("fn project_status_service(&self)")
            && !context_owner_src.contains("fn config_application(&self)")
            && !context_owner_src.contains("fn settings_application(&self)")
            && !context_owner_src.contains("fn settings_service(&self)")
            && !context_owner_src.contains("fn config_service(&self)")
            && !context_owner_src.contains("fn import_review_application(&self)")
            && !context_owner_src.contains("fn export_application(&self)")
            && !context_owner_src.contains("fn asset_application(&self)")
            && !context_owner_src.contains("fn scene_profile_application(&self)")
            && !context_owner_src.contains("fn chapter_content_application(&self)")
            && !context_owner_src.contains("fn character_application(&self)")
            && !context_owner_src.contains("fn character_service(&self)")
            && !context_owner_src.contains("fn outline_application(&self)")
            && !context_owner_src.contains("fn outline_service(&self)")
            && !context_owner_src.contains("fn workflow_application(&self)")
            && !context_owner_src.contains("fn workflow_service(&self)")
            && !context_owner_src.contains("fn manga_application(&self)")
            && !context_owner_src.contains("fn manga_service(&self)")
            && !context_owner_src.contains("fn rag_application(&self)")
            && !context_owner_src.contains("fn rag_service(&self)")
            && !context_owner_src.contains("fn optimize_application(&self)")
            && !context_owner_src.contains("fn optimize_service(&self)")
            && !context_owner_src.contains("fn galgame_application(&self)")
            && !context_owner_src.contains("fn galgame_service(&self)")
            && !context_owner_src.contains("fn video_application(&self)")
            && !context_owner_src.contains("fn video_service(&self)")
            && !context_owner_src.contains("fn canvas_application(&self)")
            && !context_owner_src.contains("fn canvas_service(&self)"),
        "OrchestratorContext 不应向 runtime 暴露数据库、完整配置、service 或 provider getter"
    );
    let workflow_src = std::fs::read_to_string(orchestrator_workflow_rs_path())
        .expect("read orchestrator workflow.rs");
    assert!(
        !workflow_src.contains("mark_workflow_active_task")
            && !workflow_src.contains("clear_workflow_active_task_if_matches"),
        "workflow active_task_id 只能由持久队列状态同步，不应恢复手工旁路入口"
    );
    assert!(
        !app_src.contains("impl std::ops::Deref for AppState")
            && !app_src.contains("impl std::ops::DerefMut for AppState")
            && !app_src.contains("pub fn app_config(&self)"),
        "AppState 不得恢复 Deref/DerefMut 或公开完整配置 facade"
    );
}

/// runtime `actions` 不得再经完整 `orchestrator()` 上下文直调（须走窄 facade / AppState 用途方法）。
#[test]
fn runtime_actions_have_no_full_orchestrator_context_calls() {
    let actions_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/actions");
    let mut offenders = Vec::new();
    for path in walk_rs_files(&actions_root) {
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read actions rs"));
        if text.contains(".orchestrator()") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "actions 仍存在 .orchestrator() 完整上下文直调:\n{}",
        offenders.join("\n")
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

/// Server/Tauri 不得绕过 runtime Host API 重新取得完整 orchestrator 上下文。
#[test]
fn hosts_have_no_direct_orchestrator_access() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest_dir.join("../a4n-server/src"),
        manifest_dir.join("../a4n-tauri/src"),
    ];
    let mut offenders = Vec::new();

    for root in roots {
        for path in walk_rs_files(&root) {
            let text = std::fs::read_to_string(&path).expect("read host rs");
            if text.contains(".orchestrator") {
                offenders.push(path.display().to_string());
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Server/Tauri 不得直接访问 AppState orchestrator:\n{}",
        offenders.join("\n")
    );
}

/// Admin runtime 适配只能使用窄 facade，不得恢复完整 OrchestratorContext 入口。
#[test]
fn admin_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = walk_rs_files(&manifest_dir.join("src/actions/admin_records"));
    paths.push(manifest_dir.join("src/actions/generation_analytics.rs"));
    let mut offenders = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).expect("read admin runtime rs");
        if collapse_ws(&text).contains(".orchestrator()") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Admin runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    assert!(
        app_src.contains("pub(crate) fn admin(&self) -> super::admin::AdminRuntimeFacade<'_>"),
        "AppState 必须只向 admin actions 提供窄 AdminRuntimeFacade"
    );
    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/admin.rs"))
        .expect("read state/admin.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "AdminRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Auth runtime 适配只能使用窄 facade，不得恢复完整 OrchestratorContext 入口。
#[test]
fn auth_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = walk_rs_files(&manifest_dir.join("src/actions/auth_records"));
    paths.push(manifest_dir.join("src/actions/oauth/flow.rs"));
    let mut offenders = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).expect("read auth runtime rs");
        if collapse_ws(&text).contains(".orchestrator()") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Auth runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    assert!(
        app_src.contains("pub(crate) fn auth(&self) -> super::auth::AuthRuntimeFacade<'_>"),
        "AppState 必须只向 auth actions 提供窄 AuthRuntimeFacade"
    );
    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/auth.rs"))
        .expect("read state/auth.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "AuthRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Collaboration runtime 适配只能使用窄 facade，不得恢复完整 OrchestratorContext 入口。
#[test]
fn collaboration_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [manifest_dir.join("src/actions/project_collaboration.rs")];
    let mut offenders = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).expect("read collaboration runtime rs");
        if collapse_ws(&text).contains(".orchestrator()") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Collaboration runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let permission_methods = [
        "verify_project_permission",
        "verify_project_permissions",
        "collaboration_root_project",
        "accessible_projects",
        "project_access_info",
        "project_access_info_batch",
    ];
    let facade_path = manifest_dir.join("src/state/collaboration.rs");
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if permission_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "collaboration 权限方法不得经 orchestrator 回流:\n{}",
        offenders.join("\n")
    );

    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    assert!(
        app_src.contains(
            "pub(crate) fn collaboration(&self) -> super::collaboration::CollaborationRuntimeFacade<'_>"
        ),
        "AppState 必须只向 collaboration actions 提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/collaboration.rs"))
        .expect("read state/collaboration.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "CollaborationRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project read 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_read_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_read.rs");
    let read_methods = [
        "project_family_metadata",
        "project_family_metadata_batch",
        "project_progress_summary",
        "project_progress_summaries",
        "project_rollback_impacts",
        "project_status_history",
        "project_ids",
        "project_kind",
        "project_by_id",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if read_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project read runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    assert!(
        app_src.contains(
            "pub(crate) fn project_read(&self) -> super::project_read::ProjectReadRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project read 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/project_read.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectReadRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project mutation 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_mutation_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_mutation.rs");
    let mutation_methods = [
        "create_project_validated",
        "import_text_project_from_json_validated",
        "create_derived_project_validated",
        "create_freeform_video_project_validated",
        "update_project_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if mutation_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project mutation runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_mutation( &self, ) -> super::project_mutation::ProjectMutationRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_mutation(&self) -> super::project_mutation::ProjectMutationRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project mutation 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/project_mutation.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectMutationRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project blueprint 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_blueprint_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_blueprint.rs");
    let blueprint_methods = [
        "novel_blueprint_for_user_validated",
        "blueprint_versions_for_user_validated",
        "diff_blueprint_versions_for_user_validated",
        "restore_blueprint_version_for_user_validated",
        "update_blueprint_world_setting_for_user_validated",
        "start_blueprint_generation_task",
        "start_blueprint_refinement_task",
        "preview_blueprint_generation_requests_for_user_validated",
        "preview_blueprint_refine_request_for_user_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if blueprint_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project blueprint runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_blueprint( &self, ) -> super::project_blueprint::ProjectBlueprintRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_blueprint(&self) -> super::project_blueprint::ProjectBlueprintRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project blueprint 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_blueprint.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectBlueprintRuntimeFacade 不得泄露完整 OrchestratorContext"
    );

    let versions_path = manifest_dir.join("src/actions/blueprint_records/versions.rs");
    let versions_src =
        std::fs::read_to_string(&versions_path).expect("read blueprint versions action");
    assert!(
        versions_src.contains(".diff_blueprint_versions_for_user_validated(")
            && versions_src.contains(".restore_blueprint_version_for_user_validated("),
        "蓝图版本 diff/restore 必须委托 Orchestrator application"
    );
    for forbidden in [
        "json_diff(",
        "blueprint_version_by_project_version",
        "restore_blueprint_version_record",
    ] {
        assert!(
            !versions_src.contains(forbidden),
            "蓝图版本 Runtime 不得恢复业务旁路: {forbidden}"
        );
    }
    assert!(
        !manifest_dir
            .join("src/actions/blueprint_records/diffing.rs")
            .exists(),
        "蓝图 JSON diff 旧 Runtime owner 不得恢复"
    );

    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/context/project_blueprint.rs"),
    )
    .expect("read Orchestrator project blueprint application");
    for owner in [
        "pub async fn diff_blueprint_versions_for_user_validated(",
        "pub async fn restore_blueprint_version_for_user_validated(",
        "fn blueprint_json_diff(",
    ] {
        assert!(
            application_src.contains(owner),
            "Orchestrator 必须拥有蓝图版本业务规则: {owner}"
        );
    }
}

/// Project duplication 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_duplication_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_duplication.rs");
    let duplication_methods = ["duplicate_project_validated", "branch_project_validated"];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if duplication_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project duplication runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_duplication( &self, ) -> super::project_duplication::ProjectDuplicationRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_duplication(&self) -> super::project_duplication::ProjectDuplicationRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project duplication 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_duplication.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectDuplicationRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    assert!(
        duplication_methods
            .iter()
            .all(|method| facade_src.contains(&format!("fn {method}"))),
        "ProjectDuplicationRuntimeFacade 必须只暴露 validated application 入口"
    );
    for removed in [
        "duplicate_text_project_record",
        "branch_text_project_record",
        "duplicate_derived_project_record",
        "duplicate_freeform_project_record",
    ] {
        assert!(
            !facade_src.contains(&format!("fn {removed}")),
            "ProjectDuplicationRuntimeFacade 不得恢复低层复制旁路: {removed}"
        );
    }
}

#[test]
fn project_duplication_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let action_root = manifest_dir.join("src/actions/project_duplication.rs");
    let execution_path = manifest_dir.join("src/actions/project_duplication/execution.rs");
    let media_path = manifest_dir.join("src/actions/project_duplication/media.rs");
    let removed_rules_path = manifest_dir.join("src/actions/project_duplication/rules.rs");
    let application_path =
        manifest_dir.join("../a4n-orchestrator/src/context/project_duplication.rs");

    let action_root_src =
        std::fs::read_to_string(&action_root).expect("read project_duplication.rs");
    let execution_src =
        std::fs::read_to_string(&execution_path).expect("read project duplication execution");
    let media_src =
        std::fs::read_to_string(&media_path).expect("read project duplication media Host Port");
    let application_src = std::fs::read_to_string(&application_path)
        .expect("read orchestrator project duplication application");

    assert!(
        !removed_rules_path.exists(),
        "Runtime 重复 rules.rs 必须删除"
    );
    assert!(!action_root_src.contains("mod rules;"));
    assert!(
        execution_src.contains(".duplicate_project_validated(")
            && execution_src.contains(".branch_project_validated(")
            && execution_src.contains("RuntimeProjectDuplicationHostPort::new(state)")
    );
    for forbidden in [
        "project_record_by_id",
        "required_project_status",
        "ensure_text_project_for_copy",
        "resolve_project_duplicate_path",
        "verify_project_editor_string",
        "error_with_rollback_file_cleanup",
        "duplicate_text_project_record",
        "duplicate_derived_project_record",
        "duplicate_freeform_project_record",
        "branch_text_project_record",
    ] {
        assert!(
            !execution_src.contains(forbidden),
            "Runtime duplication execution 不得恢复业务 owner: {forbidden}"
        );
    }
    assert!(
        media_src.contains("impl ProjectDuplicationHostPort")
            && media_src.contains("copy_media_file(")
            && media_src.contains("enqueue_file_artifact_cleanup_by_project(")
            && media_src.contains("verify_project_editor(self.state")
    );
    for required in [
        "pub async fn duplicate_project_validated",
        "pub async fn branch_project_validated",
        "resolve_project_duplicate_path",
        "required_project_status",
        "ensure_text_project_for_copy",
        "duplication_result_with_file_rollback",
        "rollback_copied_media_files",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator duplication application 缺少 owner 事实: {required}"
        );
    }
}

#[test]
fn video_studio_transfer_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/video_studio_transfer.rs");
    let methods = [
        "export_video_studio_project_validated",
        "import_video_studio_project_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Video Studio transfer runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn video_studio_transfer( &self, ) -> super::video_studio_transfer::VideoStudioTransferRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn video_studio_transfer(&self) -> super::video_studio_transfer::VideoStudioTransferRuntimeFacade<'_>"
        ),
        "AppState 必须只向 Video Studio transfer 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/video_studio_transfer.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "VideoStudioTransferRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    assert!(
        methods
            .iter()
            .all(|method| facade_src.contains(&format!("fn {method}"))),
        "VideoStudioTransferRuntimeFacade 必须只暴露 validated application 入口"
    );
}

#[test]
fn video_studio_transfer_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let api_path = manifest_dir.join("src/actions/project_transfer/video_studio_api.rs");
    let host_path =
        manifest_dir.join("src/actions/project_transfer/video_studio_package/host_port.rs");
    let state_canvas_path = manifest_dir.join("src/state/canvas.rs");
    let application_path =
        manifest_dir.join("../a4n-orchestrator/src/context/video_studio_transfer.rs");
    let removed_canvas_transfer = manifest_dir.join("../a4n-orchestrator/src/canvas/transfer.rs");
    let removed_runtime_files = [
        "src/actions/project_transfer/video_studio_package/export.rs",
        "src/actions/project_transfer/video_studio_package/export/media.rs",
        "src/actions/project_transfer/video_studio_package/import.rs",
        "src/actions/project_transfer/video_studio_package/import/writer.rs",
    ];

    let api_src = std::fs::read_to_string(&api_path).expect("read video studio API");
    let host_src = std::fs::read_to_string(&host_path).expect("read video studio Host Port");
    let state_canvas_src =
        std::fs::read_to_string(&state_canvas_path).expect("read state/canvas.rs");
    let application_src = std::fs::read_to_string(&application_path)
        .expect("read orchestrator video studio transfer application");

    assert!(
        !removed_canvas_transfer.exists(),
        "Orchestrator Canvas 裸 import record 旁路必须删除"
    );
    for relative in removed_runtime_files {
        assert!(
            !manifest_dir.join(relative).exists(),
            "Runtime Video Studio 旧 application owner 必须删除: {relative}"
        );
    }
    assert!(
        api_src.contains(".export_video_studio_project_validated(")
            && api_src.contains(".import_video_studio_project_validated(")
            && api_src.contains("RuntimeVideoStudioProjectTransferHostPort::new(state)")
    );
    for forbidden in [
        "verify_project_owner_string",
        "project_record_by_id",
        "video_studio_project_export_rows",
        "assemble_video_studio_export_package",
        "error_with_rollback_file_cleanup",
    ] {
        assert!(
            !api_src.contains(forbidden),
            "Runtime Video Studio API 不得恢复业务 owner: {forbidden}"
        );
    }
    assert!(
        host_src.contains("impl VideoStudioProjectTransferHostPort")
            && host_src.contains("package_zip_bytes(")
            && host_src.contains("read_video_studio_package_bytes(")
            && host_src.contains("self.state.runtime_profile()")
            && host_src.contains("self.state.capabilities()")
            && host_src.contains("self.state.data_dir()")
            && host_src.contains("write_new_data_file(")
            && host_src.contains("enqueue_file_artifact_cleanup_by_project(")
    );
    assert!(
        !state_canvas_src.contains("fn import_video_studio_package_validated"),
        "CanvasRuntimeFacade 不得保留 Video Studio 低层旁路: import_video_studio_package_validated"
    );
    for required in [
        "pub async fn export_video_studio_project_validated",
        "pub async fn import_video_studio_project_validated",
        ".verify_project_owner(",
        ".project_read",
        "video_studio_export_media_files",
        "assemble_video_studio_export_package",
        ".video_studio_project_export_rows(",
        ".import_video_studio_package_validated(",
        "import_result_with_file_rollback",
        "rollback_imported_media_files",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator Video Studio transfer application 缺少 owner 事实: {required}"
        );
    }
}

/// Project status 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_status_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_status.rs");
    let status_methods = [
        "forward_project_status_record",
        "rollback_project_status_record",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if status_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project status runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_status( &self, ) -> super::project_status::ProjectStatusRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_status(&self) -> super::project_status::ProjectStatusRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project status 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/project_status.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectStatusRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project dependency graph 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_dependency_graph_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_dependency_graph.rs");
    let graph_methods = [
        "asset_dependency_graph",
        "asset_dependency_graph_node_detail",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if graph_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project dependency graph runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_dependency_graph( &self, ) -> super::project_dependency_graph::ProjectDependencyGraphRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_dependency_graph(&self) -> super::project_dependency_graph::ProjectDependencyGraphRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project dependency graph 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_dependency_graph.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectDependencyGraphRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project chapter records 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_chapter_records_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_chapter_records.rs");
    let chapter_record_methods = [
        "validate_chapter_generation_readiness",
        "chapter_by_id",
        "chapter_by_project_number",
        "character_sync_chapters",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if chapter_record_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project chapter records runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_chapter_records( &self, ) -> super::project_chapter_records::ProjectChapterRecordsRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_chapter_records(&self) -> super::project_chapter_records::ProjectChapterRecordsRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project chapter records 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_chapter_records.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectChapterRecordsRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project outline records 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_outline_records_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_outline_records.rs");
    let outline_methods = [
        "chapter_outline_by_project_number",
        "chapter_outline_by_id",
        "update_chapter_outline_validated",
        "chapter_outlines_for_project",
        "part_outlines_for_project",
        "part_outline_by_id",
        "outline_generation_progress_rows",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if outline_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project outline records runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_outline_records( &self, ) -> super::project_outline_records::ProjectOutlineRecordsRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_outline_records(&self) -> super::project_outline_records::ProjectOutlineRecordsRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project outline records 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_outline_records.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectOutlineRecordsRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Project scene records 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_scene_records_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_scene_records.rs");
    let scene_methods = [
        "chapter_scenes_for_user_validated",
        "update_chapter_scene_for_user_validated",
        "delete_chapter_scene_for_user_validated",
        "reorder_chapter_scenes_for_user_validated",
        "chapter_scene_task_scope_for_user_validated",
        "prepare_chapter_scene_request_preview_for_user_validated",
        "generate_chapter_scene_for_user_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if scene_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project scene records runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_scene_records( &self, ) -> super::project_scene_records::ProjectSceneRecordsRuntimeFacade<'_>"
        ) || app_src.contains(
            "pub(crate) fn project_scene_records(&self) -> super::project_scene_records::ProjectSceneRecordsRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project scene records 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/project_scene_records.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectSceneRecordsRuntimeFacade 不得泄露完整 OrchestratorContext"
    );

    let chapter_scene_dir = manifest_dir.join("src/actions/chapter_scene");
    let mut chapter_scene_source = String::new();
    for path in walk_rs_files(&chapter_scene_dir) {
        chapter_scene_source
            .push_str(&std::fs::read_to_string(path).expect("read chapter_scene runtime source"));
        chapter_scene_source.push('\n');
    }
    for forbidden in [
        ".workflow()",
        ".llm()",
        "render_prompt_template",
        "save_generated_chapter_scene_record",
        "begin_chapter_scene_workflow_validated",
        "normalize_scene_generation_output",
        "restored_chapter_scene_checkpoint",
    ] {
        assert!(
            !chapter_scene_source.contains(forbidden),
            "chapter_scene runtime 不得恢复完整生成业务实现: {forbidden}"
        );
    }
    assert!(
        chapter_scene_source.contains(".generate_chapter_scene_for_user_validated("),
        "chapter_scene runtime 必须委托 orchestrator 完整生成 application"
    );
}

/// Project family 调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn project_family_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/project_family.rs");
    let family_methods = [
        "character_portrait_project_id",
        "protagonist_profile_project_id",
        "primary_material_image_ids_for_projects",
        "derived_project_family_context",
        "chapter_manga_source_by_id",
        "source_snapshot_chapters",
        "export_project_family_package_validated",
        "export_derived_project_package_validated",
        "import_project_family_package_validated",
        "delete_project_cascade_validated",
        "claim_project_deletion_cleanup",
        "claim_media_workflow_v2_reset",
        "reset_media_workflows_to_v2",
        "complete_claimed_cleanup",
        "renew_claimed_cleanup",
        "retry_claimed_cleanup",
        "unreferenced_media_paths",
        "media_path_prefix_has_persisted_reference",
        "reset_project_generated_data_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if family_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Project family runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn project_family(&self) -> super::project_family::ProjectFamilyRuntimeFacade<'_>"
        ),
        "AppState 必须只向 project family 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/project_family.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProjectFamilyRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    for forbidden in [
        "fn project_family_export_selection",
        "fn derived_project_export_selection",
        "fn project_family_export_rows",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "ProjectFamilyRuntimeFacade 不得保留低层 package 旁路: {forbidden}"
        );
    }
}

#[test]
fn project_family_transfer_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let api_path = manifest_dir.join("src/actions/project_transfer/family_api.rs");
    let host_path = manifest_dir.join("src/actions/project_transfer/family_package/host_port.rs");
    let facade_path = manifest_dir.join("src/state/project_family.rs");
    let application_path =
        manifest_dir.join("../a4n-orchestrator/src/context/project_family_transfer.rs");
    let removed_runtime_files = [
        "src/actions/project_transfer/family_package/export.rs",
        "src/actions/project_transfer/family_package/export/media.rs",
        "src/actions/project_transfer/family_package/export/package.rs",
        "src/actions/project_transfer/family_package/export/selection.rs",
        "src/actions/project_transfer/family_package/archive/lifecycle.rs",
        "src/actions/project_transfer/family_package/import_rows.rs",
        "src/actions/project_transfer/family_package/import_rows/media.rs",
        "src/actions/project_transfer/family_package/rules.rs",
    ];

    let api_src = std::fs::read_to_string(&api_path).expect("read project family API");
    let host_src = std::fs::read_to_string(&host_path).expect("read project family Host Port");
    let facade_src = std::fs::read_to_string(&facade_path).expect("read project family facade");
    let application_src = std::fs::read_to_string(&application_path)
        .expect("read orchestrator project family transfer application");

    for relative in removed_runtime_files {
        assert!(
            !manifest_dir.join(relative).exists(),
            "Runtime Project Family 旧 application owner 必须删除: {relative}"
        );
    }
    assert!(
        api_src.contains(".export_project_family_package_validated(")
            && api_src.contains(".export_derived_project_package_validated(")
            && api_src.contains(".import_project_family_package_validated(")
            && api_src.contains("RuntimeProjectFamilyTransferHostPort::new(state)")
    );
    for forbidden in [
        "verify_project_owner_string",
        "project_record_by_id",
        "export_project_family_package_bytes",
        "export_derived_project_package_bytes",
        "import_project_family_package_bytes",
        "project_family_export_selection",
        "project_family_export_rows",
        "assemble_project_family_export_package",
        "error_with_rollback_file_cleanup",
    ] {
        assert!(
            !api_src.contains(forbidden),
            "Runtime Project Family API 不得恢复业务 owner: {forbidden}"
        );
    }
    assert!(
        host_src.contains("impl ProjectFamilyTransferHostPort")
            && host_src.contains("package_zip_bytes(")
            && host_src.contains("read_project_family_package_bytes(")
            && host_src.contains("self.state.runtime_profile()")
            && host_src.contains("self.state.capabilities()")
            && host_src.contains("self.state.data_dir()")
            && host_src.contains("write_new_data_file(")
            && host_src.contains("enqueue_file_artifact_cleanup_by_project(")
    );
    for forbidden in [
        "fn project_family_export_selection",
        "fn derived_project_export_selection",
        "fn project_family_export_rows",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "ProjectFamilyRuntimeFacade 不得保留低层 transfer 旁路: {forbidden}"
        );
    }
    for required in [
        "pub async fn export_project_family_package_validated",
        "pub async fn export_derived_project_package_validated",
        "pub async fn import_project_family_package_validated",
        ".verify_project_owner(",
        ".project_read",
        ".export_selection(",
        ".derived_export_selection(",
        ".export_rows(",
        "project_family_export_media_files",
        "assemble_project_family_export_package",
        "validate_family_package_manifest",
        "validate_family_media_manifest",
        ".import_package_rows(",
        "import_result_with_file_rollback",
        "rollback_imported_media_files",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator Project Family transfer application 缺少 owner 事实: {required}"
        );
    }
}

/// Config/主题/系统设置调用只能使用窄 facade，不得从 AppState 恢复完整上下文入口。
#[test]
fn settings_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/settings.rs");
    let settings_methods = [
        "list_user_themes",
        "list_themes_visible_to_user",
        "theme_by_id",
        "create_theme",
        "update_theme",
        "delete_theme",
        "activate_theme",
        "list_llm_config_records",
        "llm_config_record_by_id",
        "create_llm_config_validated",
        "update_llm_config_validated",
        "set_default_llm_config_validated",
        "delete_llm_config_if_allowed",
        "resolve_llm_config",
        "llm_max_tokens_config",
        "save_llm_max_tokens_config_validated",
        "list_image_config_records",
        "image_config_record_by_id",
        "create_image_config_validated",
        "update_image_config_validated",
        "set_default_image_config_validated",
        "delete_image_config_if_allowed",
        "resolve_image_config",
        "video_config",
        "list_video_config_records",
        "video_config_record_by_id",
        "create_video_config_validated",
        "update_video_config_validated",
        "set_default_video_config_validated",
        "delete_video_config_if_allowed",
        "default_video_config",
        "active_generated_video_ids_for_config",
        "list_embedding_configs",
        "embedding_config_by_id",
        "create_embedding_config_validated",
        "update_embedding_config_validated",
        "set_default_embedding_config_validated",
        "delete_embedding_config_if_allowed",
        "export_all_configs_to_json_validated",
        "import_all_configs_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if settings_methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Settings/config runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn settings(&self) -> super::settings::SettingsRuntimeFacade<'_>"
        ),
        "AppState 必须只向 settings/config 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/settings.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "SettingsRuntimeFacade 不得泄露完整 OrchestratorContext"
    );

    let actions_root = manifest_dir.join("src/actions/settings_all");
    let runtime_src = std::fs::read_to_string(actions_root.join("mod.rs"))
        .expect("read settings_all runtime wrapper");
    assert!(
        runtime_src.contains("export_all_configs_to_json_validated")
            && runtime_src.contains("import_all_configs_validated"),
        "settings_all Runtime 必须委托 Orchestrator application"
    );
    let host_src = std::fs::read_to_string(actions_root.join("host_port.rs"))
        .expect("read settings_all host port");
    assert!(
        host_src.contains("impl SettingsAllHostPort")
            && host_src.contains("render_export_json")
            && host_src.contains("normalize_import_json")
            && host_src.contains("prompt_update_many_runtime"),
        "settings_all Runtime 只应承载 sidecar/Prompt Host 边界"
    );
    for legacy in ["export", "import", "validation", "types.rs"] {
        assert!(
            !actions_root.join(legacy).exists(),
            "settings_all Runtime 旧业务 owner 不得恢复: {legacy}"
        );
    }
    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/config/settings_all_application.rs"),
    )
    .expect("read settings_all Orchestrator application");
    assert!(
        application_src.contains("pub trait SettingsAllHostPort")
            && application_src.contains("export_all_configs_to_json_validated")
            && application_src.contains("import_all_configs_validated")
            && application_src.contains("rollback_prompts"),
        "Orchestrator 必须拥有 settings_all 导入/导出与回滚 application"
    );
}

/// 叶领域（import review / export / asset / scene profile / chapter content）只能使用窄 facade。
#[test]
fn leaf_domain_runtime_uses_narrow_facades() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domains = [
        (
            "import_review",
            manifest_dir.join("src/state/import_review.rs"),
            &[
                "import_reviews_by_project",
                "import_review_by_id",
                "reject_import_review",
                "delete_import_review",
                "accept_import_review_for_user_validated",
                "modify_import_review_for_user_validated",
                "batch_accept_import_reviews_for_user_validated",
                "check_import_dedup_for_user_validated",
                "update_import_review_status_validated",
            ][..],
            "pub(crate) fn import_review(&self) -> super::import_review::ImportReviewRuntimeFacade<'_>",
        ),
        (
            "export",
            manifest_dir.join("src/state/export.rs"),
            &[
                "export_txt",
                "export_markdown",
                "export_chapters",
                "export_outline_json",
                "export_characters_json",
            ][..],
            "pub(crate) fn export(&self) -> super::export::ExportRuntimeFacade<'_>",
        ),
        (
            "asset",
            manifest_dir.join("src/state/asset.rs"),
            &[
                "object_profiles_by_project",
                "object_profile_by_id",
                "create_object_profile_validated",
                "update_object_profile_validated",
                "delete_object_profile_record_validated",
            ][..],
            "pub(crate) fn asset(&self) -> super::asset::AssetRuntimeFacade<'_>",
        ),
        (
            "scene_profile",
            manifest_dir.join("src/state/scene_profile.rs"),
            &[
                "scene_profiles_by_project",
                "scene_profile_by_id",
                "create_scene_profile_validated",
                "update_scene_profile_validated",
                "delete_scene_profile_records_validated",
                "sync_scene_profiles_for_chapters_validated",
                "scene_occurrence_count",
                "scene_occurrence_counts_by_profile",
            ][..],
            "pub(crate) fn scene_profile(&self) -> super::scene_profile::SceneProfileRuntimeFacade<'_>",
        ),
        (
            "chapter_content",
            manifest_dir.join("src/state/chapter_content.rs"),
            &[
                "chapters_for_project",
                "preflight_chapter_content_change_validated",
                "postprocess_chapter_after_content_change_validated",
                "delete_latest_chapter_outlines_validated",
                "import_chapter_validated",
                "update_chapter_content_validated",
            ][..],
            "pub(crate) fn chapter_content( &self, ) -> super::chapter_content::ChapterContentRuntimeFacade<'_>",
        ),
    ];

    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));

    for (name, facade_path, methods, app_method) in domains {
        let mut offenders = Vec::new();
        for path in walk_rs_files(&manifest_dir.join("src")) {
            if path == facade_path {
                continue;
            }
            let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
            if methods
                .iter()
                .any(|method| text.contains(&format!(".orchestrator().{method}(")))
            {
                offenders.push(path.display().to_string());
            }
        }
        assert!(
            offenders.is_empty(),
            "{name} runtime 不得恢复完整 orchestrator 入口:\n{}",
            offenders.join("\n")
        );
        let expected = collapse_ws(app_method);
        assert!(
            app_src.contains(&expected)
                || app_src.contains(&expected.replace("( &self, )", "(&self)")),
            "AppState 必须只向 {name} 调用提供窄 facade"
        );
        let facade_src = std::fs::read_to_string(&facade_path)
            .unwrap_or_else(|_| panic!("read {}", facade_path.display()));
        assert!(
            !facade_src.contains("fn orchestrator(")
                && !facade_src.contains("fn context(")
                && !facade_src.contains("pub orchestrator:"),
            "{name} RuntimeFacade 不得泄露完整 OrchestratorContext"
        );
    }

    let chapter_actions = [
        manifest_dir.join("src/actions/chapter_content/import.rs"),
        manifest_dir.join("src/actions/chapter_content/edit.rs"),
    ]
    .into_iter()
    .map(|path| std::fs::read_to_string(path).expect("read chapter content action"))
    .collect::<String>();
    assert!(
        chapter_actions.contains("RuntimeChapterAnalysisHostPort::new(state)")
            && chapter_actions.contains(".import_chapter_validated(")
            && chapter_actions.contains(".update_chapter_content_validated("),
        "chapter_content Runtime 只能构造 Host port 并委托 application"
    );
    for forbidden in [
        "a4n_db::",
        "begin_workflow",
        "finalize_workflow",
        "transaction",
    ] {
        assert!(
            !chapter_actions.contains(forbidden),
            "chapter_content Runtime 不得恢复 DB/workflow 组合: {forbidden}"
        );
    }
    let chapter_application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/chapter_content/application.rs"),
    )
    .expect("read Orchestrator chapter content application");
    for owner in [
        "preflight_chapter_content_change_validated(",
        "postprocess_chapter_after_content_change_validated(",
        "delete_latest_chapter_outlines_validated(",
        "import_chapter_validated(",
        "update_chapter_content_validated(",
    ] {
        assert!(
            chapter_application_src.contains(owner),
            "Orchestrator 必须拥有 chapter_content application: {owner}"
        );
    }
}

/// 导入审核接受、后处理、批量部分失败和去重判断必须由 Orchestrator application 单点拥有。
#[test]
fn import_review_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let actions_root = manifest_dir.join("src/actions/import_review");
    let acceptance_src =
        std::fs::read_to_string(actions_root.join("acceptance.rs")).expect("read acceptance.rs");
    let dedup_src = std::fs::read_to_string(actions_root.join("dedup.rs")).expect("read dedup.rs");
    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/import_review.rs"))
        .expect("read state/import_review.rs");
    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/context/import_review.rs"),
    )
    .expect("read Orchestrator import review application");
    let rules_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/import_review/rules.rs"),
    )
    .expect("read Orchestrator import review rules");

    for required in [
        ".accept_import_review_for_user_validated(",
        ".modify_import_review_for_user_validated(",
        ".batch_accept_import_reviews_for_user_validated(",
    ] {
        assert!(
            acceptance_src.contains(required),
            "Import review Runtime 必须委托完整 application: {required}"
        );
    }
    assert!(
        dedup_src.contains(".check_import_dedup_for_user_validated("),
        "Import review Runtime 必须委托去重 application"
    );
    let runtime_src = format!("{acceptance_src}\n{dedup_src}");
    for forbidden in [
        ".accept_import_review_validated(",
        ".modify_import_review_validated(",
        ".import_dedup_chapters(",
        ".postprocess_chapter_after_content_change_validated(",
        "compute_simhash(",
        "hamming_distance(",
        "for review_id in input.review_ids",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Import review Runtime 不得恢复业务 owner: {forbidden}"
        );
    }

    for required in [
        "pub async fn accept_import_review_for_user_validated",
        "pub async fn modify_import_review_for_user_validated",
        "pub async fn batch_accept_import_reviews_for_user_validated",
        "pub async fn check_import_dedup_for_user_validated",
        "finish_import_review_acceptance",
        "verify_project_permission",
        "postprocess_chapter_after_content_change_validated",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator import review application 缺少 owner 事实: {required}"
        );
    }
    assert!(
        rules_src.contains("fn calculate_import_dedup(")
            && rules_src.contains("DUPLICATE_HAMMING_THRESHOLD")
            && rules_src.contains("compute_simhash("),
        "Orchestrator import review rules 必须唯一拥有去重判断"
    );
    for removed in [
        "fn accept_import_review_validated(",
        "fn modify_import_review_validated(",
        "fn import_dedup_chapters(",
    ] {
        assert!(
            !facade_src.contains(removed),
            "ImportReviewRuntimeFacade 不得保留低层旁路: {removed}"
        );
    }
}

/// 工作包 01 的剩余入口必须只在 Runtime 保留边界适配，完整流程归 Orchestrator。
#[test]
fn project_transfer_import_export_residuals_use_application_owners() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let orchestrator_dir = manifest_dir.join("../a4n-orchestrator/src");

    let export_runtime = walk_rs_files(&manifest_dir.join("src/actions/export_files"))
        .into_iter()
        .map(|path| std::fs::read_to_string(path).expect("read export runtime source"))
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        ".run_export_txt_validated(",
        ".run_export_markdown_validated(",
        ".run_export_outline_json_validated(",
        ".run_export_characters_json_validated(",
        ".run_export_epub_validated(",
        ".run_export_pdf_validated(",
        ".run_batch_export_validated(",
    ] {
        assert!(
            export_runtime.contains(required),
            "Export Runtime 必须委托 application: {required}"
        );
    }
    for forbidden in [
        "a4n_db::",
        "sea_orm",
        "export_chapter_rows(",
        "normalize_batch_export_formats(",
        "render_text_pdf(",
        "EpubBuilder",
    ] {
        assert!(
            !export_runtime.contains(forbidden),
            "Export Runtime 不得恢复读取、排序或格式生成 owner: {forbidden}"
        );
    }
    let export_application =
        std::fs::read_to_string(orchestrator_dir.join("export/application.rs"))
            .expect("read export application");
    for required in [
        "pub async fn run_export_txt_validated",
        "pub async fn run_export_markdown_validated",
        "pub async fn run_export_outline_json_validated",
        "pub async fn run_export_characters_json_validated",
        "pub async fn run_export_epub_validated",
        "pub async fn run_export_pdf_validated",
        "pub async fn run_batch_export_validated",
        "normalize_batch_export_formats",
        "render_text_pdf",
        "EpubBuilder",
    ] {
        assert!(
            export_application.contains(required),
            "Orchestrator export application 缺少 owner 事实: {required}"
        );
    }

    let text_import_runtime =
        std::fs::read_to_string(manifest_dir.join("src/actions/project_transfer/text_project.rs"))
            .expect("read text project import runtime");
    assert!(
        text_import_runtime.contains(".run_import_text_project_validated("),
        "文本项目完整导入必须委托 Orchestrator application"
    );
    for forbidden in [
        "import_text_project_from_json_validated(",
        "postprocess_chapter_after_content_change_validated(",
        "a4n_db::",
    ] {
        assert!(
            !text_import_runtime.contains(forbidden),
            "文本项目 Runtime 不得恢复导入或后处理 owner: {forbidden}"
        );
    }
    let text_import_application =
        std::fs::read_to_string(orchestrator_dir.join("project/text_project_import.rs"))
            .expect("read text project import application");
    for required in [
        "pub async fn run_import_text_project_validated",
        "import_text_project_from_json_validated",
        "postprocess_chapter_after_content_change_validated",
    ] {
        assert!(
            text_import_application.contains(required),
            "Orchestrator 文本项目导入缺少 owner 事实: {required}"
        );
    }

    let reverse_runtime = walk_rs_files(&manifest_dir.join("src/actions/import_reverse"))
        .into_iter()
        .map(|path| std::fs::read_to_string(path).expect("read import reverse runtime source"))
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        ".run_reverse_engineering_validated(",
        ".prepare_reverse_engineer_request_validated(",
    ] {
        assert!(
            reverse_runtime.contains(required),
            "Import Reverse Runtime 必须委托 application: {required}"
        );
    }
    for forbidden in [
        "begin_import_reverse_workflow_validated(",
        ".reverse_engineer_request(",
        ".apply_reverse_",
        "finalize_workflow_run_outcome_validated(",
        "a4n_db::",
    ] {
        assert!(
            !reverse_runtime.contains(forbidden),
            "Import Reverse Runtime 不得恢复 workflow/LLM/apply owner: {forbidden}"
        );
    }
    let reverse_application =
        std::fs::read_to_string(orchestrator_dir.join("import_reverse/application.rs"))
            .expect("read import reverse application");
    for required in [
        "pub async fn prepare_reverse_engineer_request_validated",
        "pub async fn run_reverse_engineering_validated",
        "begin_import_reverse_workflow_validated",
        "run_reverse_engineering_workflow",
        "finalize_workflow_run_outcome_validated",
        "apply_reverse_engineering_result",
    ] {
        assert!(
            reverse_application.contains(required),
            "Orchestrator Import Reverse 缺少 owner 事实: {required}"
        );
    }

    let import_analysis_runtime = walk_rs_files(&manifest_dir.join("src/actions/import_analysis"))
        .into_iter()
        .map(|path| std::fs::read_to_string(path).expect("read import analysis runtime source"))
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        ".run_import_project_analysis_validated(",
        ".prepare_import_project_analysis_preview_validated(",
        ".create_import_analysis_reviews(",
    ] {
        assert!(
            import_analysis_runtime.contains(required),
            "Import Analysis Runtime 必须委托 Orchestrator: {required}"
        );
    }
    for forbidden in [
        "prepare_import_project_analysis(",
        "record_import_analysis_failure(",
        "run_import_analysis_outlines(",
        "run_import_analysis_parts(",
        "save_import_stage(",
        "a4n_db::",
    ] {
        assert!(
            !import_analysis_runtime.contains(forbidden),
            "Import Analysis Runtime 不得恢复 workflow/checkpoint owner: {forbidden}"
        );
    }
    let import_analysis_application =
        std::fs::read_to_string(orchestrator_dir.join("import_analysis/application.rs"))
            .expect("read import analysis application");
    for required in [
        "pub async fn run_import_project_analysis_validated",
        "pub async fn prepare_import_project_analysis_preview_validated",
        "prepare_import_project_analysis",
        "record_import_analysis_failure",
        "run_import_analysis_outlines",
        "run_import_analysis_parts",
        "save_import_stage",
    ] {
        assert!(
            import_analysis_application.contains(required),
            "Orchestrator Import Analysis 缺少 owner 事实: {required}"
        );
    }

    let import_file_runtime = [
        "src/actions/import_file.rs",
        "src/actions/import_file/parsing.rs",
        "src/actions/import_file/upload.rs",
    ]
    .into_iter()
    .map(|relative| {
        std::fs::read_to_string(manifest_dir.join(relative))
            .unwrap_or_else(|_| panic!("read {relative}"))
    })
    .collect::<Vec<_>>()
    .join("\n");
    assert!(
        import_file_runtime.contains("decode_import_upload(input)?")
            && import_file_runtime.contains("parse_import_file_bytes("),
        "Import File Runtime 只能保留上传解码、parser 与 DTO 投影"
    );
    for forbidden in [
        "AppState",
        "OrchestratorContext",
        "a4n_db::",
        "sea_orm",
        "transaction",
        "project_mutation",
        "workflow",
    ] {
        assert!(
            !import_file_runtime.contains(forbidden),
            "Import File parser 边界不得引入项目状态或 workflow: {forbidden}"
        );
    }
}

/// 场景档案同步完整 application 只能由 Orchestrator 拥有。
#[test]
fn scene_profile_sync_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_dir = manifest_dir.join("src/actions/scene_profile_sync");
    let runtime_files = walk_rs_files(&runtime_dir);
    let relative_files = runtime_files
        .iter()
        .map(|path| {
            path.strip_prefix(&manifest_dir)
                .expect("runtime file must be below manifest")
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        relative_files,
        vec![
            "src/actions/scene_profile_sync/execution/scope.rs",
            "src/actions/scene_profile_sync/execution.rs",
        ],
        "Scene Profile Sync Runtime 只能保留权限、Host Port 与进度适配"
    );

    let runtime_source = runtime_files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read scene profile runtime source"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "begin_scene_profile_sync_workflow_validated(",
        "replace_chapter_scene_sync_from_input_validated(",
        "load_completed_llm_call_checkpoint(",
        "save_completed_llm_call_checkpoint(",
        "load_completed_workflow_checkpoint(",
        "save_completed_workflow_checkpoint(",
        "chat_json_stream_with_progress(",
        "parse_scene_profile_sync_content(",
        "bind_scene_profile_evidence(",
        "build_scene_profile_stage_request(",
    ] {
        assert!(
            !runtime_source.contains(forbidden),
            "Scene Profile Sync Runtime 不得恢复业务 owner: {forbidden}"
        );
    }
    assert!(
        runtime_source.contains(".sync_scene_profiles_for_chapters_validated("),
        "Scene Profile Sync Runtime 必须只委托完整 application"
    );

    let application_dir = manifest_dir.join("../a4n-orchestrator/src/scene_profile/application");
    let application_source = walk_rs_files(&application_dir)
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read scene profile application source"))
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "pub async fn sync_scene_profiles_for_chapters_validated(",
        "pub async fn prepare_scene_profile_sync_preview_validated(",
        "begin_scene_profile_sync_workflow_validated(",
        "load_completed_llm_call_checkpoint(",
        "save_completed_llm_call_checkpoint(",
        "load_completed_workflow_checkpoint(",
        "save_completed_workflow_checkpoint(",
        "chat_json_stream_with_progress(",
        "parse_scene_profile_sync_content(",
        "bind_scene_profile_evidence(",
        "replace_chapter_scene_sync_from_input_validated(",
        "sync_scene_rag_records(",
    ] {
        assert!(
            application_source.contains(required),
            "Scene Profile Sync Orchestrator application 必须拥有 {required}"
        );
    }
}

/// Character 领域 runtime 只能使用窄 facade。
#[test]
fn character_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/character.rs");
    let methods = [
        "profiles_by_project",
        "profile_by_id",
        "profile_by_project_name",
        "missing_portrait_profiles",
        "character_portrait_by_id",
        "character_portrait_records_by_project",
        "primary_character_portrait_record",
        "set_primary_character_portrait_record",
        "portrait_reference_images_by_ids",
        "create_profile_validated",
        "attribute_history",
        "add_profile_attribute_validated",
        "modify_profile_attribute_validated",
        "execute_ready_attribute_deletion_validated",
        "apply_implicit_update_decision_validated",
        "behavior_records_by_profile",
        "implicit_review_summary_validated",
        "prepare_implicit_update_review_request_preview_validated",
        "check_implicit_update_validated",
        "deletion_marks_by_profile_validated",
        "request_attribute_deletion_validated",
        "reset_deletion_marks_validated",
        "snapshot_references_for_user_validated",
        "profile_snapshot_for_user_at_chapter_validated",
        "character_snapshot_for_user_by_hash_validated",
        "diff_profile_snapshots_for_user_validated",
        "character_sync_conflict_for_user_validated",
        "character_conflict_task_scope_validated",
        "prepare_character_conflict_request_preview_validated",
        "check_character_conflict_validated",
        "character_discovery_task_scope_validated",
        "prepare_character_discovery_request_preview_validated",
        "discover_character_profiles_validated",
        "delete_character_profile_records_validated",
        "rollback_character_profile_timeline_validated",
        "begin_workflow_run_validated",
        "begin_video_prompt_optimization_workflow_validated",
        "begin_video_prompt_workflow_validated",
        "prepare_generate_portrait_request_preview_validated",
        "prepare_generate_portrait_prompt_request_preview_validated",
        "generate_portrait_prompt_validated",
        "generate_portrait_validated",
        "upload_portrait_validated",
        "begin_outline_workflow_validated",
        "begin_image_generation_workflow_validated",
        "begin_batch_image_workflow_validated",
        "finalize_workflow_run_validated",
        "finalize_workflow_run_outcome_validated",
        "complete_workflow_run_validated",
        "fail_workflow_run_validated",
        "cancel_workflow_run_status_validated",
        "mark_workflow_running_validated",
        "prepare_character_profile_sync_preview_validated",
        "run_character_profile_sync_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Character runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let character_conflict_dir = manifest_dir.join("src/actions/character_conflict");
    let mut character_conflict_source = String::new();
    for path in walk_rs_files(&character_conflict_dir) {
        character_conflict_source.push_str(
            &std::fs::read_to_string(path).expect("read character_conflict runtime source"),
        );
        character_conflict_source.push('\n');
    }
    for forbidden in [
        ".workflow()",
        ".llm()",
        "render_prompt_template",
        "prompt_render_async",
        "character_conflict_context_rows",
        "begin_character_conflict_workflow_validated",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "parse_conflict_output",
        "restored_character_conflict_checkpoint",
    ] {
        assert!(
            !character_conflict_source.contains(forbidden),
            "character_conflict runtime 不得恢复完整检查业务实现: {forbidden}"
        );
    }
    assert!(
        character_conflict_source.contains(".check_character_conflict_validated("),
        "character_conflict runtime 必须委托 orchestrator 完整检查 application"
    );
    assert!(
        character_conflict_source
            .contains(".prepare_character_conflict_request_preview_validated("),
        "character_conflict runtime 预览必须委托 orchestrator request application"
    );
    let mut character_discovery_source = String::new();
    for dir in [
        manifest_dir.join("src/actions/character_ai/character_discovery_flow"),
        manifest_dir.join("src/actions/character_ai/character_discovery"),
    ] {
        for path in walk_rs_files(&dir) {
            character_discovery_source.push_str(
                &std::fs::read_to_string(path).expect("read character_discovery runtime source"),
            );
            character_discovery_source.push('\n');
        }
    }
    for forbidden in [
        ".workflow()",
        ".llm()",
        "prompt_render_async",
        "render_prompt_template",
        "chat_json_stream_with_progress",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "begin_character_discovery_workflow_validated",
        "character_discovery_call_fingerprint_json",
        "parse_character_discovery_output",
        "create_profile_with_generation_run",
        "merge_profile_evidence_records",
    ] {
        assert!(
            !character_discovery_source.contains(forbidden),
            "character_discovery runtime 不得恢复完整发现业务实现: {forbidden}"
        );
    }
    assert!(
        character_discovery_source.contains(".discover_character_profiles_validated("),
        "character_discovery runtime 必须委托 orchestrator 完整发现 application"
    );
    assert!(
        character_discovery_source
            .contains(".prepare_character_discovery_request_preview_validated("),
        "character_discovery runtime 预览必须委托 orchestrator request application"
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn character(&self) -> super::character::CharacterRuntimeFacade<'_>"
        ),
        "AppState 必须只向 character 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/character.rs");
    for forbidden in [
        "snapshot_references_by_profile(",
        "snapshot_reference_by_chapter(",
        "character_snapshot_by_hash(",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "Character facade 不得暴露快照低层入口: {forbidden}"
        );
    }
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "CharacterRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Portrait generation runtime 只能保留权限、任务/预览和图片/文件 Host port。
#[test]
fn portrait_generation_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut source =
        std::fs::read_to_string(manifest_dir.join("src/actions/portrait_generation.rs"))
            .expect("read portrait_generation runtime root");
    source.push('\n');
    for path in walk_rs_files(&manifest_dir.join("src/actions/portrait_generation")) {
        source.push_str(
            &std::fs::read_to_string(path).expect("read portrait_generation runtime source"),
        );
        source.push('\n');
    }
    for forbidden in [
        ".workflow()",
        ".llm()",
        ".generation()",
        ".video()",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "load_completed_workflow_checkpoint",
        "save_completed_workflow_checkpoint",
        "begin_portrait_generation_workflow_validated",
        "begin_portrait_prompt_workflow_validated",
        "begin_missing_portrait_workflow_validated",
        "begin_portrait_image_workflow_validated",
        "record_completed_generation_run_with_checkpoint",
        "create_generated_image_record_with_generation_run_checkpoint",
        "create_character_portrait_record",
        "create_character_portrait_for_profile",
        "rollback_failed_portrait_creation_records",
        "persist_generated_portrait",
        "mirror_portrait_to_generated_image",
    ] {
        assert!(
            !source.contains(forbidden),
            "portrait_generation runtime 不得恢复完整生成业务实现: {forbidden}"
        );
    }
    for required in [
        ".start_portrait_generation_task(",
        ".run_portrait_generation_task(",
        ".generate_portrait_prompt_validated(",
        ".upload_portrait_validated(",
        ".prepare_generate_portrait_request_preview_validated(",
        ".prepare_generate_portrait_prompt_request_preview_validated(",
    ] {
        assert!(
            source.contains(required),
            "portrait_generation runtime 必须委托 orchestrator application: {required}"
        );
    }
}

/// Character implicit review runtime 只能保留权限、Host DTO 和同步流程适配。
#[test]
fn character_implicit_review_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut source = String::new();
    for relative in [
        "src/actions/character_implicit_review",
        "src/actions/character_ai/sync_implicit/review",
    ] {
        let directory = manifest_dir.join(relative);
        for path in walk_rs_files(&directory) {
            source.push_str(
                &std::fs::read_to_string(path)
                    .unwrap_or_else(|_| panic!("read {relative} runtime source")),
            );
            source.push('\n');
        }
    }
    for forbidden in [
        ".workflow()",
        ".llm()",
        "render_prompt_template",
        "context_aware_system_user_json_llm_request",
        "chat_llm(",
        "stream_complete_llm(",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "parse_implicit_update_decision",
        "build_implicit_review_summary",
        "build_implicit_attribute_review",
        "implicit_update_review_checkpoint_response",
        "begin_implicit_update_review_workflow_validated",
        "apply_implicit_review_decision_record",
        "implicit_review_behavior_records",
        "behavior_records_before_chapter",
    ] {
        assert!(
            !source.contains(forbidden),
            "character implicit review runtime 不得恢复完整复审业务实现: {forbidden}"
        );
    }
    for required in [
        ".implicit_review_summary_validated(",
        ".prepare_implicit_update_review_request_preview_validated(",
        ".check_implicit_update_validated(",
        ".apply_implicit_update_decision_validated(",
    ] {
        assert!(
            source.contains(required),
            "character implicit review runtime 必须委托 application: {required}"
        );
    }
}

/// Character implicit classification runtime 只能调用单一 Orchestrator application 并投影输出。
#[test]
fn character_implicit_classification_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let removed_runtime_files = [
        "src/actions/character_ai/implicit_classification.rs",
        "src/actions/character_ai/implicit_classification/application.rs",
        "src/actions/character_ai/implicit_classification/parsing.rs",
        "src/actions/character_ai/implicit_classification/prompt.rs",
        "src/actions/character_ai/sync_implicit/checkpoint.rs",
        "src/actions/character_ai/sync_implicit/classification.rs",
        "src/actions/character_ai/sync_implicit/classification/execution.rs",
        "src/actions/character_ai/sync_implicit/classification/execution/call.rs",
        "src/actions/character_ai/sync_implicit/classification/request.rs",
        "src/actions/character_ai/state_projection/events/application.rs",
        "src/actions/character_ai/state_projection/events/builders.rs",
    ];
    for relative in removed_runtime_files {
        assert!(
            !manifest_dir.join(relative).exists(),
            "Character implicit classification Runtime 旧 owner 必须删除: {relative}"
        );
    }

    let runtime_files = [
        "src/actions/character_ai/mod.rs",
        "src/actions/character_ai/sync_profile.rs",
        "src/actions/character_ai/sync_profile/api/execution.rs",
        "src/actions/character_ai/state_projection.rs",
        "src/actions/character_ai/state_projection/events.rs",
    ];
    let mut runtime_source = String::new();
    for relative in runtime_files {
        runtime_source.push_str(
            &std::fs::read_to_string(manifest_dir.join(relative))
                .unwrap_or_else(|_| panic!("read {relative}")),
        );
        runtime_source.push('\n');
    }
    assert!(
        !runtime_source.contains(".classify_character_sync_behaviors_validated("),
        "Character Profile Sync Runtime 不得直接编排 classification application"
    );
    for forbidden in [
        "fn classify_behaviors(",
        "parse_implicit_classification_output",
        "build_implicit_classification_manifest_user_prompt",
        "prepare_implicit_classification_call",
        "run_implicit_classification_call",
        "mark_workflow_current_step",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "apply_implicit_classification_to_behavior",
        "set_behavior_classification_results",
        "pub(crate) const PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT",
        "fn project_attribute_changes(",
        "fn pending_attribute_events(",
        "struct AttributeEventDraft",
    ] {
        assert!(
            !runtime_source.contains(forbidden),
            "Character implicit classification Runtime 不得恢复业务 owner: {forbidden}"
        );
    }

    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/character.rs"))
        .expect("read CharacterRuntimeFacade");
    assert!(
        !facade_src.contains("classify_character_sync_behaviors_validated"),
        "CharacterRuntimeFacade 不得暴露仅供完整角色同步内部使用的 classification 旁路"
    );
    for forbidden in [
        "prepare_implicit_classification_call",
        "run_implicit_classification_call",
        "parse_implicit_classification_output",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "CharacterRuntimeFacade 不得暴露 classification 低层步骤: {forbidden}"
        );
    }

    let orchestrator_dir = manifest_dir.join("../a4n-orchestrator/src/character");
    let application_src =
        std::fs::read_to_string(orchestrator_dir.join("implicit_classification/execution.rs"))
            .expect("read Orchestrator implicit classification execution");
    let request_src =
        std::fs::read_to_string(orchestrator_dir.join("implicit_classification/request.rs"))
            .expect("read Orchestrator implicit classification request");
    let output_src =
        std::fs::read_to_string(orchestrator_dir.join("implicit_classification/output.rs"))
            .expect("read Orchestrator implicit classification output");
    let projection_src = std::fs::read_to_string(orchestrator_dir.join("sync_projection.rs"))
        .expect("read Orchestrator character sync projection");
    for required in [
        "pub(crate) async fn classify_character_sync_behaviors_validated(",
        "mark_workflow_current_step",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "stream_complete_llm(request)",
        "is_truncated_finish_reason",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator classification application 必须拥有 {required}"
        );
    }
    assert!(
        request_src.contains("PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT")
            && request_src.contains("context_aware_system_user_json_llm_request")
            && output_src.contains("parse_implicit_classification_output")
            && projection_src.contains("pub fn project_character_attribute_changes(")
            && projection_src.contains("pub fn pending_character_sync_attribute_events("),
        "Orchestrator 必须统一拥有 classification request/output/state projection"
    );
}

/// Character Profile Sync 完整流程必须由单一 Orchestrator application 拥有。
#[test]
fn character_profile_sync_stages_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/actions/character_ai/sync_profile/contracts.rs",
        "src/actions/character_ai/sync_profile/parsing.rs",
        "src/actions/character_ai/sync_profile/parsing/anchors.rs",
        "src/actions/character_ai/sync_profile/request.rs",
        "src/actions/character_ai/sync_profile/request/build.rs",
        "src/actions/character_ai/sync_profile/request/contract.rs",
        "src/actions/character_ai/sync_profile/stages.rs",
        "src/actions/character_ai/sync_profile/stages/call.rs",
        "src/actions/character_ai/sync_profile/stages/execution.rs",
        "src/actions/character_ai/sync_response.rs",
        "src/actions/character_ai/sync_response/changes.rs",
        "src/actions/character_ai/sync_response/contracts.rs",
        "src/actions/character_ai/sync_response/fields.rs",
        "src/actions/character_ai/sync_response/state.rs",
        "src/actions/character_ai/sync_validate.rs",
        "src/actions/character_ai/sync_validate/evidence.rs",
        "src/actions/character_ai/sync_validate/items.rs",
        "src/actions/character_ai/sync_llm.rs",
        "src/actions/character_ai/sync_context.rs",
        "src/actions/character_ai/sync_context/formatting.rs",
        "src/actions/character_ai/snapshot_storage.rs",
        "src/actions/character_ai/sync_implicit.rs",
        "src/actions/character_ai/sync_implicit/review.rs",
        "src/actions/character_ai/sync_implicit/review/execution.rs",
        "src/actions/character_ai/sync_write.rs",
        "src/actions/character_ai/sync_write_records.rs",
        "src/actions/character_ai/sync_profile/persistence.rs",
        "src/actions/character_ai/sync_profile/persistence/write.rs",
    ] {
        assert!(
            !manifest_dir.join(relative).exists(),
            "Character Profile Sync stage Runtime 旧 owner 必须删除: {relative}"
        );
    }

    let runtime_dir = manifest_dir.join("src/actions/character_ai/sync_profile");
    let mut runtime_source = String::new();
    for path in walk_rs_files(&runtime_dir) {
        runtime_source.push_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", path.display())),
        );
        runtime_source.push('\n');
    }
    for required in [
        ".run_character_profile_sync_validated(",
        ".prepare_character_profile_sync_preview_validated(",
    ] {
        assert!(
            runtime_source.contains(required),
            "Character Profile Sync Runtime 必须委托 stage application: {required}"
        );
    }
    for forbidden in [
        "context_aware_system_user_json_llm_request",
        "PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT",
        "chat_llm(",
        "stream_complete_llm(",
        "mark_workflow_current_step",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "build_character_sync_stage_request",
        "parse_character_sync_stage_output",
        "validate_attribute_change",
        "validate_behavior",
        "validate_deletion_candidate",
        "validate_sync_evidence_against_chapters",
        "struct CharacterSyncStageCall",
        "classify_character_sync_behaviors_validated(",
        "decide_implicit_reviews_for_sync_validated(",
        "apply_character_sync_persistence_record(",
        "sync_character_continuity_rag(",
        "begin_character_profile_sync_workflow_validated(",
        "finalize_character_profile_sync_workflow_validated(",
    ] {
        assert!(
            !runtime_source.contains(forbidden),
            "Character Profile Sync Runtime 不得恢复 stage 业务 owner: {forbidden}"
        );
    }

    let facade_src = std::fs::read_to_string(manifest_dir.join("src/state/character.rs"))
        .expect("read CharacterRuntimeFacade");
    assert!(
        facade_src
            .contains("pub(crate) async fn prepare_character_profile_sync_preview_validated(")
            && facade_src.contains("pub(crate) async fn run_character_profile_sync_validated("),
        "CharacterRuntimeFacade 必须只暴露 preview/run 两个完整角色同步 application"
    );
    for forbidden in [
        "prepare_character_profile_sync_stage_call",
        "run_character_profile_sync_stage_call",
        "parse_character_profile_sync_stage_output",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "CharacterRuntimeFacade 不得暴露 stage 低层步骤: {forbidden}"
        );
    }

    let character_dir = manifest_dir.join("../a4n-orchestrator/src/character");
    let application_src = std::fs::read_to_string(character_dir.join("profile_sync/execution.rs"))
        .expect("read Orchestrator profile sync application execution");
    let context_src = std::fs::read_to_string(character_dir.join("profile_sync/context.rs"))
        .expect("read Orchestrator profile sync context");
    let write_steps_src =
        std::fs::read_to_string(character_dir.join("profile_sync/write_steps.rs"))
            .expect("read Orchestrator profile sync write steps");
    let orchestrator_dir = character_dir.join("profile_sync_stages");
    let execution_src = std::fs::read_to_string(orchestrator_dir.join("execution.rs"))
        .expect("read Orchestrator profile sync stage execution");
    let request_src = std::fs::read_to_string(orchestrator_dir.join("request.rs"))
        .expect("read Orchestrator profile sync stage request");
    let output_src = std::fs::read_to_string(orchestrator_dir.join("output.rs"))
        .expect("read Orchestrator profile sync stage output");
    for required in [
        "pub(crate) async fn prepare_character_profile_sync_stage_preview_validated(",
        "pub(crate) async fn run_character_profile_sync_stages_validated(",
        "mark_workflow_current_step",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "stream_complete_llm(request.clone())",
        "is_truncated_finish_reason",
    ] {
        assert!(
            execution_src.contains(required),
            "Orchestrator profile sync stage application 必须拥有 {required}"
        );
    }
    for required in [
        "pub async fn prepare_character_profile_sync_preview_validated(",
        "pub async fn run_character_profile_sync_validated(",
        "classify_character_sync_behaviors_validated(",
        "decide_implicit_reviews_for_sync_validated(",
        "apply_character_sync_persistence_record(",
        "sync_character_continuity_rag(",
        "finalize_workflow_result(",
    ] {
        assert!(
            application_src.contains(required),
            "Orchestrator profile sync 完整 application 必须拥有 {required}"
        );
    }
    assert!(
        request_src.contains("PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT")
            && request_src.contains("context_aware_system_user_json_llm_request")
            && output_src.contains("parse_character_profile_sync_stage_output")
            && output_src.contains("validate_character_profile_sync_extraction")
            && context_src.contains("ensure_chapter_content_anchor_records")
            && context_src.contains("snapshot_reference_by_chapter")
            && write_steps_src.contains("build_character_sync_write_step_records"),
        "Orchestrator 必须统一拥有同步 context、stage request/output、write steps 与持久化编排"
    );
}

/// Outline 生成领域 runtime 只能使用窄 facade。
#[test]
fn outline_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/outline.rs");
    let methods = [
        "start_outline_generation_task",
        "run_outline_generation_task",
        "delete_latest_part_outline_records_validated",
        "preview_generate_parts_validated",
        "preview_regenerate_parts_validated",
        "preview_regenerate_last_part_validated",
        "preview_regenerate_part_outline_validated",
        "preview_continue_part_outlines_validated",
        "preview_generate_part_chapter_outlines_validated",
        "preview_batch_generate_part_outlines_validated",
        "preview_generate_chapter_outlines_by_count_validated",
        "preview_regenerate_chapter_outline_validated",
        "preview_apply_beat_mapping_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Outline runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src
            .contains("pub(crate) fn outline(&self) -> super::outline::OutlineRuntimeFacade<'_>"),
        "AppState 必须只向 outline 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/outline.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "OutlineRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    assert!(
        facade_src.contains("pub(crate) async fn start_outline_generation_task(")
            && facade_src.contains("pub(crate) async fn run_outline_generation_task("),
        "OutlineRuntimeFacade 必须只暴露 canonical generation start/run"
    );
    let application_path = manifest_dir.join("../a4n-orchestrator/src/outline/application.rs");
    let application_src =
        std::fs::read_to_string(application_path).expect("read outline application");
    for required in [
        "pub async fn generate_parts_validated(",
        "pub async fn batch_generate_part_outlines_validated(",
        "pub async fn regenerate_chapter_outline_validated(",
        "begin_outline_workflow_validated(",
        "advance_project_status_if_allowed(",
        "finalize_outline_application(",
    ] {
        assert!(
            application_src.contains(required),
            "Outline Orchestrator application 必须拥有 {required}"
        );
    }
    let runtime_outline_dir = manifest_dir.join("src/actions/outline_records");
    for path in walk_rs_files(&runtime_outline_dir) {
        let source = std::fs::read_to_string(&path).expect("read runtime outline source");
        assert!(
            !source.contains("begin_outline_workflow_validated(")
                && !source.contains("complete_workflow_run_validated(")
                && !source.contains("fail_workflow_run_validated(")
                && !source.contains("advance_project_status_if_allowed("),
            "Runtime Outline 不得恢复 workflow/项目状态 owner: {}",
            path.display()
        );
    }

    let repository_root = manifest_dir.join("../..");
    let server_dispatch = [
        "crates/a4n-server/src/routes/commands/dispatch/projects_and_content.rs",
        "crates/a4n-server/src/routes/commands/dispatch/system_and_collaboration.rs",
    ]
    .into_iter()
    .map(|path| std::fs::read_to_string(repository_root.join(path)).expect("read server dispatch"))
    .collect::<Vec<_>>()
    .join("\n");
    let tauri_registry =
        std::fs::read_to_string(repository_root.join("crates/a4n-tauri/src/command_registry.rs"))
            .expect("read tauri command registry");
    for legacy in [
        "generate_parts",
        "regenerate_parts",
        "regenerate_last_part",
        "continue_part_outlines",
        "regenerate_part_outline",
        "generate_part_chapter_outlines",
        "batch_generate_part_outlines",
        "generate_chapter_outlines_by_count",
        "regenerate_chapter_outline",
        "apply_beat_mapping",
    ] {
        assert!(
            !server_dispatch.contains(&format!("\"{legacy}\" =>")),
            "Server 不得恢复旧 Outline 同步生成命令: {legacy}"
        );
        assert!(
            !tauri_registry.contains(&format!("commands::outline::{legacy},")),
            "Tauri 不得恢复旧 Outline 同步生成命令: {legacy}"
        );
    }
    assert!(
        !repository_root
            .join("crates/a4n-server/src/routes/commands/import_project/outline/generation.rs")
            .exists(),
        "Server 不得恢复旧 Outline progress-task generation 模块"
    );
}

/// RAG 领域 runtime 只能使用窄 facade。
#[test]
fn rag_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/rag/facade.rs");
    let methods = [
        "attribute_events_for_rag_ingest",
        "blueprint_characters_for_rag_ingest",
        "blueprint_relationships_for_rag_ingest",
        "chapter_outlines_for_rag_ingest",
        "chapter_rag_records",
        "chapters_for_rag_ingest",
        "character_states_for_rag_ingest",
        "clear_project_rag",
        "delete_rag_records",
        "enhanced_rag_search_filter",
        "execute_prepared_enhanced_rag_search",
        "execute_prepared_enhanced_rag_search_with_vectors",
        "foreshadowing_items_for_rag_ingest",
        "novel_blueprints_for_rag_ingest",
        "part_outlines_for_rag_ingest",
        "prepare_enhanced_rag_queries",
        "project_rag_metadata",
        "protagonist_profiles_for_rag_ingest",
        "scene_occurrences_for_rag_ingest",
        "scene_profiles_for_rag_ingest",
        "scene_transitions_for_rag_ingest",
        "store_embedded_rag_records",
        "store_prepared_rag_records",
        "sync_chapter_rag_records",
        "prepare_chapter_generation_rag_queries",
        "prepare_chapter_generation_rag_search",
        "execute_prepared_chapter_generation_rag_search",
        "execute_prepared_chapter_generation_rag_search_with_vectors",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "RAG runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn rag(&self) -> super::rag::RagRuntimeFacade<'_>"),
        "AppState 必须只向 rag 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/rag/facade.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "RagRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Optimize 领域 runtime 只能使用窄 facade。
#[test]
fn optimize_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/optimize.rs");
    let methods = [
        "build_analysis_preview_request",
        "build_create_optimize_analysis_preview_request_for_user_validated",
        "build_prepare_optimize_analysis_preview_request_for_user_validated",
        "build_propose_patches_preview_request",
        "build_propose_patches_preview_request_for_user_validated",
        "build_send_user_message_preview_request",
        "build_send_user_message_preview_request_for_user_validated",
        "create_analysis_session",
        "create_optimize_analysis_session_for_user_validated",
        "create_optimize_chat_session_for_user_validated",
        "delete_optimization_session_with_messages",
        "insert_optimization_session_message",
        "matching_pending_optimization_patch_sets",
        "optimization_patch_set_patches",
        "optimization_patch_sets",
        "optimization_session_by_id",
        "optimization_sessions",
        "optimization_turn_events",
        "optimization_turns",
        "pending_optimization_patch_sets",
        "prepare_analysis_session",
        "prepare_optimize_analysis_session_validated",
        "record_command_error",
        "record_patches_applied",
        "run_analysis_session",
        "run_optimize_analysis_generation_task",
        "run_optimize_chat_generation_task",
        "run_optimize_patch_generation_task",
        "send_user_message_with_permissions",
        "start_optimize_analysis_generation_task",
        "start_optimize_chat_generation_task",
        "start_optimize_patch_generation_task",
        "start_chat_session",
        "update_optimization_patch_set_status",
        "write_optimization_chapter_patch_content",
        "begin_optimize_session_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Optimize runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn optimize(&self) -> super::optimize::OptimizeRuntimeFacade"),
        "AppState 必须只向 optimize 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/optimize.rs");
    for forbidden in [
        ".create_analysis_session(",
        ".start_chat_session(",
        ".build_analysis_preview_request(",
        ".build_send_user_message_preview_request(",
        ".build_propose_patches_preview_request(",
    ] {
        assert!(
            !facade_src.contains(forbidden),
            "Optimize facade 不得暴露低层入口: {forbidden}"
        );
    }
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "OptimizeRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Galgame 领域 runtime 只能使用窄 facade。
#[test]
fn galgame_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/galgame.rs");
    let methods = [
        "delete_galgame_line",
        "galgame_pipeline_prompt_context",
        "galgame_webgal_portraits",
        "get_script_detail",
        "insert_galgame_line",
        "plan_galgame_assets",
        "reorder_galgame_lines",
        "run_pipeline",
        "update_galgame_choices",
        "update_galgame_line",
        "update_galgame_scene",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Galgame runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn galgame(&self) -> super::galgame::GalgameRuntimeFacade"),
        "AppState 必须只向 galgame 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/galgame.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "GalgameRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    for removed in [
        "get_script_detail",
        "plan_galgame_assets",
        "run_galgame_asset_generation_validated",
        "run_galgame_pipeline_validated",
        "cancel_galgame_pipeline_validated",
        "prepare_galgame_pipeline_preview_validated",
        "run_export_galgame_webgal_validated",
        "bind_galgame_media_validated",
        "unbind_galgame_media_validated",
        "import_galgame_audio_validated",
    ] {
        assert!(
            !facade_src.contains(&format!("fn {removed}(")),
            "GalgameRuntimeFacade 不得恢复低层透传: {removed}"
        );
    }
}

/// Video 领域 runtime 只能使用窄 facade。
#[test]
fn video_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/video.rs");
    let methods = [
        "chapter_video_prompt",
        "chapter_video_prompt_for_import",
        "chapter_video_prompts_for_batch_generation",
        "chapter_video_prompts_for_chapter",
        "create_generated_image_record",
        "create_prompt_video_record_with_generation_run_validated",
        "create_video_record",
        "create_video_record_with_generation_run_validated",
        "delete_chapter_video_prompt_checked",
        "delete_chapter_video_prompts_checked",
        "delete_generated_image_record",
        "delete_generated_video_record",
        "delete_missing_generated_video_references",
        "generate_chapter_frame_prompts_for_mode",
        "generate_chapter_segment_plan",
        "generate_chapter_video_prompts_for_mode",
        "generate_chapter_video_frame_prompts_validated",
        "generate_chapter_video_segment_plan_validated",
        "generate_chapter_video_prompts_validated",
        "generated_image_by_id",
        "generated_images_by_ids",
        "generated_images_for_project",
        "generated_images_for_valid_chapter",
        "generated_images_for_video_prompt_references",
        "create_generated_image_record_with_generation_run_checkpoint",
        "update_generated_image_record",
        "toggle_generated_image_favorite",
        "video_reference_candidate_images",
        "delete_manga_page_image_records",
        "generated_video_by_id",
        "generated_videos_for_chapter",
        "mark_generated_video_completed_and_link_prompt",
        "mark_generated_video_error_status_and_prompt",
        "mark_generated_video_failed_validated",
        "mark_generated_video_interrupted_validated",
        "mark_generated_video_job_and_prompt_validated",
        "missing_video_reference_project_ids",
        "project_chapter_video_source",
        "prompt_downstream_dependencies",
        "toggle_generated_video_favorite",
        "update_chapter_video_prompt_with_policy",
        "update_generated_video_record",
        "update_generated_video_waiting_status_validated",
        "complete_generated_video_provider_runs_validated",
        "validated_video_reference_images",
        "video_asset_project_ids",
        "video_tail_frame_images",
        "begin_video_prompt_workflow_validated",
        "optimize_video_prompt_validated",
        "optimize_chapter_video_prompt_validated",
        "list_chapter_video_prompts_validated",
        "update_chapter_video_prompt_validated",
        "delete_chapter_video_prompt_validated",
        "begin_video_provider_workflow_validated",
        "begin_video_provider_progress_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Video runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn video(&self) -> super::video::VideoRuntimeFacade"),
        "AppState 必须只向 video 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/video.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "VideoRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Video Generation 的请求、记录和 provider 状态机必须由 Orchestrator application 单点拥有。
#[test]
fn video_generation_runtime_uses_application_owner() {
    let runtime_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = runtime_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace dir");
    let orchestrator_video_dir = workspace_dir.join("crates/a4n-orchestrator/src/video");
    let mut owner_src = String::new();
    for file in [
        "chapter_video_generation.rs",
        "generation.rs",
        "prompt_generation_task.rs",
        "reference_media_generation.rs",
        "provider_execution.rs",
        "records.rs",
    ] {
        owner_src.push_str(
            &std::fs::read_to_string(orchestrator_video_dir.join(file))
                .unwrap_or_else(|_| panic!("read video application owner {file}")),
        );
    }
    for required in [
        "run_video_generation_task",
        "run_video_prompt_generation_task",
        "run_chapter_video_generation_validated",
        "preview_chapter_video_request_validated",
        "run_reference_media_video_generation_validated",
        "preview_reference_media_video_request_validated",
        "apply_video_provider_result_validated",
        "poll_video_provider_once_validated",
        "create_video_tail_frame_record_validated",
    ] {
        let plain = format!("fn {required}(");
        let generic = format!("fn {required}<");
        assert!(
            owner_src.contains(&plain) || owner_src.contains(&generic),
            "Video Generation application owner 缺少入口: {required}"
        );
    }

    let runtime_root = runtime_dir.join("src/actions/video_generation");
    let mut runtime_src =
        std::fs::read_to_string(runtime_dir.join("src/actions/video_generation.rs"))
            .expect("read video generation runtime root");
    for path in walk_rs_files(&runtime_root) {
        if path.components().any(|part| part.as_os_str() == "tests")
            || path.file_name().is_some_and(|name| name == "tests.rs")
        {
            continue;
        }
        runtime_src
            .push_str(&std::fs::read_to_string(path).expect("read video generation runtime"));
        runtime_src.push('\n');
    }
    for forbidden in [
        ".create_generated_image_record(",
        ".create_video_record(",
        ".mark_generated_video_",
        ".update_generated_video_",
        "begin_video_provider_workflow_validated(",
        "begin_video_provider_progress_workflow_validated(",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Video Generation Runtime 不得恢复记录/workflow/状态 owner: {forbidden}"
        );
    }
    for required in [
        ".start_video_generation_task(",
        ".run_video_generation_task(",
        ".run_video_prompt_generation_task(",
        "start_video_generation_typed(",
        "spawn_generation_execution(",
        "spawn_video_generation_execution(",
    ] {
        assert!(
            runtime_src.contains(required),
            "Video Generation Runtime 必须委托 application owner: {required}"
        );
    }
    for forbidden in [
        ".run_chapter_video_generation_validated(",
        ".run_chapter_videos_batch_validated(",
        ".run_reference_media_video_generation_validated(",
        "job_lifecycle/polling/wait.rs",
        "chapter/execution.rs",
        "reference_media/execution.rs",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Video Generation Runtime 不得保留旧生产入口: {forbidden}"
        );
    }

    let host_src = std::fs::read_to_string(runtime_root.join("provider_host.rs"))
        .expect("read video provider host");
    for required in [
        "impl VideoProviderHostPort",
        "impl ChapterVideoGenerationHostPort",
        "impl ReferenceMediaVideoGenerationHostPort",
        "impl CanvasVideoGenerationHostPort",
        "sidecar_clients::generate_video",
        "poll_provider_job",
        "persist_video_result",
    ] {
        assert!(
            host_src.contains(required),
            "Video provider Host 必须显式承载窄 IO 边界: {required}"
        );
    }

    for legacy in [
        "chapter/execution.rs",
        "reference_media/execution.rs",
        "canvas/execution.rs",
        "job_lifecycle/workflow.rs",
        "job_lifecycle/polling/wait.rs",
        "provider_request/chapter_builder.rs",
        "provider_request/preview.rs",
        "results.rs",
        "common/lifecycle.rs",
    ] {
        assert!(
            !runtime_root.join(legacy).exists(),
            "Video Generation 旧 owner/facade 不得恢复: {legacy}"
        );
    }
}

/// Video Prompts 的生成、预览、记录变更与下游计划必须由一个 application owner 持有。
#[test]
fn video_prompts_runtime_uses_application_owner() {
    let runtime_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = runtime_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace dir");
    let owner_src = std::fs::read_to_string(
        workspace_dir.join("crates/a4n-orchestrator/src/video/prompt_application.rs"),
    )
    .expect("read video prompt application owner");
    for required in [
        "generate_chapter_video_frame_prompts_validated",
        "generate_chapter_video_segment_plan_validated",
        "generate_chapter_video_prompts_validated",
        "update_chapter_video_prompt_validated",
        "optimize_chapter_video_prompt_validated",
    ] {
        assert!(
            owner_src.contains(&format!("fn {required}(")),
            "Video Prompt application owner 缺少入口: {required}"
        );
    }

    let runtime_root = runtime_dir.join("src/actions/video_prompts");
    let mut runtime_src = std::fs::read_to_string(runtime_dir.join("src/actions/video_prompts.rs"))
        .expect("read video prompt runtime root");
    for path in walk_rs_files(&runtime_root) {
        runtime_src.push_str(&std::fs::read_to_string(path).expect("read video prompt runtime"));
        runtime_src.push('\n');
    }
    for forbidden in [
        "begin_video_prompt_workflow_validated(",
        "generate_chapter_video_prompts_for_mode(",
        "prompt_downstream_dependencies(",
        "update_chapter_video_prompt_with_policy(",
        "validate_generate_prompt_options_inner(",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Video Prompt Runtime 不得恢复 application 业务 owner: {forbidden}"
        );
    }
}

/// Video Records 业务规则与记录生命周期必须由 Orchestrator application 单点拥有。
#[test]
fn video_records_runtime_uses_application_owner() {
    let runtime_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = runtime_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace dir");
    let orchestrator_video_dir = workspace_dir.join("crates/a4n-orchestrator/src/video");
    let module_src = std::fs::read_to_string(orchestrator_video_dir.join("mod.rs"))
        .expect("read orchestrator video mod");
    for required in [
        "mod assembly_clip_rules;",
        "mod assembly_postprocess_rules;",
        "mod assembly_push_rules;",
        "mod import_rules;",
    ] {
        assert!(
            module_src.contains(required),
            "Video 纯规则模块必须实际挂载: {required}"
        );
    }

    let owner_src = std::fs::read_to_string(orchestrator_video_dir.join("records.rs"))
        .expect("read video records owner");
    for required in [
        "list_video_records_by_chapter_validated",
        "update_video_record_validated",
        "delete_video_record_validated",
        "create_assembly_push_record_validated",
        "prepare_assembly_postprocess_edit_validated",
        "prepare_assembly_postprocess_source_validated",
        "create_imported_chapter_video_record_validated",
        "prepare_canvas_frame_validated",
        "prepare_canvas_assembly_validated",
        "prepare_video_trim_validated",
    ] {
        assert!(
            owner_src.contains(&format!("fn {required}(")),
            "Video Records application owner 缺少入口: {required}"
        );
    }

    let facade_src = std::fs::read_to_string(runtime_dir.join("src/state/video.rs"))
        .expect("read video runtime facade");
    for removed in [
        "pub(crate) async fn generated_videos_for_chapter(",
        "pub(crate) async fn create_video_record(",
        "pub(crate) async fn update_generated_video_record(",
        "pub(crate) async fn toggle_generated_video_favorite(",
        "pub(crate) async fn chapter_video_prompt_for_import(",
        "pub(crate) async fn delete_missing_generated_video_references(",
    ] {
        assert!(
            !facade_src.contains(removed),
            "VideoRuntimeFacade 不得恢复无消费者的低层透传: {removed}"
        );
    }

    let runtime_root = runtime_dir.join("src/actions/video_records");
    let mut runtime_src = String::new();
    for path in walk_rs_files(&runtime_root) {
        if path.components().any(|part| part.as_os_str() == "tests") {
            continue;
        }
        runtime_src.push_str(&std::fs::read_to_string(path).expect("read video records runtime"));
        runtime_src.push('\n');
    }
    for forbidden in [
        "assembly_clip_display_shot_number(",
        "assembly_clip_insert_after_video_id(",
        "assembly_clip_sort_order(",
        "assembly_push_records(",
        "active_assembly_push_source_ids(",
        "validate_assembly_workflow_mode(",
        "validate_postprocess_operation(",
        "postprocess_edit_range_bounds(",
        "imported_chapter_video_metadata(",
        "trimmed_video_metadata(",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Video Records runtime 不得恢复 application 业务规则: {forbidden}"
        );
    }
    for required in [
        ".assembly_video_records_validated(",
        ".create_assembly_push_record_validated(",
        ".prepare_assembly_postprocess_edit_validated(",
        ".create_imported_chapter_video_record_validated(",
        ".prepare_canvas_frame_validated(",
        ".prepare_canvas_assembly_validated(",
        ".prepare_video_trim_validated(",
        "canonicalize_data_file_path(",
        "run_ffmpeg_",
        "mark_imported_video_completed(",
    ] {
        assert!(
            runtime_src.contains(required),
            "Video Records runtime 必须保留 application 委托或 Host 边界: {required}"
        );
    }

    for legacy_path in [
        "src/actions/video_records/assembly/clips.rs",
        "src/actions/video_records/assembly/clips/collection.rs",
        "src/actions/video_records/assembly/push/guards.rs",
        "src/actions/video_records/assembly/push/metadata.rs",
        "src/actions/video_records/assembly/push/selection.rs",
        "src/actions/video_records/assembly/push/workflow.rs",
        "src/actions/video_records/assembly/postprocess/metadata.rs",
        "src/actions/video_records/assembly/postprocess/range.rs",
        "src/actions/video_records/assembly/postprocess/versions.rs",
        "src/actions/video_records/imports/processing/metadata.rs",
        "src/actions/video_records/imports/source/prompt.rs",
        "src/actions/video_records/imports/source/reference_options.rs",
    ] {
        assert!(
            !runtime_dir.join(legacy_path).exists(),
            "Video Records runtime 不得保留旧 owner: {legacy_path}"
        );
    }
}

/// Canvas 领域 runtime 只能使用窄 facade。
#[test]
fn canvas_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/canvas.rs");
    let methods = [
        "import_canvas_image_validated",
        "import_canvas_video_validated",
        "get_or_create_canvas_graph_record",
        "import_video_studio_package_validated",
        "finalize_canvas_video_workflow_validated",
        "prepare_canvas_graph_resume_validated",
        "prepare_canvas_graph_run_validated",
        "prepare_canvas_single_node_run_validated",
        "prepare_canvas_llm_text_input_validated",
        "run_prepared_canvas_graph_validated",
        "run_canvas_image_node_validated",
        "run_canvas_video_node_validated",
        "run_canvas_llm_text_node_validated",
        "run_canvas_output_node_validated",
        "run_canvas_probe_node_validated",
        "run_canvas_render_text_node_validated",
        "run_canvas_editing_node_validated",
        "save_canvas_graph_record",
        "update_canvas_node_artifact_record",
        "upsert_canvas_text_asset_record",
        "video_studio_project_export_rows",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Canvas runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn canvas(&self) -> super::canvas::CanvasRuntimeFacade"),
        "AppState 必须只向 canvas 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/canvas.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "CanvasRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// Canvas graph/single-node/resume 的规划和执行循环只能由 Orchestrator application 拥有。
#[test]
fn canvas_graph_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let execution_dir = manifest_dir.join("src/actions/canvas_workflow/execution");
    let mut source = String::new();
    let mut offenders = Vec::new();
    let forbidden = [
        "runnable_canvas_node_ids",
        "topological_canvas_node_ids",
        "upstream_canvas_node_ids",
        "validate_canvas_output_target",
        "begin_canvas_graph_workflow_validated",
        "begin_canvas_single_node_workflow_validated",
        "complete_canvas_graph_workflow_validated",
        "mark_workflow_running_validated",
        "mark_workflow_current_step",
        "workflow_checkpoint_models",
        "workflow_run_model_by_id",
        "canvas_workflow_run_is_cancelled",
        "fail_workflow_run_validated",
        "get_or_create_canvas_graph_record",
    ];
    for path in [
        manifest_dir.join("src/actions/canvas_workflow/execution.rs"),
        execution_dir.join("api.rs"),
        execution_dir.join("host_port.rs"),
    ] {
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read canvas execution rs"));
        source.push_str(&text);
        source.push('\n');
        if forbidden.iter().any(|marker| text.contains(marker)) {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Canvas graph runtime 不得恢复规划/workflow/执行循环 owner:\n{}",
        offenders.join("\n")
    );
    for required in [
        ".prepare_canvas_graph_run_validated(",
        ".prepare_canvas_single_node_run_validated(",
        ".prepare_canvas_graph_resume_validated(",
        ".run_prepared_canvas_graph_validated(",
        "CanvasGraphNodeExecutionKind",
        "runtime_config::scope(",
        "tokio::spawn(",
    ] {
        assert!(
            source.contains(required),
            "Canvas graph runtime 必须保留 application/Host 适配标记: {required}"
        );
    }
    for legacy in [
        "contracts.rs",
        "graph.rs",
        "graph/kinds.rs",
        "graph/scope.rs",
        "graph/topology.rs",
        "preparation.rs",
        "preparation/context.rs",
        "preparation/graph.rs",
        "preparation/resume.rs",
        "preparation/single.rs",
        "runner.rs",
        "runner/dispatch.rs",
        "runner/graph.rs",
        "tests.rs",
    ] {
        let legacy_path = execution_dir.join(legacy);
        assert!(
            !legacy_path.is_file(),
            "Canvas graph runtime 不得保留旧 owner: {}",
            legacy_path.display()
        );
    }
}

/// Canvas 文本节点 runtime 只能保留图调度委托与动态 LLM Host port。
#[test]
fn canvas_text_generation_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest_dir.join("src/actions/canvas_workflow/text_nodes.rs"),
        manifest_dir.join("src/actions/canvas_workflow/text_nodes"),
        manifest_dir.join("src/actions/canvas_workflow/inputs/text.rs"),
        manifest_dir.join("src/actions/canvas_workflow/inputs/text"),
        manifest_dir.join("src/actions/canvas_workflow/preview/llm.rs"),
    ];
    let mut source = String::new();
    for root in roots {
        if root.is_file() {
            source.push_str(&std::fs::read_to_string(root).expect("read canvas text runtime file"));
            source.push('\n');
            continue;
        }
        for path in walk_rs_files(&root) {
            source
                .push_str(&std::fs::read_to_string(path).expect("read canvas text runtime source"));
            source.push('\n');
        }
    }
    for forbidden in [
        ".workflow()",
        "LlmContextLedger",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "build_canvas_llm_transform_request",
        "record_canvas_node_checkpoint",
        "upsert_freeform_canvas_generation_spec",
        "upsert_canvas_prompt_artifact",
        "mark_canvas_generation_spec_status",
        "is_truncated_finish_reason",
        "response_content.trim()",
    ] {
        assert!(
            !source.contains(forbidden),
            "canvas text runtime 不得恢复完整生成业务实现: {forbidden}"
        );
    }
    for required in [
        ".run_canvas_llm_text_node_validated(",
        ".run_canvas_render_text_node_validated(",
        ".prepare_canvas_llm_text_input_validated(",
    ] {
        assert!(
            source.contains(required),
            "canvas text runtime 必须委托 orchestrator application: {required}"
        );
    }
}

/// Canvas 图片节点 runtime 只能保留权限、Host port 和 application 委托。
#[test]
fn canvas_image_generation_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest_dir.join("src/actions/canvas_workflow/media_nodes/image.rs"),
        manifest_dir.join("src/actions/canvas_workflow/preview/image.rs"),
    ];
    let mut source = String::new();
    for path in roots {
        source.push_str(&std::fs::read_to_string(path).expect("read canvas image runtime file"));
        source.push('\n');
    }
    for forbidden in [
        ".workflow()",
        "build_canvas_image_input_for_run",
        "generate_canvas_image(",
        "upsert_freeform_canvas_generation_spec",
        "mark_canvas_generation_spec_status",
        "sync_completed_canvas_image_asset",
        "record_canvas_node_checkpoint",
        "finish_canvas_run_failed",
        "start_canvas_node_run",
    ] {
        assert!(
            !source.contains(forbidden),
            "canvas image runtime 不得恢复完整生成业务实现: {forbidden}"
        );
    }
    assert!(
        source.contains(".run_canvas_image_node_validated("),
        "canvas image runtime 必须委托 orchestrator application"
    );
    assert!(
        !manifest_dir
            .join("src/actions/canvas_workflow/inputs/media/image.rs")
            .exists(),
        "canvas image runtime 不得保留旧输入 owner"
    );
}

/// Canvas 抽帧、裁剪与成片节点 runtime 只能保留队列/媒体 Host port 和 application 委托。
#[test]
fn canvas_editing_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let editing_root = manifest_dir.join("src/actions/canvas_workflow/media_nodes/editing.rs");
    let editing_dir = manifest_dir.join("src/actions/canvas_workflow/media_nodes/editing");
    let mut source =
        std::fs::read_to_string(&editing_root).expect("read canvas editing runtime root");
    for path in walk_rs_files(&editing_dir) {
        source
            .push_str(&std::fs::read_to_string(path).expect("read canvas editing runtime source"));
        source.push('\n');
    }
    for forbidden in [
        "build_canvas_extract_frame_input_for_run",
        "build_canvas_trim_input_for_run",
        "build_canvas_assemble_input_for_run",
        "upsert_freeform_canvas_generation_spec",
        "mark_canvas_generation_spec_status",
        "sync_canvas_assembly_contract",
        "record_canvas_node_checkpoint",
        "finish_canvas_run_failed",
        "start_canvas_node_run",
        "finish_canvas_node_success",
        "finish_canvas_node_cancelled",
        "canvas_workflow_run_is_cancelled",
        "ensure_canvas_image_media_asset",
        "ensure_canvas_video_media_asset",
    ] {
        assert!(
            !source.contains(forbidden),
            "canvas editing runtime 不得恢复 application 业务实现: {forbidden}"
        );
    }
    for required in [
        ".run_canvas_editing_node_validated(",
        "impl CanvasEditingNodeHostPort",
        "extract_canvas_video_frame",
        "trim_video(",
        "assemble_canvas_videos",
        "queue_run_tracked_with_workflow_context",
    ] {
        assert!(
            source.contains(required),
            "canvas editing runtime 必须保留 application/Host 适配标记: {required}"
        );
    }
    for legacy_path in [
        "src/actions/canvas_workflow/media_nodes/assembly.rs",
        "src/actions/canvas_workflow/media_nodes/editing/extract.rs",
        "src/actions/canvas_workflow/media_nodes/editing/trim.rs",
        "src/actions/canvas_workflow/inputs/media/assembly.rs",
        "src/actions/canvas_workflow/inputs/media/transform.rs",
        "src/actions/canvas_workflow/node_lifecycle/task.rs",
        "src/actions/canvas_workflow/persistence/assembly.rs",
    ] {
        assert!(
            !manifest_dir.join(legacy_path).exists(),
            "canvas editing runtime 不得保留旧 owner: {legacy_path}"
        );
    }
}

/// Canvas 输出节点 runtime 只能保留节点类型分发、DTO 投影和 application 委托。
#[test]
fn canvas_output_nodes_runtime_use_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("src/actions/canvas_workflow/output_nodes");
    let mut source =
        std::fs::read_to_string(manifest_dir.join("src/actions/canvas_workflow/output_nodes.rs"))
            .expect("read canvas output runtime root");
    for name in ["text.rs", "image.rs", "audio.rs", "video.rs"] {
        source.push_str(&std::fs::read_to_string(root.join(name)).expect("read output node file"));
        source.push('\n');
    }
    for forbidden in [
        "build_canvas_output_value",
        "build_canvas_export_video_input_for_run",
        "start_canvas_node_run",
        "record_canvas_node_checkpoint",
        "finish_canvas_node_success",
        "update_canvas_node_artifact",
        ".upsert_canvas_text_asset_record(",
        ".generated_image_by_id(",
        ".generated_video_by_id(",
        ".canvas_media_asset_by_id(",
    ] {
        assert!(
            !source.contains(forbidden),
            "canvas output runtime 不得恢复 application 业务实现: {forbidden}"
        );
    }
    assert!(
        source.contains(".run_canvas_output_node_validated("),
        "canvas output runtime 必须委托 orchestrator application"
    );
    assert!(
        !manifest_dir
            .join("src/actions/canvas_workflow/inputs/output.rs")
            .exists(),
        "canvas output runtime 不得保留通用输出输入 owner"
    );
    assert!(
        !manifest_dir
            .join("src/actions/canvas_workflow/inputs/media/export.rs")
            .exists(),
        "canvas output runtime 不得保留视频输出输入 owner"
    );
}

/// Canvas 探针节点 runtime 只能保留 DTO 投影和 application 委托。
#[test]
fn canvas_probe_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(manifest_dir.join("src/actions/canvas_workflow/probe_node.rs"))
            .expect("read canvas probe runtime root");
    for forbidden in [
        "resolve_canvas_output_value_validated",
        "probe_node_data_type",
        "canvas_probe_artifact_json",
        "canvas_probe_observation_patch",
        "update_canvas_node_observation",
        "start_canvas_node_run",
        "record_canvas_node_checkpoint",
        "finish_canvas_node_success",
        ".generated_image_by_id(",
        ".generated_video_by_id(",
        ".canvas_media_asset_by_id(",
    ] {
        assert!(
            !source.contains(forbidden),
            "canvas probe runtime 不得恢复 application 业务实现: {forbidden}"
        );
    }
    assert!(
        source.contains(".run_canvas_probe_node_validated("),
        "canvas probe runtime 必须委托 orchestrator application"
    );
    for legacy_path in [
        "src/actions/canvas_workflow/probe_node/artifacts.rs",
        "src/actions/canvas_workflow/probe_node/contracts.rs",
        "src/actions/canvas_workflow/probe_node/execution.rs",
        "src/actions/canvas_workflow/persistence/nodes.rs",
    ] {
        assert!(
            !manifest_dir.join(legacy_path).exists(),
            "canvas probe runtime 不得保留旧 owner: {legacy_path}"
        );
    }
}

/// Source Sync 领域 runtime 只能使用窄 facade。
#[test]
fn source_sync_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/source_sync.rs");
    let methods = [
        "run_source_sync_apply_for_user_validated",
        "preview_source_sync_for_user_validated",
        "source_sync_preview_context_for_user_validated",
        "source_sync_run_for_user_validated",
        "detach_derived_project_for_user_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Source Sync runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn source_sync(&self) -> super::source_sync::SourceSyncRuntimeFacade"
        ),
        "AppState 必须只向 source_sync 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/source_sync.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "SourceSyncRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    for required in [
        "run_source_sync_apply_for_user_validated(",
        "preview_source_sync_for_user_validated(",
        "source_sync_run_for_user_validated(",
        "detach_derived_project_for_user_validated(",
    ] {
        assert!(
            facade_src.contains(required),
            "SourceSyncRuntimeFacade 缺少当前窄入口: {required}"
        );
    }

    let actions_root = manifest_dir.join("src/actions/source_sync");
    let mut actions_src = String::new();
    for path in walk_rs_files(&actions_root) {
        actions_src.push_str(&std::fs::read_to_string(path).expect("read source sync action"));
        actions_src.push('\n');
    }
    for forbidden in [
        "a4n_db::source_sync",
        "apply_prepared_source_sync_decisions_records_transaction",
        "begin_source_sync_apply_workflow_validated(",
        "finalize_workflow_run_outcome_validated(",
        ".orchestrator()",
    ] {
        assert!(
            !actions_src.contains(forbidden),
            "Source Sync Runtime 不得恢复决策/DB/workflow owner: {forbidden}"
        );
    }
    assert!(
        actions_src.contains(".run_source_sync_apply_for_user_validated("),
        "Source Sync Runtime apply 必须委托完整 application"
    );

    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/source_sync/service.rs"),
    )
    .expect("read Orchestrator source sync application");
    for owner in [
        "run_source_sync_apply_validated(",
        "begin_source_sync_apply_workflow_validated(",
        "apply_source_sync_decisions_with_context(",
        "prepared_source_sync_decisions(",
    ] {
        assert!(
            application_src.contains(owner),
            "Orchestrator 必须拥有 Source Sync application: {owner}"
        );
    }
}

/// 生成运行账本低层 facade 已删除，runtime 不得恢复旁路。
#[test]
fn generation_runtime_has_no_low_level_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/generation.rs");
    let methods = [
        "attach_generation_runs_to_workflow_for_target",
        "complete_generation_runs_for_target",
        "fail_generation_runs_for_target",
        "record_completed_generation_run_with_checkpoint",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "generation runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        !facade_path.exists() && !app_src.contains("fn generation("),
        "无消费者的 GenerationRuntimeFacade 不得恢复"
    );
}

/// 灵感对话 runtime 只能使用窄 facade。
#[test]
fn inspiration_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/inspiration.rs");
    let methods = [
        "start_inspiration_generation_task",
        "run_inspiration_generation_task",
        "inspiration_messages",
        "inspiration_project_context_rows",
        "begin_inspiration_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "inspiration runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn inspiration(&self) -> super::inspiration::InspirationRuntimeFacade"
        ),
        "AppState 必须只向 inspiration 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/inspiration.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "InspirationRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 资料素材 runtime 只能使用窄 facade。
#[test]
fn profile_material_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/profile_material.rs");
    let methods = [
        "list_profile_materials_validated",
        "set_profile_primary_material_validated",
        "upload_profile_material_image_validated",
        "count_profile_material_images",
        "count_profile_material_images_by_profile",
        "profile_material_image_ids",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "profile_material runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn profile_material(")
            && app_src.contains("super::profile_material::ProfileMaterialRuntimeFacade"),
        "AppState 必须只向 profile_material 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/profile_material.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ProfileMaterialRuntimeFacade 不得泄露完整 OrchestratorContext"
    );

    let actions_root = manifest_dir.join("src/actions/profile_materials");
    for (path, symbol) in [
        ("list.rs", "list_profile_materials_validated"),
        ("primary.rs", "set_profile_primary_material_validated"),
        ("upload/api.rs", "upload_profile_material_image_validated"),
    ] {
        let source = std::fs::read_to_string(actions_root.join(path))
            .unwrap_or_else(|_| panic!("read profile material {path}"));
        assert!(
            source.contains(symbol),
            "profile material Runtime wrapper 必须委托 application: {symbol}"
        );
    }
    for legacy in ["list", "prompts.rs", "tests.rs"] {
        assert!(
            !actions_root.join(legacy).exists(),
            "profile material Runtime 旧业务 owner 不得恢复: {legacy}"
        );
    }
    let application_src =
        std::fs::read_to_string(manifest_dir.join("../a4n-orchestrator/src/profile_material.rs"))
            .expect("read Orchestrator profile material application");
    for symbol in [
        "profile_material_groups",
        "set_profile_primary_material_validated",
        "upload_profile_material_image_validated",
        "ProfileMaterialUploadHostPort",
    ] {
        assert!(
            application_src.contains(symbol),
            "Orchestrator 必须拥有 profile material application: {symbol}"
        );
    }
}

/// 导入反推 runtime 只能使用窄 facade。
#[test]
fn import_reverse_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/import_reverse.rs");
    let methods = [
        "apply_reverse_blueprint_data",
        "apply_reverse_outline_data",
        "apply_reverse_part_outline_data",
        "reverse_context_rows",
        "reverse_engineer_request",
        "begin_import_reverse_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "import_reverse runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn import_reverse(&self) -> super::import_reverse::ImportReverseRuntimeFacade"),
        "AppState 必须只向 import_reverse 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/import_reverse.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ImportReverseRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 资产谱系同步 runtime 只能使用窄 facade。
#[test]
fn asset_lineage_sync_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/asset_lineage_sync.rs");
    let methods = ["sync_project_asset_lineage"];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "asset_lineage_sync runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn asset_lineage_sync(")
            && app_src.contains("super::asset_lineage_sync::AssetLineageSyncRuntimeFacade"),
        "AppState 必须只向 asset_lineage_sync 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/asset_lineage_sync.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "AssetLineageSyncRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 章节结构化分析 runtime 只能使用窄 facade。
#[test]
fn chapter_analysis_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/chapter_analysis.rs");
    let methods = [
        "analyze_and_store_chapter",
        "build_chapter_analysis_requests",
        "sync_chapter_analysis_foreshadowing_outputs",
        "begin_chapter_analysis_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "chapter_analysis runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn chapter_analysis(")
            && app_src.contains("super::chapter_analysis::ChapterAnalysisRuntimeFacade"),
        "AppState 必须只向 chapter_analysis 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/chapter_analysis.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ChapterAnalysisRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 导入分析 runtime 只能使用窄 facade，不得注入完整 orchestrator。
#[test]
fn import_analysis_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/import_analysis.rs");
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src/actions/import_analysis")) {
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read import_analysis rs"));
        if text.contains(".orchestrator()") || text.contains(".orchestrator,") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "import_analysis actions 不得恢复完整 orchestrator 注入:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn import_analysis(")
            && app_src.contains("super::import_analysis::ImportAnalysisRuntimeFacade"),
        "AppState 必须只向 import_analysis 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/import_analysis.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ImportAnalysisRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 章节生成 runtime 只能使用窄 facade。
#[test]
fn chapter_generation_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/chapter_generation.rs");
    let methods = [
        "start_chapter_generation_task",
        "run_chapter_generation_task",
        "preview_chapter_generation_request_validated",
        "prepare_chapter_generation_rag_embedding_validated",
        "finish_chapter_generation_rag_context_validated",
        "active_generation_chapter",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "chapter_generation runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn chapter_generation(")
            && app_src.contains("super::chapter_generation::ChapterGenerationRuntimeFacade"),
        "AppState 必须只向 chapter_generation 调用提供窄 facade"
    );
    let facade_src =
        std::fs::read_to_string(&facade_path).expect("read state/chapter_generation.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ChapterGenerationRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 章节生成准备、RAG、请求、工作流与后处理必须由 Orchestrator application 统一编排。
#[test]
fn chapter_generation_runtime_uses_application_owner() {
    let runtime_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = runtime_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace dir");
    let owner_src = std::fs::read_to_string(
        workspace_dir.join("crates/a4n-orchestrator/src/chapter_generation/application.rs"),
    )
    .expect("read chapter generation application owner");
    for required in [
        "run_chapter_generation_task",
        "preview_chapter_generation_request_validated",
        "prepare_chapter_generation_rag_embedding_validated",
        "finish_chapter_generation_rag_context_validated",
    ] {
        assert!(
            owner_src.contains(&format!("fn {required}(")),
            "Chapter Generation application owner 缺少入口: {required}"
        );
    }

    let runtime_src =
        std::fs::read_to_string(runtime_dir.join("src/actions/chapter_generation.rs"))
            .expect("read chapter generation runtime");
    for required in [
        "start_chapter_generation_typed(",
        "run_chapter_generation_task(",
        "StartChapterGenerationInput",
    ] {
        assert!(
            runtime_src.contains(required),
            "Chapter Generation Runtime 缺少统一 generation 入口: {required}"
        );
    }
}

/// Prompt Records 业务只能由 Orchestrator application 拥有。
#[test]
fn prompt_records_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/prompt_records.rs");
    let actions_path = manifest_dir.join("src/actions/prompt_records.rs");
    let application_path = manifest_dir.join("../a4n-orchestrator/src/prompt_records.rs");
    let methods = [
        "list_prompt_records_validated",
        "load_default_prompt_records_validated",
        "update_prompt_record_validated",
        "reset_prompt_record_validated",
        "render_prompt_record_validated",
        "render_prompt_record_ab_validated",
        "set_prompt_project_override_validated",
        "remove_prompt_project_override_validated",
        "list_prompt_project_overrides_validated",
        "prompt_dependency_graph_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "prompt_records runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn prompt_records(")
            && app_src.contains("super::prompt_records::PromptRecordsRuntimeFacade"),
        "AppState 必须只向 prompt_records 调用提供高层 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read prompt_records facade");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "PromptRecordsRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    let actions_src = std::fs::read_to_string(&actions_path).expect("read prompt_records actions");
    assert!(
        actions_src.contains("state.prompt_records()")
            && !actions_src.contains("PromptClient")
            && !actions_src.contains("prompt_service_request")
            && !actions_src.contains("project_override_summaries"),
        "Prompt Records actions 必须只保留 Host API/错误映射"
    );
    let application_src =
        std::fs::read_to_string(&application_path).expect("read prompt_records application");
    for required in methods {
        assert!(
            application_src.contains(required),
            "Prompt Records application 缺少 {required}"
        );
    }
    for removed in [
        "src/state/prompt_override.rs",
        "src/state/prompt_engine/facade.rs",
        "src/actions/prompt_records/client.rs",
        "src/actions/prompt_records/contracts.rs",
        "src/actions/prompt_records/overrides/mod.rs",
        "src/actions/prompt_records/queries.rs",
        "src/actions/prompt_records/render.rs",
        "src/actions/prompt_records/templates.rs",
    ] {
        assert!(
            !manifest_dir.join(removed).exists(),
            "Prompt Records 旧 owner 不得恢复：{removed}"
        );
    }
}

/// LLM 聊天/结构化修复已归 application owner，Runtime 不保留无消费者 facade。
#[test]
fn llm_runtime_has_no_unused_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/llm.rs");
    assert!(!facade_path.exists(), "Runtime 不得恢复无消费者 LLM facade");
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        !app_src.contains("fn llm(&self)"),
        "AppState 不得恢复无消费者 LLM 入口"
    );
}

/// 动态 LLM / Embedding provider 只能持有 DynamicProviderFacade，不得再持有完整 OrchestratorContext。
#[test]
fn dynamic_providers_use_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest_dir.join("src/dynamic_llm"),
        manifest_dir.join("src/dynamic_embedding.rs"),
        manifest_dir.join("src/sidecar_clients/llm.rs"),
        manifest_dir.join("src/sidecar_clients/embedding"),
    ];
    let mut offenders = Vec::new();
    for root in roots {
        if root.is_file() {
            let text = std::fs::read_to_string(&root).expect("read dynamic provider rs");
            if text.contains("OrchestratorContext") {
                offenders.push(root.display().to_string());
            }
            continue;
        }
        for path in walk_rs_files(&root) {
            let text = std::fs::read_to_string(&path).expect("read dynamic provider rs");
            if text.contains("OrchestratorContext") {
                offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "dynamic_llm / dynamic_embedding / sidecar provider 不得再持有完整 OrchestratorContext:\n{}",
        offenders.join("\n")
    );

    let mut field_name_offenders = Vec::new();
    for root in [
        manifest_dir.join("src/dynamic_llm"),
        manifest_dir.join("src/dynamic_embedding.rs"),
        manifest_dir.join("src/sidecar_clients/llm.rs"),
        manifest_dir.join("src/sidecar_clients/embedding"),
    ] {
        let paths = if root.is_file() {
            vec![root.clone()]
        } else {
            walk_rs_files(&root)
        };
        for path in paths {
            let text = std::fs::read_to_string(&path).expect("read dynamic provider rs");
            if text.contains("orchestrator: DynamicProviderFacade")
                || text.contains("orchestrator: &DynamicProviderFacade")
                || text.contains(".orchestrator.")
            {
                field_name_offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        field_name_offenders.is_empty(),
        "DynamicProviderFacade 字段/参数名必须为 facade，不得继续叫 orchestrator:\n{}",
        field_name_offenders.join("\n")
    );

    let app_src = std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs");
    assert!(
        app_src.contains("dynamic_provider_facade()")
            && !app_src
                .contains("DynamicLlmProvider::new(\n            self.orchestrator().clone()")
            && !collapse_ws(&app_src)
                .contains("DynamicLlmProvider::new(self.orchestrator().clone())")
            && !collapse_ws(&app_src).contains(
                "DynamicLlmProvider::new_with_sidecar_client(self.orchestrator().clone()"
            ),
        "AppState 装配 dynamic provider 必须使用 dynamic_provider_facade，不得 clone 完整上下文"
    );

    let context_src = std::fs::read_to_string(orchestrator_context_rs_path())
        .expect("read orchestrator context.rs");
    assert!(
        context_src.contains("mod dynamic_provider;"),
        "OrchestratorContext 必须提供 dynamic_provider 窄 facade 模块"
    );
}

/// 基础设施用途方法不得再经完整 orchestrator 入口回流。
#[test]
fn infrastructure_purpose_methods_use_appstate_entrypoints() {
    let methods = [
        "capabilities",
        "log_dir",
        "temp_dir",
        "app_paths",
        "default_fastembed_cache_dir",
        "configured_database_url",
        "required_jwt_secret",
        "register_task_handle",
        "remove_task_handle",
        "cancel_task_handle",
        "active_task_snapshots",
    ];
    let mut offenders = Vec::new();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path.ends_with("state/app.rs") {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "基础设施用途方法不得经 orchestrator 回流:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    for method in methods {
        assert!(
            app_src.contains(&format!("fn {method}(")),
            "AppState 必须提供用途方法 {method}"
        );
    }
}

/// 任务队列 runtime 只能使用窄 facade（dynamic_llm 持有 OrchestratorContext 直调除外）。
#[test]
fn task_queue_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/task_queue.rs");
    let methods = [
        "queue_run_tracked",
        "queue_run_tracked_with_cancel",
        "queue_run_tracked_with_family_context",
        "queue_run_tracked_with_workflow_context",
        "queue_submit",
        "queue_submit_with_workflow_context",
        "queue_submitted_task_guard",
        "queue_acquire_task_with_cancel",
        "queue_cancel",
        "queue_mark_done",
        "queue_mark_failed",
        "queue_stats",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        // dynamic_llm 持有 OrchestratorContext，允许直调 queue_*。
        if path
            .strip_prefix(manifest_dir.join("src"))
            .map(|p| p.starts_with("dynamic_llm"))
            .unwrap_or(false)
        {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "task_queue runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn task_queue(&self) -> super::task_queue::TaskQueueRuntimeFacade"
        ),
        "AppState 必须只向 task_queue 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/task_queue.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "TaskQueueRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 媒体资产 runtime 只能使用窄 facade。
#[test]
fn media_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/media.rs");
    let methods = [
        "list_generated_images_for_user_validated",
        "list_chapter_images_for_user_validated",
        "list_video_reference_images_for_user_validated",
        "prepare_imported_image_upload_for_user_validated",
        "create_imported_image_record_for_user_validated",
        "update_generated_image_for_user_validated",
        "toggle_generated_image_favorite_for_user_validated",
        "delete_generated_image_for_user_validated",
        "prepare_scene_image_request_validated",
        "generate_scene_image_validated",
        "prepare_canvas_image_request_validated",
        "generate_galgame_image_validated",
        "generate_object_profile_image_prompt_validated",
        "prepare_object_profile_image_request_validated",
        "generate_object_profile_image_validated",
        "generate_scene_profile_image_prompt_validated",
        "generate_scene_profile_image_validated",
        "prepare_page_image_request_validated",
        "generate_page_image_validated",
        "prepare_batch_images_request_validated",
        "generate_batch_images_validated",
        "delete_generated_image_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "media runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn media(&self) -> super::media::MediaRuntimeFacade"),
        "AppState 必须只向 media 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/media.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "MediaRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

/// 图片记录的权限、筛选、记录写入和更新必须由 Orchestrator media owner 拥有。
#[test]
fn image_records_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_root = manifest_dir.join("src/actions/image_records");
    let mut runtime_src = String::new();
    for path in walk_rs_files(&runtime_root) {
        runtime_src
            .push_str(&std::fs::read_to_string(&path).expect("read image records runtime source"));
        runtime_src.push('\n');
    }
    for required in [
        ".list_generated_images_for_user_validated(",
        ".list_chapter_images_for_user_validated(",
        ".list_video_reference_images_for_user_validated(",
        ".prepare_imported_image_upload_for_user_validated(",
        ".create_imported_image_record_for_user_validated(",
        ".update_generated_image_for_user_validated(",
        ".toggle_generated_image_favorite_for_user_validated(",
        ".delete_generated_image_for_user_validated(",
    ] {
        assert!(
            runtime_src.contains(required),
            "Image Records Runtime 必须委托 media application: {required}"
        );
    }
    for forbidden in [
        "access::verify_project",
        ".video().generated_images_for_project(",
        ".video().generated_images_for_valid_chapter(",
        ".video().video_reference_candidate_images(",
        ".video().update_generated_image_record(",
        ".video().toggle_generated_image_favorite(",
        "fn imported_image_metadata(",
        "decode_uploaded_image_data(",
        "HashMap",
    ] {
        assert!(
            !runtime_src.contains(forbidden),
            "Image Records Runtime 不得恢复业务 owner: {forbidden}"
        );
    }
    let owner_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/media/image_record_lifecycle.rs"),
    )
    .expect("read image record lifecycle owner");
    for required in [
        "list_generated_images_for_user_validated",
        "list_video_reference_images_for_user_validated",
        "prepare_imported_image_upload_for_user_validated",
        "create_imported_image_record_for_user_validated",
        "update_generated_image_for_user_validated",
        "delete_generated_image_for_user_validated",
    ] {
        assert!(
            owner_src.contains(&format!("fn {required}(")),
            "Image Records application owner 缺少入口: {required}"
        );
    }
    assert!(
        !runtime_root.join("upload/metadata.rs").exists(),
        "导入图片 metadata 旧 owner 不得恢复"
    );
}

/// scene/page/direct/batch item 的单图执行必须由 Orchestrator application 唯一拥有。
#[test]
fn image_generation_single_item_runtime_uses_application_owner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let image_root = manifest_dir.join("src/actions/image_generation");
    let wrappers = [
        (
            "direct.rs",
            &[
                "prepare_scene_image_request_validated",
                "prepare_canvas_image_request_validated",
                "generate_galgame_image_validated",
            ] as &[&str],
        ),
        (
            "page.rs",
            &["prepare_page_image_request_validated"] as &[&str],
        ),
        (
            "batch_api.rs",
            &["prepare_batch_images_request_validated"] as &[&str],
        ),
        (
            "profiles.rs",
            &[
                "generate_object_profile_image_prompt_validated",
                "prepare_object_profile_image_request_validated",
                "generate_scene_profile_image_prompt_validated",
            ] as &[&str],
        ),
    ];
    let forbidden = [
        "save_generated_image_result",
        "run_image_generation_provider_request_with_workflow",
        "load_completed_image_checkpoint",
        "save_completed_image_checkpoint",
        "save_completed_batch_image_checkpoint",
        "create_generated_image_record_with_generation_run_checkpoint",
        "load_completed_workflow_checkpoint",
        "save_completed_workflow_checkpoint",
        "sidecar_clients::generate_image",
        "persist_generated_image_file",
    ];
    let mut offenders = Vec::new();
    for (relative_path, required) in wrappers {
        let path = image_root.join(relative_path);
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", path.display()));
        for symbol in required {
            assert!(
                text.contains(symbol),
                "{} 必须委托 Orchestrator application: {symbol}",
                path.display()
            );
        }
        for symbol in &forbidden {
            if text.contains(symbol) {
                offenders.push(format!("{}: {symbol}", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "Image generation runtime 不得恢复单图 workflow/checkpoint/provider/persistence owner:\n{}",
        offenders.join("\n")
    );

    for legacy in [
        "application.rs",
        "batch.rs",
        "batch",
        "request.rs",
        "request",
        "source_binding.rs",
        "source_binding",
        "metadata.rs",
        "video_frame_context.rs",
        "scene/execution.rs",
        "page/generation.rs",
        "page/preparation.rs",
        "generation_io/contracts.rs",
        "generation_io/metadata.rs",
        "generation_io/persistence.rs",
        "generation_io/provider.rs",
        "workflow/checkpoints.rs",
        "page/completion.rs",
    ] {
        assert!(
            !image_root.join(legacy).exists(),
            "Image generation 旧 owner 文件不得恢复: {legacy}"
        );
    }

    let host_src =
        std::fs::read_to_string(image_root.join("host_port.rs")).expect("read host_port.rs");
    for symbol in [
        "sidecar_clients::generate_image",
        "persist_generated_image_file",
        "read_reference_image",
        "resolve_image_config",
    ] {
        assert!(
            host_src.contains(symbol),
            "Image generation Host port 必须显式承载 Host 边界: {symbol}"
        );
    }

    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/media/image_generation/application.rs"),
    )
    .expect("read Orchestrator image generation application.rs");
    for symbol in [
        "resolve_image_chapter_binding",
        "build_batch_image_jobs",
        "prepare_page_image",
        "object_profile_image_material",
        "scene_profile_image_material",
    ] {
        assert!(
            application_src.contains(symbol),
            "Orchestrator application 必须拥有图片准备规则: {symbol}"
        );
    }
}

/// 拍摄剧本领域 runtime 只能使用窄 facade。
#[test]
fn shooting_script_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/shooting_script.rs");
    let methods = [
        "current_chapter_shooting_script",
        "generate_chapter_shooting_script_with_progress",
        "edit_current_chapter_shooting_script_validated",
        "require_current_chapter_shooting_script",
        "edit_chapter_shooting_script",
        "next_chapter_shooting_script_revision",
        "begin_chapter_shooting_script_workflow_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "拍摄剧本 runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn shooting_script(")
            && app_src.contains("super::shooting_script::ShootingScriptRuntimeFacade"),
        "AppState 必须只向 shooting_script 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/shooting_script.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "ShootingScriptRuntimeFacade 不得泄露完整 OrchestratorContext"
    );

    let edit_src =
        std::fs::read_to_string(manifest_dir.join("src/actions/shooting_script_records/edit.rs"))
            .expect("read shooting script edit action");
    assert!(
        edit_src.contains(".edit_current_chapter_shooting_script_for_user_validated("),
        "拍摄剧本编辑必须委托当前来源校验 application"
    );
    for forbidden in [
        ".current_chapter_shooting_script(",
        ".edit_chapter_shooting_script(",
        "existing.record.id",
    ] {
        assert!(
            !edit_src.contains(forbidden),
            "拍摄剧本编辑 Runtime 不得恢复来源校验旁路: {forbidden}"
        );
    }
    let application_src = std::fs::read_to_string(
        manifest_dir.join("../a4n-orchestrator/src/context/shooting_script.rs"),
    )
    .expect("read Orchestrator shooting script application");
    assert!(
        application_src.contains("pub async fn edit_current_chapter_shooting_script_validated(")
            && !application_src.contains("pub async fn edit_chapter_shooting_script("),
        "Orchestrator 必须只公开带当前来源校验的拍摄剧本编辑入口"
    );
}

/// Manga 领域 runtime 只能使用窄 facade。
#[test]
fn manga_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/manga.rs");
    let methods = [
        "bind_manga_reference_image_validated",
        "cancel_manga_pipeline_validated",
        "delete_manga_by_chapter_validated",
        "delete_manga_prompt_validated",
        "get_manga_pipeline_status_validated",
        "list_manga_panels_validated",
        "mark_manga_page_prompt_image_completed_with_image",
        "preview_generate_panels_requests",
        "optimize_manga_prompt_record_validated",
        "update_manga_prompt_validated",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Manga runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains("pub(crate) fn manga(&self) -> super::manga::MangaRuntimeFacade<'_>"),
        "AppState 必须只向 manga 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/manga.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "MangaRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
    let optimization_dir = manifest_dir.join("src/actions/manga_records/optimization");
    let forbidden = [
        ".workflow()",
        ".llm()",
        "render_prompt_template",
        "context_aware_system_user_request",
        "context_aware_chat_request_trace_json",
        "load_completed_llm_call_checkpoint",
        "save_completed_llm_call_checkpoint",
        "parse_validate_or_repair_structured_value",
        "begin_manga_prompt_optimization_workflow_validated",
        "manga_prompt_optimization_checkpoint_output",
    ];
    let mut owner_offenders = Vec::new();
    let mut optimization_src = String::new();
    for path in walk_rs_files(&optimization_dir) {
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read optimization rs"));
        optimization_src.push_str(&text);
        if forbidden.iter().any(|marker| text.contains(marker)) {
            owner_offenders.push(path.display().to_string());
        }
    }
    assert!(
        owner_offenders.is_empty(),
        "Manga optimization runtime 不得恢复 request/workflow/checkpoint/LLM owner:\n{}",
        owner_offenders.join("\n")
    );
    assert!(
        optimization_src.contains(".optimize_manga_prompt_record_validated("),
        "Manga optimization runtime 必须委托完整 orchestrator application"
    );
}

/// Workflow 领域 runtime 只能使用窄 facade。
#[test]
fn workflow_runtime_uses_narrow_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade_path = manifest_dir.join("src/state/workflow.rs");
    let methods = [
        "cancel_workflow_run_and_interrupt_videos",
        "claim_exclusive_workflow_run",
        "latest_workflow_run_by_business_key",
        "load_completed_llm_call_checkpoint",
        "load_completed_workflow_checkpoint",
        "mark_workflow_current_step",
        "mark_workflow_status",
        "persisted_task_by_id",
        "persisted_task_history",
        "record_workflow_progress_event",
        "save_completed_llm_call_checkpoint",
        "save_completed_workflow_checkpoint",
        "upsert_workflow_checkpoint",
        "upsert_workflow_run",
        "workflow_checkpoint_models",
        "workflow_progress_events",
        "workflow_run_detail",
        "workflow_run_model_by_id",
        "workflow_run_model_optional_by_id",
        "workflow_run_models",
        "workflow_run_summary",
        "workflow_runs",
        "workflow_runs_by_project_kind",
    ];
    let mut offenders = Vec::new();
    for path in walk_rs_files(&manifest_dir.join("src")) {
        if path == facade_path {
            continue;
        }
        let text = collapse_ws(&std::fs::read_to_string(&path).expect("read runtime rs"));
        if methods
            .iter()
            .any(|method| text.contains(&format!(".orchestrator().{method}(")))
        {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "Workflow runtime 不得恢复完整 orchestrator 入口:\n{}",
        offenders.join("\n")
    );
    let app_src =
        collapse_ws(&std::fs::read_to_string(app_state_rs_path()).expect("read state/app.rs"));
    assert!(
        app_src.contains(
            "pub(crate) fn workflow(&self) -> super::workflow::WorkflowRuntimeFacade<'_>"
        ),
        "AppState 必须只向 workflow 调用提供窄 facade"
    );
    let facade_src = std::fs::read_to_string(&facade_path).expect("read state/workflow.rs");
    assert!(
        !facade_src.contains("fn orchestrator(")
            && !facade_src.contains("fn context(")
            && !facade_src.contains("pub orchestrator:"),
        "WorkflowRuntimeFacade 不得泄露完整 OrchestratorContext"
    );
}

const FORBIDDEN_SERVICE_FIELDS: &[&str] = &[
    "project",
    "object_profile",
    // scene_profile 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    "blueprint",
    // canvas 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    // outline 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    // portrait 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    // manga 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    "image",
    // video 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    // text_anchor 已收敛为 AppState 窄 facade 方法，不再是旁路字段
    "media_asset",
    // source_sync 已收敛为 AppState 窄 facade 方法，不再是旁路字段
];

const PRIVATE_CONTEXT_FIELDS: &[&str] = &[
    "admin",
    "auth",
    "collaboration",
    "project_read",
    "project_mutation",
    "project_outline_records",
    "project_scene_records",
    "project_family",
    "project_blueprint",
    "project_chapter_context",
    "project_chapter_records",
    "project_dependency_graph",
    "project_duplication",
    "project_status",
    "settings",
    "import_review",
    "export",
    "asset",
    "scene_profile",
    "chapter_content",
    "character",
    "outline",
    "galgame",
    "manga",
    "optimize",
    "rag",
    "workflow",
    "video",
    "canvas",
    "source_sync",
    "shooting_script",
    "prompt_override",
    "generation",
    "inspiration",
    "profile_material",
    "text_anchor",
    "import_reverse",
    "llm_context",
    "asset_lineage_sync",
    "config",
    "event_bus",
    "task_manager",
    "chapter",
    "rag_store",
    "task_queue",
    "llm",
    "engine",
    "embedding",
    "snapshot_store",
];

fn app_state_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/state/app.rs")
}

fn orchestrator_context_rs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../a4n-orchestrator/src/context.rs")
}

fn orchestrator_context_owner_sources() -> String {
    let context_path = orchestrator_context_rs_path();
    let mut source = std::fs::read_to_string(&context_path).expect("read orchestrator context.rs");
    let context_dir = context_path.with_extension("");
    for path in walk_rs_files(&context_dir) {
        source.push('\n');
        source.push_str(&std::fs::read_to_string(path).expect("read orchestrator context module"));
    }
    source
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
    out.sort();
    out
}
