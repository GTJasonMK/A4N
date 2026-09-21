#[cfg(test)]
mod tests {
    use clap::Parser;
    use serde_json::json;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::{
        AgentCommand, Cli, Command, RuntimeConfigSnapshot, SessionContext, agent_update_body,
        apply_context_args, apply_runtime_config, catalog, decode_pointer_token,
        parse_file_assignment, prepare_outputs, query_path, read_json_source, read_password,
        read_password_pair, request_secret_values, save_outputs, set_json_pointer, task_path,
        trim_secret_line, validate_attempt, validate_non_empty, validate_optional_positive,
    };

    #[test]
    fn inline_json_and_query_values_are_parsed() {
        assert_eq!(
            read_json_source(r#"{"projectId":1}"#).unwrap(),
            json!({"projectId": 1})
        );
        assert_eq!(
            query_path(
                "/generation-tasks/history",
                [("limit", Some("20".into())), ("cursor", Some("a/b".into()))]
            ),
            "/generation-tasks/history?limit=20&cursor=a%2Fb"
        );
    }

    #[test]
    fn infers_agent_phases_only_for_work_commands() {
        assert!(
            Command::Call {
                command: "start_video_generation".into(),
                args: "{}".into(),
                set_base64: vec![],
                set_text: vec![],
                save_base64: vec![],
                save_text: vec![],
                force: false,
                allow_destructive: false,
            }
            .auto_agent_phase()
            .is_some()
        );
        assert!(Command::Health.auto_agent_phase().is_none());
        assert!(
            Command::Agent {
                command: AgentCommand::Status,
            }
            .auto_agent_phase()
            .is_none()
        );
    }

    #[test]
    fn durable_agent_commands_parse_without_breaking_existing_syntax() {
        let label = |args: &[&str]| Cli::try_parse_from(args).unwrap().command.label();

        assert_eq!(
            label(&[
                "a4n",
                "agent",
                "run",
                "run-1",
                "pause",
                "--thread-id",
                "thread-1",
            ]),
            "agent run"
        );
        assert_eq!(
            label(&["a4n", "agent", "tools", "--query", "canvas"]),
            "agent tools"
        );
        for (args, expected) in [
            (&["a4n", "agent", "ask", "继续写作"][..], "agent ask"),
            (
                &["a4n", "agent", "thread", "fork", "thread-1"][..],
                "agent thread fork",
            ),
            (
                &[
                    "a4n",
                    "agent",
                    "thread",
                    "update",
                    "thread-1",
                    "--title",
                    "新标题",
                ][..],
                "agent thread update",
            ),
            (
                &["a4n", "agent", "thread", "archive", "thread-1"][..],
                "agent thread archive",
            ),
            (&["a4n", "agent", "run-show", "run-1"][..], "agent run show"),
            (
                &["a4n", "agent", "message", "list", "thread-1"][..],
                "agent message list",
            ),
            (
                &["a4n", "agent", "tool", "describe", "get_canvas_graph"][..],
                "agent tool describe",
            ),
            (
                &[
                    "a4n",
                    "agent",
                    "memory",
                    "create",
                    "project",
                    "7",
                    "style",
                    "第三人称",
                ][..],
                "agent memory create",
            ),
            (
                &[
                    "a4n",
                    "agent",
                    "child",
                    "spawn",
                    "run-1",
                    "research",
                    "调查设定",
                ][..],
                "agent child spawn",
            ),
            (
                &["a4n", "agent", "child", "send", "delegation-1", "继续"][..],
                "agent child send",
            ),
        ] {
            assert_eq!(label(args), expected);
        }
    }

    #[test]
    fn file_json_pointers_create_inputs_and_decode_outputs_without_overwrite() {
        let mut input = json!({});
        set_json_pointer(&mut input, "/input/data_base64", json!("aGVsbG8=")).unwrap();
        assert_eq!(input["input"]["data_base64"], "aGVsbG8=");

        let path = std::env::temp_dir().join(format!("a4n-cli-{}.txt", Uuid::new_v4()));
        let spec = format!("/data_base64={}", path.display());
        let mut output = json!({ "data_base64": "aGVsbG8=" });
        let outputs = prepare_outputs(std::slice::from_ref(&spec), &[], false).unwrap();
        save_outputs(&mut output, &outputs, false).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert_eq!(output["data_base64"]["byte_size"], 5);

        assert!(prepare_outputs(&[spec], &[], false).is_err());
        std::fs::remove_file(path).unwrap();

        let text_path = std::env::temp_dir().join(format!("a4n-cli-{}.json", Uuid::new_v4()));
        let mut text = json!("{\"ok\":true}");
        let outputs = prepare_outputs(&[], &[format!("={}", text_path.display())], false).unwrap();
        save_outputs(&mut text, &outputs, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(&text_path).unwrap(),
            "{\"ok\":true}"
        );
        std::fs::remove_file(text_path).unwrap();
    }

    #[test]
    fn file_pointer_validation_covers_arrays_escaping_and_atomic_output_failures() {
        assert_eq!(
            decode_pointer_token("name~0part~1value").unwrap(),
            "name~part/value"
        );
        assert!(decode_pointer_token("bad~2escape").is_err());

        let mut input = json!({"items": [{"name": "old"}]});
        set_json_pointer(&mut input, "/items/0/name", json!("new")).unwrap();
        assert_eq!(input["items"][0]["name"], "new");
        for (pointer, expected) in [
            ("", "empty pointer"),
            ("/items/1/name", "array bounds"),
            ("/items/name", "array index"),
            ("/items/0/name/child", "scalar parent"),
            ("/items/0/bad~2escape", "pointer escape"),
        ] {
            assert!(
                set_json_pointer(&mut input, pointer, json!("x")).is_err(),
                "{expected}"
            );
        }

        let temp = TempDir::new().unwrap();
        let output = temp.path().join("artifact.bin");
        assert!(parse_file_assignment("data.bin", true).is_err());
        assert!(parse_file_assignment("data=/tmp/artifact", false).is_err());
        assert!(parse_file_assignment("/data=", true).is_err());
        assert!(prepare_outputs(&[], &[], true).is_err());
        assert!(
            prepare_outputs(
                &[
                    format!("/one={}", output.display()),
                    format!("/two={}", output.display())
                ],
                &[],
                false,
            )
            .is_err()
        );

        let first = temp.path().join("first.bin");
        let second = temp.path().join("second.bin");
        let specs = [
            format!("/first={}", first.display()),
            format!("/second={}", second.display()),
        ];
        let assignments = prepare_outputs(&specs, &[], false).unwrap();
        let mut response = json!({"first": "aGVsbG8=", "second": "not-base64%%%"});
        assert!(save_outputs(&mut response, &assignments, false).is_err());
        assert!(
            !first.exists(),
            "decode all outputs before writing any file"
        );
        assert!(!second.exists());
    }

    #[test]
    fn cli_scalar_validation_rejects_empty_ranges_and_invalid_agent_progress() {
        assert!(validate_attempt(0).is_err());
        assert!(validate_non_empty("  ", "task_id").is_err());
        assert!(validate_optional_positive(Some(0), "project_id").is_err());
        assert!(validate_optional_positive(None, "project_id").is_ok());
        assert_eq!(
            task_path("task/id", 2, "/snapshot").unwrap(),
            "/generation-tasks/task%2Fid/snapshot?attempt=2"
        );
        assert!(task_path("", 1, "").is_err());
        assert!(task_path("task", 0, "").is_err());

        assert!(read_password(false).is_err());
        assert!(read_password_pair(false).is_err());
        assert!(trim_secret_line("\n".into()).is_err());
        assert_eq!(trim_secret_line("secret\r\n".into()).unwrap(), "secret");

        assert!(
            agent_update_body(Some("unknown".into()), None, None, None, None, None, None).is_err()
        );
        assert!(agent_update_body(None, None, None, Some(f32::NAN), None, None, None).is_err());
        assert!(agent_update_body(None, None, None, Some(100.1), None, None, None).is_err());
        let body = agent_update_body(
            Some("running".into()),
            Some("phase".into()),
            Some("summary".into()),
            Some(25.0),
            Some("task-1".into()),
            Some(7),
            Some(8),
        )
        .unwrap();
        assert_eq!(body["status"], "running");
        assert_eq!(body["progress"], 25.0);
        assert_eq!(body["current_task_id"], "task-1");
        assert_eq!(body["project_id"], 7);
        assert_eq!(body["chapter_id"], 8);
    }

    #[test]
    fn call_context_injects_only_explicit_resource_identity() {
        let mut context = SessionContext::load(None, "http://localhost/api").unwrap();
        context.bind_user(7).unwrap();
        let mut settings_args = json!({});
        let settings_descriptor = catalog::command_descriptor("detect_llm_config_models").unwrap();
        apply_context_args(&mut settings_args, &settings_descriptor, &context).unwrap();
        assert_eq!(settings_args, json!({}));

        let mut user_args = json!({});
        let user_descriptor = catalog::command_descriptor("create_scene_profile").unwrap();
        apply_context_args(&mut user_args, &user_descriptor, &context).unwrap();
        assert_eq!(user_args, json!({}));

        let mut admin_args = json!({});
        let admin_descriptor = catalog::command_descriptor("list_all_users").unwrap();
        apply_context_args(&mut admin_args, &admin_descriptor, &context).unwrap();
        assert_eq!(admin_args, json!({}));
    }

    #[test]
    fn call_runtime_config_respects_manifest_policy_and_explicit_override() {
        let snapshot: RuntimeConfigSnapshot = serde_json::from_value(json!({
            "schema_version": 2,
            "llm": {
                "id": 1,
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "secret",
                "model": "model",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "max_context_tokens": 4096,
                "max_output_tokens": 1024
            }
        }))
        .unwrap();

        let mut full = json!({});
        apply_runtime_config(
            &mut full,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(
            full["runtime_config_snapshot"]["llm"]["api_key_encrypted"],
            "secret"
        );

        let mut preview = json!({});
        apply_runtime_config(
            &mut preview,
            &catalog::command_descriptor("preview_check_character_conflict_request").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert!(
            preview["runtime_config_snapshot"]["llm"]
                .get("api_key_encrypted")
                .is_none()
        );

        let mut no_runtime = json!({});
        apply_runtime_config(
            &mut no_runtime,
            &catalog::command_descriptor("list_projects").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(no_runtime, json!({}));

        let mut explicit = json!({ "runtime_config_snapshot": { "schema_version": 2 } });
        apply_runtime_config(
            &mut explicit,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(
            explicit["runtime_config_snapshot"],
            json!({ "schema_version": 2 })
        );

        let mut disabled = json!({ "runtime_config_snapshot": null });
        apply_runtime_config(
            &mut disabled,
            &catalog::command_descriptor("check_character_conflict").unwrap(),
            Some(&snapshot),
        )
        .unwrap();
        assert_eq!(disabled, json!({}));
    }

    #[test]
    fn call_redaction_collects_nested_runtime_config_secrets() {
        let descriptor = catalog::command_descriptor("apply_source_sync_decisions").unwrap();
        let secrets = request_secret_values(
            &descriptor,
            &json!({
                "runtime_config_snapshot": {
                    "llm": { "api_key_encrypted": "active-key" },
                    "llm_configs": [
                        { "api_key_encrypted": "first-key" },
                        { "api_key_encrypted": "second-key" }
                    ],
                    "system_llm_api_key_encrypted": "system-key",
                    "token_count": 4096
                }
            }),
        );
        for secret in ["active-key", "first-key", "second-key", "system-key"] {
            assert!(secrets.iter().any(|value| value == secret), "{secret}");
        }
        assert!(!secrets.iter().any(|value| value == "4096"));
    }
}
