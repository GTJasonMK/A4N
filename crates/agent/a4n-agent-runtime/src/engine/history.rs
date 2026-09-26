use a4n_agent_runtime_contract::AgentRunContext;
use a4n_error::AppError;
use a4n_llm_context::{ContextBudgetPlanner, estimate_context_tokens};
use a4n_llm_provider_contract::{ChatMessage, MessageRole, ToolCall};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};

pub(super) const CONTEXT_SUMMARY_SOURCE_TOKENS: usize = 12_000;

pub(super) fn advance_message_watermark(watermark: &AtomicI64, message_id: i64) {
    watermark.fetch_max(message_id, Ordering::Relaxed);
}

#[derive(Debug)]
pub(super) struct ContextFit {
    pub messages: Vec<ChatMessage>,
    pub omitted_history: bool,
    pub input_tokens: usize,
    pub capacity: usize,
    pub output_tokens: usize,
}

#[cfg(test)]
pub(super) fn fit_context(
    messages: Vec<ChatMessage>,
    tools: &[Value],
    context_tokens: usize,
    output_tokens: usize,
    pinned_user_contents: &[&str],
) -> Result<Vec<ChatMessage>, AppError> {
    Ok(fit_context_projection(
        messages,
        tools,
        context_tokens,
        output_tokens,
        pinned_user_contents,
    )?
    .messages)
}

pub(super) fn fit_context_projection(
    messages: Vec<ChatMessage>,
    tools: &[Value],
    context_tokens: usize,
    output_tokens: usize,
    pinned_user_contents: &[&str],
) -> Result<ContextFit, AppError> {
    let mut used = estimate_context_tokens(&serde_json::to_string(tools)?);
    let mut units = Vec::<Vec<ChatMessage>>::new();
    for message in messages {
        if matches!(message.role, MessageRole::Tool) {
            units
                .last_mut()
                .ok_or_else(|| AppError::InvalidArgument("tool result 缺少调用批次".into()))?
                .push(message);
        } else {
            units.push(vec![message]);
        }
    }
    let costs = units
        .iter()
        .map(|unit| serde_json::to_string(unit).map(|text| estimate_context_tokens(&text)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut keep = vec![false; units.len()];
    let mut pin_counts = HashMap::<&str, usize>::new();
    for content in pinned_user_contents {
        *pin_counts.entry(content).or_default() += 1;
    }
    for (index, unit) in units.iter().enumerate().rev() {
        if matches!(unit[0].role, MessageRole::User)
            && let Some(count) = pin_counts.get_mut(unit[0].content.as_str())
            && *count > 0
        {
            keep[index] = true;
            *count -= 1;
        }
    }
    for (index, unit) in units.iter().enumerate() {
        let first = &unit[0];
        if matches!(first.role, MessageRole::System) || index + 1 == units.len() || keep[index] {
            keep[index] = true;
            used = used.saturating_add(costs[index]);
        }
    }
    let plan = ContextBudgetPlanner::default().plan_requirements_adaptive(
        context_tokens,
        output_tokens,
        Vec::new(),
        used,
    )?;
    let capacity = plan.available_input_tokens;
    // This projection measures pressure; callers must summarize any omitted
    // history before sending a model request, never silently discard it.
    for index in (0..units.len()).rev() {
        if !keep[index] && used.saturating_add(costs[index]) <= capacity {
            keep[index] = true;
            used += costs[index];
        }
    }
    let omitted_history = keep.contains(&false);
    let selected = units
        .into_iter()
        .zip(keep)
        .filter(|(_, keep)| *keep)
        .flat_map(|(unit, _)| unit)
        .collect::<Vec<_>>();
    Ok(ContextFit {
        messages: selected,
        omitted_history,
        input_tokens: used,
        capacity,
        output_tokens: plan.output_token_reserve,
    })
}

/// Select a continuous prefix of complete call/result batches. Return the
/// actual final message ID read by the summarizer, not the latest loaded ID.
pub(super) fn summary_source(
    context: &AgentRunContext,
    max_tokens: usize,
) -> Result<Option<(String, i64)>, AppError> {
    let covered = context
        .thread
        .context_summary
        .as_ref()
        .and_then(|value| value.get("covered_through_sequence"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let messages = context
        .messages
        .iter()
        .filter(|message| message.id > covered)
        .collect::<Vec<_>>();
    let observations = messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            message
                .payload_json
                .as_ref()?
                .get("tool_call_id")?
                .as_str()
                .map(|id| (id, index))
        })
        .collect::<HashMap<_, _>>();
    let mut request = json!({
        "previous_summary": context.thread.context_summary,
        "current_request": context.messages.iter()
            .filter(|message| message.turn_id == Some(context.turn.id) && message.role == "user")
            .map(|message| json!({"message_id": message.id, "content": message.content})).collect::<Vec<_>>(),
        "messages": [],
        "covered_through_sequence": messages.last().map_or(covered, |message| message.id),
    });
    let mut source = Vec::new();
    let mut used = estimate_context_tokens(&serde_json::to_string(&request)?);
    let mut index = 0;
    let mut watermark = covered;
    while index < messages.len().saturating_sub(1) {
        let mut end = index;
        let mut cursor = index;
        while cursor <= end {
            if let Some(calls) = messages[cursor]
                .payload_json
                .as_ref()
                .and_then(|value| value.get("tool_calls"))
            {
                let calls: Vec<ToolCall> = serde_json::from_value(calls.clone())?;
                for call in calls {
                    if let Some(result_index) = observations.get(call.id.as_str()) {
                        end = end.max(*result_index);
                    }
                }
            }
            cursor += 1;
        }
        // The newest whole batch always remains verbatim in the main request.
        if end + 1 >= messages.len() {
            break;
        }
        let unit = messages[index..=end].iter().map(|message| {
            Ok(json!({
                "message_id": message.id,
                "message": super::chat_message_from_model(message)?,
                "execution_id": message.payload_json.as_ref().and_then(|value| value.get("execution_id")),
            }))
        }).collect::<Result<Vec<_>, AppError>>()?;
        let cost = estimate_context_tokens(&serde_json::to_string(&unit)?);
        if used.saturating_add(cost) > max_tokens {
            break;
        }
        used += cost;
        source.extend(unit);
        watermark = messages[end].id;
        index = end + 1;
    }
    if source.is_empty() {
        return Ok(None);
    }
    request["messages"] = json!(source);
    request["covered_through_sequence"] = json!(watermark);
    Ok(Some((serde_json::to_string(&request)?, watermark)))
}

/// Only pass messages belonging to the active Run: an old cancelled Turn
/// must never become a new request to execute its unfinished writes.
pub(super) fn pending_tool_batches(
    messages: &[ChatMessage],
) -> Result<Vec<Vec<ToolCall>>, AppError> {
    let completed = messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::Tool))
        .filter_map(|message| message.tool_call_id.as_deref())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut batches = Vec::new();
    for message in messages {
        let Some(calls) = &message.tool_calls else {
            continue;
        };
        let mut pending = Vec::new();
        for call in calls {
            if call.id.trim().is_empty() || !seen.insert(call.id.as_str()) {
                return Err(AppError::InvalidState {
                    current: format!("Agent tool call ID 为空或重复: {}", call.id),
                    operation: "恢复 Agent 调用批次".into(),
                });
            }
            if !completed.contains(call.id.as_str()) {
                pending.push(call.clone());
            }
        }
        if !pending.is_empty() {
            batches.push(pending);
        }
    }
    Ok(batches)
}

/// Inbox messages can arrive between an assistant batch and its observations.
/// Keep each batch atomic on the wire, then present the intervening messages.
/// Historical gaps are reported as unknown; they are never replayed or called
/// successful merely to satisfy a provider's message protocol.
pub(super) fn pair_tool_messages(messages: &[ChatMessage]) -> Result<Vec<ChatMessage>, AppError> {
    let mut paired = Vec::new();
    let mut consumed = HashSet::new();
    for (index, message) in messages.iter().enumerate() {
        if consumed.contains(&index) {
            continue;
        }
        if matches!(message.role, MessageRole::Tool) {
            paired.push(ChatMessage::assistant(serde_json::to_string(&json!({
                "historical_tool_observation": message.content,
                "tool_call_id": message.tool_call_id,
                "note": "对应调用不在保留的历史中，此观察不代表当前任务已执行。"
            }))?));
            continue;
        }
        paired.push(message.clone());
        let Some(calls) = &message.tool_calls else {
            continue;
        };
        let end = messages[index + 1..]
            .iter()
            .position(|next| matches!(next.role, MessageRole::Assistant))
            .map_or(messages.len(), |offset| index + 1 + offset);
        for call in calls {
            let observation = messages[index + 1..end]
                .iter()
                .enumerate()
                .find(|(_, next)| {
                    matches!(next.role, MessageRole::Tool)
                        && next.tool_call_id.as_deref() == Some(call.id.as_str())
                });
            if let Some((offset, observation)) = observation {
                consumed.insert(index + 1 + offset);
                paired.push(observation.clone());
            } else {
                let mut missing = ChatMessage::new(
                    MessageRole::Tool,
                    json!({
                        "error": "历史调用缺少可配对的结果；执行状态未知，不能据此声称成功。",
                        "code": "historical_result_unavailable"
                    })
                    .to_string(),
                );
                missing.tool_call_id = Some(call.id.clone());
                paired.push(missing);
            }
        }
    }
    Ok(paired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    #[test]
    fn message_watermark_advances_monotonically_for_out_of_order_tool_results() {
        let watermark = AtomicI64::new(20);

        advance_message_watermark(&watermark, 30);
        advance_message_watermark(&watermark, 25);

        assert_eq!(watermark.load(Ordering::Relaxed), 30);
    }

    use crate::engine::MAX_OBSERVATION_BYTES;
    use crate::engine::bounded_observation;

    fn batch() -> ChatMessage {
        let mut message = ChatMessage::assistant("读取并修改");
        message.tool_calls = Some(
            ["read", "write"]
                .map(|id| ToolCall {
                    id: id.into(),
                    name: id.into(),
                    arguments: json!({"version": 7}),
                    stream_index: None,
                    provider_metadata: None,
                })
                .to_vec(),
        );
        message
    }

    fn observation(id: &str) -> ChatMessage {
        let mut message = ChatMessage::new(MessageRole::Tool, "{\"ok\":true}");
        message.tool_call_id = Some(id.into());
        message
    }

    #[test]
    fn recovery_keeps_original_arguments_and_only_unanswered_calls() {
        let messages = vec![batch(), observation("read"), ChatMessage::user("继续")];
        let pending = pending_tool_batches(&messages).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].len(), 1);
        assert_eq!(pending[0][0].id, "write");
        assert_eq!(pending[0][0].arguments, json!({"version": 7}));
        let mut completed = messages;
        completed.push(observation("write"));
        assert!(pending_tool_batches(&completed).unwrap().is_empty());
        completed.push(batch());
        assert!(pending_tool_batches(&completed).is_err());
    }

    #[test]
    fn steer_is_preserved_after_the_complete_tool_batch() {
        let messages = vec![
            batch(),
            observation("read"),
            ChatMessage::user("保留原镜头"),
            observation("write"),
        ];
        let paired = pair_tool_messages(&messages).unwrap();
        assert!(paired[0].tool_calls.is_some());
        assert_eq!(paired[1].tool_call_id.as_deref(), Some("read"));
        assert_eq!(paired[2].tool_call_id.as_deref(), Some("write"));
        assert_eq!(paired[3].content, "保留原镜头");
        assert_eq!(paired.len(), 4);
    }

    #[test]
    fn historical_gaps_are_explicit_unknown_results() {
        let paired = pair_tool_messages(&[
            batch(),
            observation("read"),
            ChatMessage::assistant("已取消"),
        ])
        .unwrap();
        assert!(paired[2].content.contains("historical_result_unavailable"));
        assert_eq!(paired[2].tool_call_id.as_deref(), Some("write"));
        let orphan = pair_tool_messages(&[observation("orphan")]).unwrap();
        assert!(matches!(orphan[0].role, MessageRole::Assistant));
        assert!(orphan[0].content.contains("historical_tool_observation"));
    }

    #[test]
    fn large_observation_is_valid_json_with_a_durable_read_handle() {
        let result =
            bounded_observation(73, json!({"content": "字".repeat(MAX_OBSERVATION_BYTES)}))
                .unwrap();
        assert_eq!(result["execution_id"], "73");
        assert_eq!(result["read_tool"], "read_agent_execution_result");
        assert_eq!(result["complete"], false);
        assert!(serde_json::to_vec(&result).unwrap().len() < MAX_OBSERVATION_BYTES);
        assert_eq!(
            bounded_observation(73, json!({"ok": true})).unwrap(),
            json!({"ok": true})
        );
    }

    #[test]
    fn context_budget_retains_current_request_and_entire_tool_batch() {
        let mut old = ChatMessage::assistant("旧观察".repeat(10_000));
        old.tool_calls = None;
        let messages = vec![
            ChatMessage::system("职责"),
            old,
            ChatMessage::user("修改选中镜头"),
            batch(),
            observation("read"),
            observation("write"),
        ];
        let result = fit_context(messages, &[], 8_000, 1_000, &["修改选中镜头"]).unwrap();
        assert!(
            result
                .iter()
                .any(|message| message.content == "修改选中镜头")
        );
        assert_eq!(
            result
                .iter()
                .filter(|message| matches!(message.role, MessageRole::Tool))
                .count(),
            2
        );
        assert!(
            !result
                .iter()
                .any(|message| message.content.starts_with("旧观察"))
        );
        assert!(
            fit_context(
                vec![ChatMessage::user("新目标".repeat(10_000))],
                &[],
                8_000,
                1_000,
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn context_projection_adapts_large_output_to_fit_required_input() {
        let tools = vec![json!({
            "type": "function",
            "function": {
                "name": "read_chapter_visual_plan",
                "parameters": {"type": "object", "properties": {"input": {"type": "object"}}}
            }
        })];
        let projection = fit_context_projection(
            vec![
                ChatMessage::system("Agent system instructions"),
                ChatMessage::user("当前目标：读取并核对时间线"),
            ],
            &tools,
            131_072,
            393_216,
            &[],
        )
        .expect("required input should fit after adapting output reserve");

        assert!(projection.output_tokens > 0);
        assert!(projection.output_tokens < 393_216);
        assert!(projection.input_tokens <= projection.capacity);
    }

    #[test]
    fn removable_history_does_not_require_extra_room_beyond_the_current_request() {
        let request = ChatMessage::user("x".repeat(5000));
        let required = fit_context_projection(vec![request.clone()], &[], 8192, 4096, &[]).unwrap();
        let projected = fit_context_projection(
            vec![ChatMessage::assistant("old".repeat(10000)), request],
            &[],
            8192,
            4096,
            &[],
        )
        .unwrap();
        assert!(projected.omitted_history);
        assert_eq!(projected.input_tokens, required.input_tokens);
        assert_eq!(projected.messages.len(), 1);
    }

    #[test]
    fn context_projection_reports_pressure_without_inventing_messages() {
        let projection = fit_context_projection(
            vec![
                ChatMessage::system("system"),
                ChatMessage::assistant("old observation ".repeat(6_000)),
                ChatMessage::user("new request"),
            ],
            &[],
            16_384,
            8_000,
            &[],
        )
        .expect("history should be projected");

        assert!(projection.omitted_history);
        assert_eq!(projection.messages.len(), 2);
        assert_eq!(projection.messages[0].content, "system");
        assert_eq!(projection.messages[1].content, "new request");
        assert!(projection.input_tokens <= projection.capacity);
    }
}
