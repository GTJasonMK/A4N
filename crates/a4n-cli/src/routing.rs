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
    if let Some(request) = special_request(command, &args)? {
        return Ok(request);
    }
    Ok(JsonRequest {
        method: Method::POST,
        path: format!("/commands/{command}"),
        body: Some(Value::Object(args)),
        authenticated: true,
    })
}

fn special_request(command: &str, args: &Map<String, Value>) -> Result<Option<JsonRequest>> {
    if let Some(path) = generation_start_path(command) {
        return Ok(Some(post(path, generation_input(args)?, true)));
    }
    let request = match command {
        "login_user" => post("/auth/login", input_body(args), false),
        "register_user" => post("/auth/register", input_body(args), false),
        "get_current_user" => get("/auth/me", true),
        "logout_user" => post(
            "/auth/logout",
            json!({ "session_id": required(args, "sessionId")? }),
            true,
        ),
        "refresh_token" => post("/auth/refresh", input_body(args), false),
        "oauth2_authorize_url" => post(
            "/commands/oauth2_authorize_url",
            Value::Object(args.clone()),
            false,
        ),
        "oauth2_callback" => post(
            "/commands/oauth2_callback",
            Value::Object(args.clone()),
            false,
        ),
        "list_projects" => get(
            &query_path(
                "/projects",
                args,
                &[
                    ("page", "page"),
                    ("page_size", "pageSize"),
                    ("project_kind", "projectKind"),
                    ("root_project_id", "rootProjectId"),
                    ("access_scope", "accessScope"),
                ],
            )?,
            true,
        ),
        "get_project" => get(
            &format!("/projects/{}", path_segment(required(args, "projectId")?)?),
            true,
        ),
        "create_project" => post("/projects", input_body(args), true),
        "create_derived_project" => post("/projects/derived", input_body(args), true),
        "create_freeform_video_project" => post("/projects/freeform-video", input_body(args), true),
        "list_source_snapshot_chapters" => get(
            &format!(
                "/projects/{}/source-snapshot-chapters",
                path_segment(required(args, "projectId")?)?
            ),
            true,
        ),
        "list_all_users" => get("/admin/users", true),
        "create_user" => post("/admin/users", input_body(args), true),
        "list_invite_codes" => get("/admin/invite-codes", true),
        "create_invite_code" => post("/admin/invite-codes", input_body(args), true),
        "update_user_role" => post(
            &format!(
                "/admin/users/{}/role",
                path_segment(required(args, "userId")?)?
            ),
            input_body(args),
            true,
        ),
        "disable_user" => post(
            &format!(
                "/admin/users/{}/disable",
                path_segment(required(args, "userId")?)?
            ),
            json!({}),
            true,
        ),
        _ => return Ok(None),
    };
    Ok(Some(request))
}

fn get(path: &str, authenticated: bool) -> JsonRequest {
    JsonRequest {
        method: Method::GET,
        path: path.to_string(),
        body: None,
        authenticated,
    }
}

fn post(path: &str, body: Value, authenticated: bool) -> JsonRequest {
    JsonRequest {
        method: Method::POST,
        path: path.to_string(),
        body: Some(body),
        authenticated,
    }
}

fn input_body(args: &Map<String, Value>) -> Value {
    args.get("input").cloned().unwrap_or_else(|| json!({}))
}

fn generation_input(args: &Map<String, Value>) -> Result<Value> {
    let Value::Object(mut input) = input_body(args) else {
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

fn required<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a Value> {
    args.get(key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| anyhow!("缺少参数: {key}"))
}

fn query_path(base: &str, args: &Map<String, Value>, fields: &[(&str, &str)]) -> Result<String> {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (query_key, arg_key) in fields {
        if let Some(value) = args.get(*arg_key).filter(|value| !value.is_null()) {
            serializer.append_pair(query_key, &scalar_text(value)?);
        }
    }
    let query = serializer.finish();
    Ok(if query.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{query}")
    })
}

fn path_segment(value: &Value) -> Result<String> {
    Ok(url::form_urlencoded::byte_serialize(scalar_text(value)?.as_bytes()).collect())
}

fn scalar_text(value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => bail!("URL 参数必须是 string、number 或 boolean"),
    }
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
    fn source_sync_uses_the_command_endpoint_and_preserves_runtime_config() {
        let request = command_request(
            "apply_source_sync_decisions",
            json!({
                "input": { "sync_run_id": 1, "decisions": [] },
                "runtime_config_snapshot": { "schema_version": 1 }
            }),
        )
        .unwrap();
        assert_eq!(request.path, "/commands/apply_source_sync_decisions");
        assert_eq!(
            request.body.unwrap()["runtime_config_snapshot"]["schema_version"],
            1
        );
    }

    #[test]
    fn project_list_uses_the_same_query_mapping_as_web() {
        let request = command_request(
            "list_projects",
            json!({ "page": 2, "pageSize": 10, "projectKind": "text" }),
        )
        .unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.path,
            "/projects?page=2&page_size=10&project_kind=text"
        );
    }

    #[test]
    fn generation_start_adds_client_request_identity() {
        let request = command_request(
            "start_chapter_generation",
            json!({
                "input": { "project_id": 1, "chapter_number": 1 },
                "runtime_config_snapshot": {
                    "schema_version": 1,
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
        }
    }
}
