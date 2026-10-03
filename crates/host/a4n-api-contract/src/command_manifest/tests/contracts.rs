    #[test]
    fn manga_page_prompt_contract_stays_on_the_canonical_command() {
        assert!(command_descriptor("list_manga_panels").is_none());
        let descriptor = command_descriptor("list_manga_page_prompts").unwrap();
        assert_eq!(descriptor.domain, "manga");
        assert_eq!(descriptor.effect, "read");
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn manga_pipeline_start_commands_return_task_handles() {
        for command in ["start_manga_pipeline", "resume_manga_pipeline"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.result_schema["type"], "object", "{command}");
            assert_eq!(
                descriptor.result_schema["properties"]["task_id"]["type"],
                "string",
                "{command}"
            );
        }
    }

    #[test]
    fn chapter_video_preview_uses_the_shared_override_contract() {
        let descriptor = command_descriptor("preview_chapter_video_request").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-schema-quality"],
            "typed-input"
        );
        let prompt_override = &descriptor.args_schema["properties"]["promptOverride"];
        assert_eq!(prompt_override["type"], "object");
        assert_eq!(prompt_override["additionalProperties"], false);
    }

    #[test]
    fn update_profile_uses_authenticated_identity_and_non_null_email() {
        let descriptor = command_descriptor("update_user_profile").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(
            descriptor.args_schema["properties"]["input"]["properties"]["email"]["type"],
            "string"
        );
    }

    #[test]
    fn provider_config_commands_use_session_identity_and_strict_contracts() {
        for command in [
            "detect_llm_config_models",
            "detect_image_config_models",
            "detect_video_config_models",
            "test_llm_config",
            "test_image_config",
            "test_embedding_config",
            "preview_test_llm_config_request",
            "preview_test_image_config_request",
            "preview_test_embedding_config_request",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
            assert_eq!(descriptor.args_schema["additionalProperties"], false);
        }

        for command in [
            "update_llm_config",
            "update_image_config",
            "update_video_config",
            "update_embedding_config",
        ] {
            let schema = command_descriptor(command).unwrap().args_schema;
            assert_eq!(schema["required"], serde_json::json!(["configId", "input"]));
            assert_eq!(schema["additionalProperties"], false);
        }

        let catalog = command_descriptor("list_model_provider_catalog").unwrap();
        assert_eq!(catalog.args_schema["additionalProperties"], false);
        assert!(catalog.args_schema.get("x-a4n-schema-quality").is_some());
    }

    #[test]
    fn change_password_uses_authenticated_identity_and_unit_result() {
        let descriptor = command_descriptor("change_user_password").unwrap();
        assert_eq!(
            descriptor.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert!(descriptor.args_schema["properties"].get("userId").is_none());
        assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn session_commands_use_one_authenticated_identity() {
        assert!(command_descriptor("logout_user").is_none());

        let current_user = command_descriptor("get_current_user").unwrap();
        assert_eq!(
            current_user.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(current_user.args_schema["additionalProperties"], false);

        let list = command_descriptor("list_sessions").unwrap();
        assert_eq!(
            list.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(list.args_schema["additionalProperties"], false);

        let revoke = command_descriptor("revoke_session").unwrap();
        assert_eq!(
            revoke.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(
            revoke.args_schema["properties"]["input"]["properties"]["session_id"]["type"],
            "string"
        );
        assert_eq!(revoke.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn theme_commands_use_one_authenticated_identity() {
        for command in [
            "list_themes",
            "create_theme",
            "update_theme",
            "delete_theme",
            "activate_theme",
            "export_theme",
            "import_theme",
            "get_theme_preference",
            "update_theme_preference",
            "upload_theme_resource",
            "delete_theme_resource",
            "export_theme_package",
            "import_theme_package",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command} must use the authenticated Host identity"
            );
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command} must reject retired identity args"
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
        }
    }

    #[test]
    fn inspiration_commands_use_one_strict_contract() {
        let commands = [
            "get_inspiration_history",
            "preview_inspiration_request",
            "start_inspiration_generation",
        ];
        assert_commands_use_authenticated_host_identity(&commands);
        for command in commands {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed"
            );
        }

        let history = command_descriptor("get_inspiration_history").unwrap();
        assert!(
            history.args_schema["properties"]["input"]["properties"]
                .get("limit")
                .is_none()
        );
        let preview = command_descriptor("preview_inspiration_request").unwrap();
        assert!(
            preview.args_schema["properties"]["input"]["properties"]
                .get("user_input")
                .is_none()
        );
        assert!(
            preview.args_schema["properties"]["input"]["properties"]
                .get("variables")
                .is_none()
        );
        let start = command_descriptor("start_inspiration_generation").unwrap();
        assert!(
            start.args_schema["properties"]["input"]["properties"]
                .get("variables")
                .is_none()
        );
    }

    #[test]
    fn blueprint_commands_use_one_authenticated_strict_contract() {
        let commands = [
            "get_blueprint",
            "sync_blueprint_materials",
            "preview_generate_blueprint_request",
            "preview_refine_blueprint_request",
            "list_blueprint_versions",
            "diff_blueprint_versions",
            "restore_blueprint_version",
            "save_blueprint",
            "update_world_setting",
        ];
        assert_commands_use_authenticated_host_identity(&commands);

        for command in commands {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{command}"
            );
        }

        for command in ["get_blueprint", "sync_blueprint_materials"] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(descriptor.args_schema["properties"].get("input").is_none());
        }

        for command in [
            "preview_generate_blueprint_request",
            "preview_refine_blueprint_request",
            "list_blueprint_versions",
            "diff_blueprint_versions",
            "restore_blueprint_version",
            "save_blueprint",
            "update_world_setting",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["properties"]["input"]["additionalProperties"], false,
                "{command}"
            );
        }

        for command in [
            "preview_generate_blueprint_request",
            "preview_refine_blueprint_request",
            "restore_blueprint_version",
            "save_blueprint",
            "update_world_setting",
        ] {
            assert!(
                command_descriptor(command).unwrap().args_schema["properties"]
                    .get("runtime_config_snapshot")
                    .is_some(),
                "{command}"
            );
        }

        for command in ["list_blueprint_versions", "diff_blueprint_versions"] {
            assert!(
                command_descriptor(command).unwrap().args_schema["properties"]
                    .get("runtime_config_snapshot")
                    .is_none(),
                "{command}"
            );
        }

        assert_eq!(
            command_descriptor("update_world_setting").unwrap().domain,
            "blueprint"
        );

        for command in [
            "preview_generate_blueprint_request",
            "start_blueprint_generation",
        ] {
            let target_length = &command_descriptor(command).unwrap().args_schema["properties"]["input"]
                ["properties"]["target_length"];
            assert_eq!(target_length["minimum"], 5, "{command}");
            assert_eq!(target_length["maximum"], 10_000, "{command}");
        }
    }

    #[test]
    fn galgame_commands_use_one_authenticated_identity() {
        assert_commands_use_authenticated_host_identity(&[
            "get_galgame_script",
            "start_galgame_pipeline",
            "cancel_galgame_pipeline",
            "preview_galgame_pipeline_request",
            "get_galgame_pipeline_status",
            "update_galgame_line",
            "insert_galgame_line",
            "update_galgame_scene",
            "delete_galgame_line",
            "reorder_galgame_lines",
            "update_galgame_choices",
            "plan_galgame_assets",
            "run_galgame_asset_generation",
            "bind_galgame_media",
            "unbind_galgame_media",
            "import_galgame_audio",
            "export_galgame_webgal",
        ]);
    }

    #[test]
    fn prompt_commands_use_authenticated_identity_and_strict_contracts() {
        let commands = [
            "prompt_list",
            "prompt_list_categories",
            "prompt_list_project_overrides",
            "prompt_load_defaults",
            "prompt_remove_project_override",
            "prompt_render",
            "prompt_render_ab",
            "prompt_reset",
            "prompt_set_project_override",
            "prompt_update",
            "prompt_validate_variables",
            "prompt_version_diff",
            "prompt_version_history",
        ];
        assert_commands_use_authenticated_host_identity(&commands);

        for command in [
            "prompt_remove_project_override",
            "prompt_render",
            "prompt_render_ab",
            "prompt_set_project_override",
            "prompt_update",
            "prompt_validate_variables",
            "prompt_version_diff",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["properties"]["input"]["additionalProperties"],
                false,
                "{command}"
            );
        }

        let render = command_descriptor("prompt_render").unwrap();
        let required = render.args_schema["properties"]["input"]["required"]
            .as_array()
            .unwrap();
        assert_eq!(required.len(), 3);
        for field in ["project_id", "template_name", "variables"] {
            assert!(required.contains(&serde_json::json!(field)), "{field}");
        }

        let update = command_descriptor("prompt_update").unwrap();
        assert!(
            update.args_schema["properties"]["input"]["properties"]
                .get("author")
                .is_none()
        );
    }

    #[test]
    fn rag_commands_use_authenticated_identity_and_typed_contracts() {
        let commands = [
            "prepare_chapter_generation_rag_embedding",
            "rag_enhanced_search",
            "rag_enhanced_search_with_vectors",
            "rag_ingest_all",
            "rag_prepare_enhanced_search",
            "rag_prepare_ingest_all",
            "rag_upload_ingest_embeddings",
        ];
        assert_commands_use_authenticated_host_identity(&commands);

        for command in commands {
            let descriptor = command_descriptor(command).unwrap();
            assert_ne!(
                descriptor.args_schema["x-a4n-schema-quality"], "opaque",
                "{command} args"
            );
            assert_ne!(
                descriptor.result_schema["x-a4n-schema-quality"], "opaque",
                "{command} result"
            );
        }

        let search = command_descriptor("rag_enhanced_search").unwrap();
        let search_input = &search.args_schema["properties"]["input"];
        assert!(
            search_input["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("top_k"))
        );
        assert_eq!(search_input["properties"]["top_k"]["minimum"], 1);
        assert_eq!(search.effect, "read");

        let prepare = command_descriptor("rag_prepare_ingest_all").unwrap();
        assert_eq!(prepare.effect, "read");

        let upload = command_descriptor("rag_upload_ingest_embeddings").unwrap();
        assert!(upload.file_inputs.is_empty());
        let upload_input = &upload.args_schema["properties"]["input"];
        assert!(
            upload_input["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("rebuild"))
        );
    }

    #[test]
    fn manga_commands_use_one_authenticated_identity() {
        assert_commands_use_authenticated_host_identity(&[
            "list_manga_page_prompts",
            "bind_manga_reference_image",
            "update_manga_prompt",
            "optimize_manga_prompt",
            "delete_manga_prompt",
            "delete_manga_by_chapter",
            "start_manga_pipeline",
            "preview_start_manga_pipeline_request",
            "resume_manga_pipeline",
            "preview_resume_manga_pipeline_request",
            "get_manga_pipeline_status",
            "list_manga_pipeline_statuses",
            "cancel_manga_pipeline",
        ]);
    }

    #[test]
    fn canvas_commands_use_one_authenticated_identity() {
        assert_commands_use_authenticated_host_identity(&[
            "get_canvas_graph",
            "get_canvas_node_contracts",
            "save_canvas_graph",
            "save_canvas_text_asset",
            "preview_canvas_node_request",
            "run_canvas_graph",
            "run_canvas_node",
            "import_canvas_media",
        ]);
    }

    #[test]
    fn outline_commands_use_one_authenticated_identity() {
        assert_commands_use_authenticated_host_identity(&[
            "list_parts",
            "save_part_outline",
            "start_outline_generation",
            "preview_generate_parts_request",
            "preview_continue_part_outlines_request",
            "preview_regenerate_parts_request",
            "preview_regenerate_last_part_request",
            "preview_regenerate_part_outline_request",
            "delete_latest_parts",
            "preview_generate_part_chapter_outlines_request",
            "preview_batch_generate_part_outlines_request",
            "get_generation_progress",
            "preview_apply_beat_mapping_request",
            "list_chapter_outlines",
            "save_chapter_outline",
            "preview_generate_chapter_outlines_by_count_request",
            "preview_regenerate_chapter_outline_request",
            "update_chapter_outline",
            "delete_latest_chapter_outlines",
            "get_chapter_outlines_pacing",
        ]);
    }

    #[test]
    fn portrait_commands_use_one_typed_authenticated_contract() {
        let commands = [
            "generate_portrait_prompt",
            "get_primary_portrait",
            "list_portraits_by_project",
            "preview_generate_age_variant_request",
            "preview_generate_missing_portraits_request",
            "preview_generate_portrait_prompt_request",
            "preview_generate_portrait_request",
            "preview_regenerate_portrait_request",
            "set_primary_portrait",
            "start_portrait_generation",
            "upload_portrait",
        ];
        assert_commands_use_authenticated_host_identity(&commands);
        for command in commands {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command} must expose its real result contract"
            );
        }

        let prompt = command_descriptor("generate_portrait_prompt").unwrap();
        assert_eq!(prompt.domain, "prompt");
        assert_eq!(prompt.execution, "immediate");
        assert_eq!(prompt.required_capabilities, ["prompt", "llm"]);

        let generation = command_descriptor("start_portrait_generation").unwrap();
        assert_eq!(generation.required_capabilities, ["prompt", "llm", "image"]);
        assert!(
            command_descriptor("upload_portrait")
                .unwrap()
                .required_capabilities
                .is_empty()
        );
    }

    #[test]
    fn image_commands_use_one_typed_authenticated_contract() {
        let commands = [
            "delete_image",
            "generate_object_profile_image_prompt",
            "generate_scene_profile_image_prompt",
            "list_images_by_chapter",
            "list_images_by_project",
            "list_images_by_projects",
            "list_video_reference_images",
            "preview_batch_images_request",
            "preview_object_profile_image_request",
            "preview_page_image_request",
            "preview_scene_image_request",
            "start_image_generation",
            "update_image",
            "upload_image",
        ];
        assert_commands_use_authenticated_host_identity(&commands);
        for command in commands {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command} must expose its real result contract"
            );
        }
    }

    #[test]
    fn video_record_commands_use_one_authenticated_identity() {
        let commands = [
            "list_videos_by_chapter",
            "update_video",
            "select_segment_video",
            "load_video_playback_data",
            "load_video_scrub_playback_data",
            "assemble_chapter_videos",
            "load_cached_assembled_chapter_video_preview",
            "push_assembled_chapter_video",
            "update_assembly_push_status",
            "apply_assembly_postprocess_edit",
            "delete_video",
            "upload_chapter_video",
            "upload_reference_media_video",
            "upload_assembly_insert_video",
            "trim_video",
            "preview_chapter_video_request",
            "preview_video_provider_request_by_video_id",
            "preview_reference_media_video_request",
        ];
        assert_commands_use_authenticated_host_identity(&commands);
        for command in commands {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command} must expose its real result contract"
            );
        }
        assert_eq!(
            command_runtime_config_policy("preview_video_provider_request_by_video_id"),
            RuntimeConfigPolicy::None
        );
    }

    #[test]
    fn video_segment_recipe_updates_expose_typed_cli_contracts() {
        let draft = command_descriptor("update_video_segment_plan_draft_recipe").unwrap();
        assert_eq!(draft.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(draft.args_schema["additionalProperties"], false);
        assert_eq!(
            draft.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert_eq!(draft.result_schema["x-a4n-schema-quality"], "typed");

        let published = command_descriptor("update_chapter_video_segment_recipe").unwrap();
        assert_eq!(
            published.args_schema["x-a4n-context-fields"],
            serde_json::json!([])
        );
        assert_eq!(published.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(published.args_schema["additionalProperties"], false);
        assert_eq!(
            published.args_schema["properties"]["promptId"]["minimum"],
            1
        );
        assert_eq!(
            published.args_schema["properties"]["recipe"]["type"],
            "object"
        );
        assert_eq!(published.result_schema["x-a4n-schema-quality"], "typed");
        assert_eq!(published.result_schema["type"], "array");

        let plan = command_descriptor("update_chapter_video_segment_plan").unwrap();
        assert_eq!(plan.args_schema["x-a4n-schema-quality"], "typed-input");
        assert_eq!(
            plan.args_schema["properties"]["input"]["additionalProperties"],
            false
        );
        assert_eq!(
            plan.args_schema["properties"]["input"]["properties"]["segments"]["minItems"],
            1
        );
        assert_eq!(plan.result_schema["x-a4n-schema-quality"], "typed");
    }

    #[test]
    fn project_commands_use_authenticated_identity_without_user_id_args() {
        for command in [
            "list_projects",
            "create_project",
            "create_derived_project",
            "create_freeform_video_project",
            "duplicate_project",
            "export_project_family_data",
            "export_derived_project_data",
            "import_project_family_upload",
            "export_video_studio_project_data",
            "import_video_studio_project_upload",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
        }

        for command in [
            "export_project_family_data",
            "export_derived_project_data",
            "import_project_family_upload",
            "export_video_studio_project_data",
            "import_video_studio_project_upload",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().args_schema["additionalProperties"],
                false,
                "{command}"
            );
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command}"
            );
        }
        assert_eq!(
            command_descriptor("import_project_family_upload")
                .unwrap()
                .args_schema,
            command_descriptor("import_video_studio_project_upload")
                .unwrap()
                .args_schema
        );
        assert!(command_descriptor("import_project_full_upload").is_none());
        assert!(command_descriptor("branch_project").is_none());

        let duplicate = command_descriptor("duplicate_project").unwrap();
        assert_eq!(duplicate.result_schema["x-a4n-schema-quality"], "typed");

        let update = command_descriptor("update_project").unwrap();
        assert_eq!(
            update.args_schema["x-a4n-context-fields"],
            serde_json::json!(["projectId"])
        );
        assert!(update.args_schema["properties"].get("userId").is_none());

        for command in [
            "get_project",
            "get_project_asset_dependency_graph",
            "get_project_asset_dependency_graph_node_detail",
            "get_project_status_machine",
            "list_chapters",
            "list_source_snapshot_chapters",
            "delete_project",
            "reset_project_generated_data",
            "sync_blueprint_materials",
            "sync_outline_materials",
            "forward_project_status",
            "preview_rollback",
            "rollback_project_status",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"].get("userId").is_none(),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command}"
            );
        }

        for command in [
            "get_project_asset_dependency_graph",
            "get_project_asset_dependency_graph_node_detail",
        ] {
            assert_eq!(
                command_descriptor(command).unwrap().result_schema["x-a4n-schema-quality"],
                "typed",
                "{command}"
            );
        }
    }

    #[test]
    fn collaboration_commands_separate_caller_from_target_identity() {
        for command in [
            "list_my_project_invites",
            "accept_project_invite",
            "decline_project_invite",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
        }

        for command in [
            "list_project_members",
            "invite_project_member",
            "list_project_invites",
            "revoke_project_invite",
            "update_project_member_role",
            "remove_project_member",
            "leave_project",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!(["projectId"]),
                "{command}"
            );
            assert!(descriptor.args_schema["properties"].get("userId").is_none());
        }
    }

    #[test]
    fn provider_defaults_use_one_strict_admin_contract() {
        assert!(command_descriptor("activate_image_config").is_none());
        assert!(command_descriptor("activate_video_config").is_none());

        for command in [
            "set_default_embedding_config",
            "set_default_image_config",
            "set_default_llm_config",
            "set_default_video_config",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.auth, "admin");
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([])
            );
            assert_eq!(descriptor.args_schema["additionalProperties"], false);
            assert_eq!(descriptor.result_schema["x-a4n-schema-quality"], "typed");
        }
    }

    #[test]
    fn admin_commands_use_authenticated_identity_without_caller_args() {
        for command in [
            "get_admin_overview",
            "get_generation_call_analytics",
            "get_storage_paths",
            "get_token_summary",
            "get_usage_trends",
            "get_user_usage",
            "health_check",
            "list_all_projects",
            "list_all_users",
            "list_invite_codes",
            "list_orphan_project_data_dirs",
            "list_pending_tasks",
        ] {
            let descriptor = command_descriptor(command).unwrap();
            assert_eq!(descriptor.auth, "admin", "{command}");
            assert_eq!(
                descriptor.args_schema["x-a4n-context-fields"],
                serde_json::json!([]),
                "{command}"
            );
            assert!(
                descriptor.args_schema["properties"]
                    .get("callerUserId")
                    .is_none(),
                "{command}"
            );
            assert_eq!(
                descriptor.args_schema["additionalProperties"], false,
                "{command}"
            );
        }
    }

    #[test]
    fn orphan_cleanup_requires_the_confirmed_project_id_set() {
        let descriptor = command_descriptor("cleanup_orphan_project_data_dirs").unwrap();
        let project_ids =
            &descriptor.args_schema["properties"]["input"]["properties"]["project_ids"];

        assert_eq!(project_ids["type"], "array");
        assert_eq!(project_ids["minItems"], 1);
        assert_eq!(project_ids["items"]["type"], "integer");
        assert!(descriptor.args_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|argument| argument == "input"));
    }

    #[test]
    fn package_contract_covers_all_commands_and_representative_boundaries() {
        use crate::package::{PackageCommandCategory, PackageId, package_catalog};

        let package_ids = package_catalog()
            .packages
            .into_iter()
            .map(|package| package.id)
            .collect::<HashSet<_>>();
        for command in API_COMMANDS {
            let descriptor = command_descriptor(command).expect("declared command descriptor");
            assert!(package_ids.contains(&descriptor.package_id), "{command}");
        }

        for (command, package_id, category) in [
            (
                "get_package_overview",
                PackageId::Platform,
                PackageCommandCategory::Normal,
            ),
            (
                "list_chapters",
                PackageId::NovelManagement,
                PackageCommandCategory::Normal,
            ),
            (
                "start_chapter_generation",
                PackageId::AiWorkflow,
                PackageCommandCategory::Generation,
            ),
            (
                "start_video_generation",
                PackageId::Video,
                PackageCommandCategory::Generation,
            ),
            (
                "list_manga_page_prompts",
                PackageId::Manga,
                PackageCommandCategory::Normal,
            ),
            (
                "import_project_family_upload",
                PackageId::ExternalImport,
                PackageCommandCategory::Normal,
            ),
            (
                "export_project_family_data",
                PackageId::Export,
                PackageCommandCategory::Normal,
            ),
            (
                "cancel_generation_task",
                PackageId::Platform,
                PackageCommandCategory::Maintenance,
            ),
        ] {
            let descriptor = command_descriptor(command).expect("representative command");
            assert_eq!(descriptor.package_id, package_id, "{command}");
            assert_eq!(descriptor.package_category, category, "{command}");
        }
    }
