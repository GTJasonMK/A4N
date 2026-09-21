use a4n_api_contract::command_manifest::command_descriptor;
use a4n_api_contract::generation_tasks::generation_start_path;
use anyhow::{Result, anyhow, bail};
use reqwest::Method;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::client::JsonRequest;

pub(crate) fn command_request(command: &str, args: Value) -> Result<JsonRequest> {
    validate_command_name(command)?;
    let Value::Object(args) = args else {
        bail!("命令参数必须是 JSON object");
    };
    if let Some(path) = generation_start_path(command) {
        return Ok(JsonRequest {
            method: Method::POST,
            path: path.to_string(),
            body: Some(generation_input(command, &args)?),
            authenticated: true,
        });
    }
    let authenticated = command_descriptor(command).is_none_or(|command| command.auth != "public");
    Ok(JsonRequest {
        method: Method::POST,
        path: format!("/commands/{command}"),
        body: Some(Value::Object(args)),
        authenticated,
    })
}

fn generation_input(command: &str, args: &Map<String, Value>) -> Result<Value> {
    if let Some(field) = args
        .keys()
        .find(|field| !matches!(field.as_str(), "input" | "runtime_config_snapshot"))
    {
        bail!("{command} 不接受参数: {field}");
    }
    let Some(Value::Object(mut input)) = args.get("input").cloned() else {
        bail!("generation input 必须是 JSON object");
    };
    let meta = input.entry("meta").or_insert_with(|| json!({}));
    let meta = meta
        .as_object_mut()
        .ok_or_else(|| anyhow!("generation input.meta 必须是 JSON object"))?;
    meta.entry("start_request_id")
        .or_insert_with(|| Value::String(Uuid::new_v4().to_string()));
    meta.entry("retry_of_task_id").or_insert(Value::Null);
    if let Some(snapshot) = args.get("runtime_config_snapshot") {
        input.insert("runtime_config_snapshot".to_string(), snapshot.clone());
    }
    Ok(Value::Object(input))
}

fn validate_command_name(command: &str) -> Result<()> {
    if command.is_empty()
        || !command
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'_')
    {
        bail!("命令名只能包含 ASCII 字母、数字和下划线");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::generation_tasks::GENERATION_START_ROUTES;
    use reqwest::Method;
    use serde_json::json;

    use super::command_request;

    #[test]
    fn generic_commands_use_the_command_endpoint() {
        let request = command_request("list_chapters", json!({ "projectId": 1 })).unwrap();
        assert_eq!(request.method, Method::POST);
        assert_eq!(request.path, "/commands/list_chapters");
        assert!(request.authenticated);
    }

    #[test]
    fn public_auth_commands_use_the_command_endpoint_without_authentication() {
        for command in [
            "login_user",
            "register_user",
            "refresh_token",
            "oauth2_authorize_url",
            "oauth2_callback",
        ] {
            let request = command_request(command, json!({ "input": {} })).unwrap();
            assert_eq!(request.method, Method::POST, "{command}");
            assert_eq!(request.path, format!("/commands/{command}"), "{command}");
            assert!(!request.authenticated, "{command}");
            assert_eq!(request.body.unwrap(), json!({ "input": {} }), "{command}");
        }
    }

    #[test]
    fn source_sync_uses_the_command_endpoint_and_preserves_runtime_config() {
        let request = command_request(
            "apply_source_sync_decisions",
            json!({
                "input": { "sync_run_id": 1, "decisions": [] },
                "runtime_config_snapshot": { "schema_version": 2 }
            }),
        )
        .unwrap();
        assert_eq!(request.path, "/commands/apply_source_sync_decisions");
        assert_eq!(
            request.body.unwrap()["runtime_config_snapshot"]["schema_version"],
            2
        );
    }

    #[test]
    fn project_list_uses_the_command_endpoint() {
        let request = command_request(
            "list_projects",
            json!({ "page": 2, "pageSize": 10, "projectKind": "text" }),
        )
        .unwrap();
        assert_eq!(request.method, Method::POST);
        assert_eq!(request.path, "/commands/list_projects");
    }

    #[test]
    fn generation_start_adds_client_request_identity() {
        let request = command_request(
            "start_chapter_generation",
            json!({
                "input": { "project_id": 1, "chapter_number": 1 },
                "runtime_config_snapshot": {
                    "schema_version": 2,
                    "image": { "api_key_encrypted": "sk-test" }
                }
            }),
        )
        .unwrap();
        let body = request.body.unwrap();
        let meta = body["meta"].clone();
        assert_eq!(meta["start_request_id"].as_str().unwrap().len(), 36);
        assert!(meta["retry_of_task_id"].is_null());
        assert_eq!(
            body["runtime_config_snapshot"]["image"]["api_key_encrypted"],
            "sk-test"
        );

        let request = command_request(
            "start_chapter_generation",
            json!({ "input": { "meta": { "retry_of_task_id": "old-task" } } }),
        )
        .unwrap();
        let meta = request.body.unwrap()["meta"].clone();
        assert_eq!(meta["retry_of_task_id"], "old-task");
        assert_eq!(meta["start_request_id"].as_str().unwrap().len(), 36);
    }

    #[test]
    fn generation_start_routes_use_the_canonical_contract() {
        for &(command, path) in GENERATION_START_ROUTES {
            let request = command_request(command, json!({ "input": {} })).unwrap();
            assert_eq!(request.method, Method::POST, "{command}");
            assert_eq!(request.path, path, "{command}");
            assert!(request.authenticated, "{command}");
            assert!(
                command_request(command, json!({ "userId": 7, "input": {} })).is_err(),
                "{command} must reject retired envelope fields"
            );
        }
    }
}
