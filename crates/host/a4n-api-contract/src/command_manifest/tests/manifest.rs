#[test]
fn command_manifest_is_sorted_unique_and_self_consistent() {
    assert!(API_COMMANDS.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        API_COMMANDS.len(),
        API_COMMANDS.iter().copied().collect::<HashSet<_>>().len()
    );
    assert_eq!(
        RUNTIME_EQUIVALENTS.len(),
        RUNTIME_EQUIVALENTS
            .iter()
            .map(|(command, _)| *command)
            .collect::<HashSet<_>>()
            .len()
    );
    assert!(
        GENERATION_START_ROUTES
            .iter()
            .all(|(command, _)| API_COMMANDS.binary_search(command).is_ok())
    );

    for (name, spec) in API_COMMANDS.iter().zip(COMMAND_SPECS) {
        assert_eq!(*name, spec.name);
        let descriptor = command_descriptor(name).expect("declared command");
        assert_eq!(descriptor.name, *name);
        let context_fields = descriptor.args_schema["x-a4n-context-fields"]
            .as_array()
            .expect("context fields");
        assert!(
            context_fields
                .iter()
                .all(|field| field.as_str() != Some("userId")),
            "{name} must derive user identity from authentication"
        );
    }
}
#[test]
fn security_and_lifecycle_metadata_remain_explicit() {
    let provider = command_descriptor("create_llm_config").unwrap();
    assert_eq!(provider.auth, "admin");
    assert!(
        provider
            .secret_pointers
            .contains(&"/input/api_key_encrypted".to_string())
    );

    let generation = command_descriptor("start_chapter_generation").unwrap();
    assert_eq!(generation.execution, "task");
    assert_eq!(generation.idempotency, "client_request_id");
    assert_eq!(generation.runtime_config_policy, RuntimeConfigPolicy::Full);
    assert!(
        generation
            .secret_pointers
            .contains(&"/runtime_config_snapshot/llm/api_key_encrypted".to_string())
    );
    let web_search = command_descriptor("web_search").unwrap();
    assert_eq!(web_search.runtime_config_policy, RuntimeConfigPolicy::Full);
    assert!(
        web_search
            .secret_pointers
            .contains(&"/runtime_config_snapshot/web_search/api_key_encrypted".to_string())
    );
    let read_image = command_descriptor("read_image").unwrap();
    assert_eq!(read_image.runtime_config_policy, RuntimeConfigPolicy::Full);
    assert!(read_image.agent_policy.is_some());
    let web_fetch = command_descriptor("web_fetch").unwrap();
    assert_eq!(
        web_fetch.runtime_config_policy,
        RuntimeConfigPolicy::WithoutSecrets
    );
    assert!(web_fetch.secret_pointers.is_empty());
    assert_eq!(
        command_runtime_config_policy("preview_chapter_prompt"),
        RuntimeConfigPolicy::WithoutSecrets
    );
    assert_eq!(
        command_runtime_config_policy("preview_source_sync"),
        RuntimeConfigPolicy::None
    );

    assert!(is_destructive("delete_project"));
    assert!(!is_destructive("get_project"));
    assert!(is_destructive("delete_future_resource"));
}

#[test]
fn character_ai_commands_use_required_task_typed_input_contract() {
    for (command_name, preview_name) in [
        (
            "discover_character_profiles",
            "preview_discover_character_profiles_request",
        ),
        (
            "check_character_conflict",
            "preview_check_character_conflict_request",
        ),
    ] {
        let command = command_descriptor(command_name).unwrap();
        assert_eq!(command.execution, "task", "{command_name}");
        assert_eq!(command.required_capabilities, ["llm"], "{command_name}");
        assert_eq!(command.result_references, ["task_ids"], "{command_name}");
        assert_eq!(
            command.args_schema["required"],
            serde_json::json!(["input", "taskId"]),
            "{command_name}",
        );
        assert_eq!(
            command.args_schema["properties"]["input"]["additionalProperties"], false,
            "{command_name}",
        );
        assert_eq!(
            command.result_schema["x-a4n-schema-quality"], "typed",
            "{command_name}",
        );

        let preview = command_descriptor(preview_name).unwrap();
        assert_eq!(
            preview.args_schema["required"],
            serde_json::json!(["input"]),
            "{preview_name}",
        );
        assert!(
            preview.args_schema["properties"].get("taskId").is_none(),
            "{preview_name}",
        );
    }
}

#[test]
fn source_sync_commands_expose_one_strict_typed_contract() {
    let commands = [
        "preview_source_sync",
        "apply_source_sync_decisions",
        "detach_derived_project",
    ];
    assert_commands_use_authenticated_host_identity(&commands);

    for command in commands {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
            "{command}",
        );
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([]),
            "{command}",
        );
        assert_eq!(
            descriptor.result_schema["x-a4n-schema-quality"], "typed",
            "{command}",
        );
    }

    let apply = command_descriptor("apply_source_sync_decisions").unwrap();
    assert!(
        apply.args_schema["properties"]
            .get("runtime_config_snapshot")
            .is_some()
    );
    assert_eq!(apply.runtime_config_policy, RuntimeConfigPolicy::Full);
}

#[test]
fn generation_start_commands_share_one_strict_runtime_envelope() {
    let commands = GENERATION_START_ROUTES
        .iter()
        .map(|(command, _)| *command)
        .collect::<Vec<_>>();
    assert_commands_use_authenticated_host_identity(&commands);

    for command in commands {
        let descriptor = command_descriptor(command).unwrap();
        let properties = descriptor.args_schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 2, "{command}");
        assert!(properties.contains_key("input"), "{command}");
        assert!(
            properties.contains_key("runtime_config_snapshot"),
            "{command}"
        );
        assert_eq!(
            descriptor.args_schema["required"],
            serde_json::json!(["input"])
        );
        assert_eq!(
            descriptor.args_schema["x-a4n-schema-quality"], "typed-input",
            "{command}"
        );
        assert_eq!(
            descriptor.runtime_config_policy,
            RuntimeConfigPolicy::Full,
            "{command}"
        );
    }
}

#[test]
fn shooting_script_generation_requires_a_confirmed_visual_plan() {
    let descriptor = command_descriptor("start_chapter_shooting_script_generation").unwrap();
    let input = &descriptor.args_schema["properties"]["input"];
    assert_eq!(
        input["required"],
        serde_json::json!([
            "meta",
            "project_id",
            "source_chapter_id",
            "visual_plan_id",
            "mode"
        ]),
    );
    assert_eq!(
        input["properties"]["visual_plan_id"]["minimum"],
        serde_json::json!(1)
    );
    assert!(command_descriptor("generate_chapter_shooting_script").is_none());
    assert!(command_descriptor("generate_next_chapter_shooting_script_unit").is_none());
}

#[test]
fn optimize_commands_expose_one_strict_typed_contract() {
    let commands = [
        "apply_optimization_patches",
        "create_optimization_analysis_session",
        "create_optimization_chat_session",
        "delete_optimize_session",
        "discard_optimization_patch_set",
        "discard_pending_optimize_patch_sets",
        "get_optimize_session",
        "list_optimize_patch_sets",
        "list_optimize_sessions",
        "list_optimize_turns",
        "prepare_optimization_analysis_session",
        "preview_create_optimization_analysis_session_request",
        "preview_prepare_optimization_analysis_session_request",
        "preview_propose_optimization_patches_request",
        "preview_send_optimization_message_request",
        "record_optimize_agent_event",
    ];
    assert_commands_use_authenticated_host_identity(&commands);

    for command in commands {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
            "{command}",
        );
        assert_eq!(
            descriptor.result_schema["x-a4n-schema-quality"], "typed",
            "{command}",
        );
    }

    for command in [
        "apply_optimization_patches",
        "create_optimization_analysis_session",
        "create_optimization_chat_session",
        "prepare_optimization_analysis_session",
        "preview_create_optimization_analysis_session_request",
        "preview_prepare_optimization_analysis_session_request",
        "preview_propose_optimization_patches_request",
        "preview_send_optimization_message_request",
    ] {
        assert!(
            command_descriptor(command).unwrap().args_schema["properties"]
                .get("runtime_config_snapshot")
                .is_some(),
            "{command}",
        );
    }

    let apply = command_descriptor("apply_optimization_patches").unwrap();
    assert!(
        apply.args_schema["properties"]["input"]["properties"]
            .get("confirmed_apply")
            .is_none()
    );
    assert!(
        apply.result_schema["properties"]
            .get("session_id")
            .is_none()
    );
    assert!(
        apply.result_schema["properties"]
            .get("content_hash")
            .is_none()
    );
    assert!(command_descriptor("reject_pending_optimize_patch_sets").is_none());
}

#[test]
fn material_mutations_use_the_request_embedding_configuration() {
    for command in [
        "accept_import_review",
        "apply_optimization_patches",
        "batch_accept_reviews",
        "confirm_reverse_engineering_candidate",
        "create_foreshadowing",
        "create_profile",
        "create_scene_profile",
        "delete_latest_chapter_outlines",
        "delete_latest_parts",
        "delete_scene_profile",
        "import_chapter",
        "restore_blueprint_version",
        "save_blueprint",
        "save_chapter_outline",
        "save_part_outline",
        "update_chapter_content",
        "update_chapter_outline",
        "update_character_profile",
        "update_foreshadowing",
        "update_scene_profile",
        "update_world_setting",
    ] {
        assert_eq!(
            command_runtime_config_policy(command),
            RuntimeConfigPolicy::Full,
            "{command} 会更新资料检索索引，必须复用当前请求的 Embedding 配置",
        );
    }
}

#[test]
fn profile_material_commands_expose_one_typed_contract() {
    for command in [
        "list_foreshadowing",
        "create_foreshadowing",
        "update_foreshadowing",
        "delete_foreshadowing",
        "list_object_profiles",
        "create_object_profile",
        "update_object_profile",
        "delete_object_profile",
        "list_scene_profiles",
        "create_scene_profile",
        "update_scene_profile",
        "delete_scene_profile",
        "list_profile_materials",
        "set_profile_primary_material",
        "upload_profile_material_image",
        "generate_object_profile_image_prompt",
        "generate_scene_profile_image_prompt",
        "preview_object_profile_image_request",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command}"
        );
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
            "{command}",
        );
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([]),
            "{command}",
        );
        assert_ne!(
            descriptor.result_schema["x-a4n-schema-quality"], "opaque",
            "{command}",
        );
    }

    assert!(
        command_descriptor("upload_profile_material_image")
            .unwrap()
            .required_capabilities
            .is_empty()
    );

    for command in [
        "generate_object_profile_image_prompt",
        "generate_scene_profile_image_prompt",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(descriptor.effect, "read", "{command}");
        assert_eq!(descriptor.execution, "immediate", "{command}");
        assert!(descriptor.secret_pointers.is_empty(), "{command}");
        assert!(descriptor.result_references.is_empty(), "{command}");
        assert_eq!(descriptor.runtime_config_policy, RuntimeConfigPolicy::None);
    }
}

#[test]
fn scene_profile_sync_uses_authenticated_typed_input_and_unit_result() {
    let descriptor = command_descriptor("sync_scene_profiles").unwrap();
    assert_eq!(descriptor.execution, "task");
    assert_eq!(descriptor.args_schema["additionalProperties"], false);
    assert_eq!(
        descriptor.args_schema["properties"]["input"]["additionalProperties"],
        false
    );
    assert!(descriptor.args_schema["properties"].get("userId").is_none());
    assert!(
        descriptor.args_schema["properties"]
            .get("projectId")
            .is_none()
    );
    assert!(
        descriptor.args_schema["properties"]
            .get("chapterNumber")
            .is_none()
    );
    assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    assert!(descriptor.result_references.is_empty());
}

#[test]
fn required_task_commands_reject_duplicate_identity_fields() {
    for command in [
        "export_batch_data",
        "export_epub_data",
        "export_markdown_data",
        "export_outline_json_data",
        "export_pdf_data",
        "export_txt_data",
        "export_characters_json_data",
        "sync_all_character_profiles",
        "sync_character_from_chapters",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command}"
        );
        assert!(
            descriptor.args_schema["properties"].get("userId").is_none(),
            "{command}"
        );
    }
    for command in [
        "sync_all_character_profiles",
        "sync_character_from_chapters",
    ] {
        assert_eq!(
            command_descriptor(command).unwrap().args_schema["x-a4n-context-fields"],
            serde_json::json!([]),
            "{command}",
        );
        assert!(
            command_descriptor(command).unwrap().args_schema["properties"]
                .get("runtime_config_snapshot")
                .is_some(),
            "{command}"
        );
    }
}

#[test]
fn character_history_commands_expose_one_strict_contract() {
    for command in [
        "create_profile",
        "list_profiles",
        "update_character_profile",
        "add_profile_attribute",
        "modify_profile_attribute",
        "attribute_history",
        "list_behaviors",
        "get_implicit_review",
        "check_implicit_update",
        "preview_check_implicit_update_request",
        "apply_implicit_update_decision",
        "list_snapshots",
        "get_profile_state_at_chapter",
        "get_snapshot",
        "diff_profile_chapters",
        "check_profile_sync_conflict",
        "rollback_to_chapter",
        "list_deletion_marks",
        "request_attribute_deletion",
        "execute_attribute_deletion",
        "reset_deletion_marks",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command}"
        );
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
            "{command}",
        );
        assert_ne!(
            descriptor.result_schema["x-a4n-schema-quality"], "opaque",
            "{command}"
        );
    }
    for command in [
        "create_profile",
        "update_character_profile",
        "check_implicit_update",
        "preview_check_implicit_update_request",
    ] {
        assert!(
            command_descriptor(command).unwrap().args_schema["properties"]
                .get("runtime_config_snapshot")
                .is_some(),
            "{command}",
        );
    }
}

#[test]
fn chapter_content_commands_expose_the_immediate_strict_contract() {
    let import = command_descriptor("import_chapter").unwrap();
    assert_eq!(import.execution, "immediate");
    assert_eq!(import.idempotency, "none");
    assert!(import.file_inputs.is_empty());
    assert_eq!(import.result_references, vec!["chapter_ids".to_string()]);
    assert_eq!(import.args_schema["additionalProperties"], false);
    assert_eq!(
        import.args_schema["properties"]["input"]["additionalProperties"],
        false
    );
    assert!(
        import.args_schema["properties"]["input"]["properties"]
            .get("manual")
            .is_none()
    );
    assert_eq!(import.result_schema["x-a4n-schema-quality"], "typed");

    let update = command_descriptor("update_chapter_content").unwrap();
    assert_eq!(
        update.args_schema["x-a4n-context-fields"],
        serde_json::json!([])
    );
    assert_eq!(update.args_schema["additionalProperties"], false);
    assert_eq!(update.result_schema["x-a4n-schema-quality"], "typed");

    let list = command_descriptor("list_chapters").unwrap();
    assert_eq!(list.result_schema["x-a4n-schema-quality"], "typed");
    let index = command_descriptor("list_chapter_index").unwrap();
    assert_eq!(index.effect, "read");
    assert_eq!(index.result_schema["x-a4n-schema-quality"], "typed");
    let analyses = command_descriptor("list_chapter_analyses").unwrap();
    assert_eq!(analyses.effect, "read");
    assert_eq!(analyses.result_schema["x-a4n-schema-quality"], "typed");
    assert_eq!(analyses.args_schema["additionalProperties"], false);
    let get = command_descriptor("get_chapter").unwrap();
    assert_eq!(get.effect, "read");
    assert_eq!(get.args_schema["properties"]["input"]["additionalProperties"], false);
    for id in ["project_id", "chapter_id"] {
        assert_eq!(get.args_schema["properties"]["input"]["properties"][id]["minimum"], 1);
    }
}

#[test]
fn chapter_analysis_commands_expose_one_strict_contract() {
    let generate = command_descriptor("generate_chapter_analysis").unwrap();
    assert_eq!(generate.execution, "task");
    assert_eq!(generate.idempotency, "none");
    assert_eq!(
        generate.args_schema["x-a4n-context-fields"],
        serde_json::json!([])
    );
    assert_eq!(generate.args_schema["additionalProperties"], false);
    assert_eq!(
        generate.args_schema["properties"]["input"]["additionalProperties"],
        false
    );
    assert_eq!(
        generate.args_schema["properties"]["taskId"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert!(generate.args_schema["properties"].get("userId").is_none());
    assert!(
        generate.args_schema["properties"]
            .get("chapterId")
            .is_none()
    );
    assert_eq!(generate.result_schema["x-a4n-schema-quality"], "typed");

    let preview = command_descriptor("preview_chapter_analysis_request").unwrap();
    assert_eq!(preview.effect, "read");
    assert_eq!(preview.args_schema["additionalProperties"], false);
    assert!(preview.args_schema["properties"].get("taskId").is_none());
    assert_eq!(preview.result_schema["x-a4n-schema-quality"], "typed");

    let resume = command_descriptor("resume_chapter_analysis_sync").unwrap();
    assert_eq!(resume.execution, "workflow");
    assert_eq!(resume.idempotency, "server_workflow");
    assert_eq!(resume.args_schema["additionalProperties"], false);
    assert_eq!(resume.result_schema["x-a4n-schema-quality"], "typed");

    let resume_preview =
        command_descriptor("preview_resume_chapter_analysis_sync_request").unwrap();
    assert_eq!(resume_preview.effect, "read");
    assert_eq!(resume_preview.args_schema["additionalProperties"], false);
    assert_eq!(
        resume_preview.result_schema["x-a4n-schema-quality"],
        "typed"
    );
}

#[test]
fn import_analysis_and_review_commands_expose_one_typed_contract() {
    for command in [
        "accept_import_review",
        "batch_accept_reviews",
        "cancel_import_analysis",
        "check_import_dedup",
        "delete_import_review",
        "get_project_import_analysis_progress",
        "list_import_queue",
        "preview_import_project_analysis_request",
        "reject_import_review",
        "run_import_project_analysis",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert!(descriptor.file_inputs.is_empty(), "{command}");
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command}"
        );
        assert_eq!(
            descriptor.result_schema["x-a4n-schema-quality"], "typed",
            "{command}"
        );
    }

    let run = command_descriptor("run_import_project_analysis").unwrap();
    assert_eq!(run.execution, "immediate");
    assert!(run.result_references.is_empty());

    let preview = command_descriptor("preview_import_project_analysis_request").unwrap();
    assert_eq!(preview.effect, "read");
    assert_eq!(
        preview.runtime_config_policy,
        RuntimeConfigPolicy::WithoutSecrets
    );
}

#[test]
fn reverse_engineering_uses_one_typed_command_family() {
    for command in [
        "reverse_engineer",
        "preview_reverse_engineer_request",
        "get_reverse_engineering_candidate",
        "confirm_reverse_engineering_candidate",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(
            descriptor.args_schema["additionalProperties"], false,
            "{command}"
        );
        assert_eq!(
            descriptor.result_schema["x-a4n-schema-quality"], "typed",
            "{command}"
        );
    }

    for retired in [
        "reverse_engineer_blueprint",
        "reverse_engineer_outline",
        "reverse_engineer_part_outline",
        "preview_reverse_engineer_blueprint_request",
        "preview_reverse_engineer_outline_request",
        "preview_reverse_engineer_part_outline_request",
    ] {
        assert!(command_descriptor(retired).is_none(), "{retired}");
    }

    let run = command_descriptor("reverse_engineer").unwrap();
    assert_eq!(run.execution, "workflow");
    assert_eq!(run.runtime_config_policy, RuntimeConfigPolicy::Full);
    assert!(run.args_schema["properties"].get("taskId").is_some());

    let preview = command_descriptor("preview_reverse_engineer_request").unwrap();
    assert_eq!(preview.effect, "read");
    assert_eq!(
        preview.runtime_config_policy,
        RuntimeConfigPolicy::WithoutSecrets
    );
}

#[test]
fn chapter_scene_commands_expose_one_strict_contract() {
    let generate = command_descriptor("generate_chapter_scene").unwrap();
    assert_eq!(generate.execution, "task");
    assert_eq!(generate.args_schema["additionalProperties"], false);
    assert_eq!(
        generate.args_schema["properties"]["input"]["additionalProperties"],
        false
    );
    assert!(generate.args_schema["properties"].get("userId").is_none());
    assert!(
        generate.args_schema["properties"]
            .get("chapterId")
            .is_none()
    );
    assert_eq!(generate.result_schema["x-a4n-schema-quality"], "typed");

    let preview = command_descriptor("preview_chapter_scene_request").unwrap();
    assert_eq!(preview.effect, "read");
    assert_eq!(preview.args_schema["additionalProperties"], false);
    assert!(preview.args_schema["properties"].get("userId").is_none());
    assert!(preview.args_schema["properties"].get("chapterId").is_none());
    assert_eq!(preview.result_schema["x-a4n-schema-quality"], "typed");

    for command in [
        "list_chapter_scenes",
        "update_chapter_scene",
        "delete_chapter_scene",
        "reorder_chapter_scenes",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_ne!(descriptor.args_schema["x-a4n-schema-quality"], "opaque");
        assert_eq!(descriptor.args_schema["additionalProperties"], false);
        assert_ne!(descriptor.result_schema["x-a4n-schema-quality"], "opaque");
    }
}

#[test]
fn task_workflow_commands_expose_generated_contracts_without_identity_args() {
    for command in ["get_queue_stats", "list_active_workflows"] {
        let descriptor = command_descriptor(command).unwrap();
        assert_eq!(descriptor.args_schema["additionalProperties"], false);
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    }

    let cancel_task = command_descriptor("cancel_task").unwrap();
    assert_eq!(cancel_task.args_schema["additionalProperties"], false);
    assert!(
        cancel_task.args_schema["properties"]
            .get("taskId")
            .is_some()
    );
    assert!(
        cancel_task.args_schema["properties"]
            .get("userId")
            .is_none()
    );
    assert_eq!(cancel_task.result_schema["x-a4n-schema-quality"], "typed");

    let history = command_descriptor("get_task_history").unwrap();
    assert_eq!(history.args_schema["additionalProperties"], false);
    assert!(history.args_schema["properties"].get("projectId").is_some());
    assert!(history.args_schema["properties"].get("userId").is_none());
    assert_eq!(history.result_schema["x-a4n-schema-quality"], "typed");

    for command in [
        "list_workflow_runs",
        "get_workflow_run_detail",
        "resume_workflow_run",
        "get_workflow_progress_events",
        "cancel_workflow",
    ] {
        let descriptor = command_descriptor(command).unwrap();
        assert_ne!(descriptor.args_schema["x-a4n-schema-quality"], "opaque");
        assert!(descriptor.args_schema["properties"].get("input").is_some());
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_ne!(descriptor.result_schema["x-a4n-schema-quality"], "opaque");
    }

    let cancel_generation = command_descriptor("cancel_generation_task").unwrap();
    assert_eq!(cancel_generation.args_schema["additionalProperties"], false);
    assert!(
        cancel_generation.args_schema["properties"]
            .get("input")
            .is_some()
    );
    assert!(
        cancel_generation.args_schema["properties"]
            .get("userId")
            .is_none()
    );
    assert_eq!(
        cancel_generation.result_schema["x-a4n-schema-quality"],
        "typed"
    );
}
