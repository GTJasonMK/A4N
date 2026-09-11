//! Native Tauri transport for the durable Agent control plane.
//!
//! The web host exposes the same operations as REST routes.  Desktop clients
//! call this small dispatcher so Agent state does not depend on a local HTTP
//! server being started or on a second persistence implementation.

use a4n_api_contract::agent::{
    AgentEventsInput, AgentMemoryInput, AgentMemoryScopeKind, ArchiveAgentThreadInput,
    ControlAgentRunInput, ControlChildAgentInput, CreateAgentThreadInput, DescribeAgentToolInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, ListAgentMessagesInput,
    ListAgentThreadsInput, ResolveAgentApprovalInput, RetractAgentMemoryInput,
    SearchAgentToolsInput, SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput,
    UpdateAgentThreadInput,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::agent_runtime as agent_actions;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tauri::State;

fn decode<T: DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|error| format!("Agent 参数无效: {error}"))
}

fn encode<T: serde::Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| format!("Agent 响应序列化失败: {error}"))
}

/// Invoke one durable Agent operation from the desktop transport.
///
/// `caller_user_id` is supplied by the authenticated frontend session.  All
/// mutating operations still perform ownership and scope checks in the
/// Orchestrator; this command is only a transport adapter.
#[tauri::command]
pub async fn invoke_agent_command(
    state: State<'_, AppState>,
    command: String,
    args: Value,
    caller_user_id: i64,
) -> Result<Value, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("Agent command 不能为空".into());
    }

    let result = match command {
        "list_agent_threads" => {
            let input = decode::<ListAgentThreadsInput>(args)?;
            agent_actions::list_threads(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "ensure_main_agent_thread" => {
            let input = decode::<EnsureMainAgentThreadInput>(args)?;
            agent_actions::ensure_main_thread(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "create_agent_thread" => {
            let input = decode::<CreateAgentThreadInput>(args)?;
            agent_actions::create_thread(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "fork_agent_thread" => {
            let input = decode::<ForkAgentThreadInput>(args)?;
            agent_actions::fork_thread(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "update_agent_thread" => {
            let input = decode::<UpdateAgentThreadInput>(args)?;
            agent_actions::update_thread(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "archive_agent_thread" => {
            let input = decode::<ArchiveAgentThreadInput>(args)?;
            agent_actions::archive_thread(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "get_agent_thread" => {
            let thread_id = decode::<ThreadIdArgs>(args)?.thread_id;
            agent_actions::show_thread(&state, caller_user_id, &thread_id)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "list_agent_messages" => {
            let input = decode::<ListAgentMessagesInput>(args)?;
            agent_actions::list_messages(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "get_agent_run" => {
            let run_id = decode::<RunIdArgs>(args)?.run_id;
            agent_actions::show_run(&state, caller_user_id, &run_id)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "send_agent_message" => {
            let input = decode::<SendAgentMessageInput>(args)?;
            agent_actions::send_message(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "control_agent_run" => {
            let input = decode::<ControlAgentRunInput>(args)?;
            agent_actions::control_run(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "send_agent_message_to_child" => {
            let input = decode::<SendAgentMessageToChildInput>(args)?;
            agent_actions::send_message_to_child(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "control_child_agent" => {
            let input = decode::<ControlChildAgentInput>(args)?;
            agent_actions::control_child_agent(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "spawn_agent" => {
            let input = decode::<SpawnAgentInput>(args)?;
            agent_actions::spawn_agent(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "list_agent_events" => {
            let input = decode::<AgentEventsInput>(args)?;
            agent_actions::events(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "search_agent_tools" => {
            let input = decode::<SearchAgentToolsInput>(args)?;
            agent_actions::search_tools(&state, input)
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "describe_agent_tool" => {
            let input = decode::<DescribeAgentToolInput>(args)?;
            agent_actions::describe_tool(&state, input)
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "resolve_agent_approval" => {
            let input = decode::<ResolveAgentApprovalInput>(args)?;
            agent_actions::resolve_approval(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "list_agent_memories" => {
            let input = decode::<ListAgentMemoriesArgs>(args)?;
            agent_actions::list_memories(&state, caller_user_id, input.scope_kind, input.scope_id)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "create_agent_memory" => {
            let input = decode::<AgentMemoryInput>(args)?;
            agent_actions::create_memory(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        "retract_agent_memory" => {
            let input = decode::<RetractAgentMemoryInput>(args)?;
            agent_actions::retract_memory(&state, caller_user_id, input)
                .await
                .map_err(|error| error.to_string())
                .and_then(encode)
        }
        _ => return Err(format!("未知 Agent command: {command}")),
    };

    result.map_err(|error| error.to_string())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ThreadIdArgs {
    thread_id: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RunIdArgs {
    run_id: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ListAgentMemoriesArgs {
    #[serde(default)]
    scope_kind: Option<AgentMemoryScopeKind>,
    #[serde(default)]
    scope_id: Option<String>,
}
