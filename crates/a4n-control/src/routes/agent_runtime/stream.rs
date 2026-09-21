struct AgentSseState {
    state: ServerState,
    user_id: i64,
    thread_id: String,
    limit: u32,
    cursor: u64,
    pending: VecDeque<AgentEvent>,
}

fn agent_event_stream(
    state: AgentSseState,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.pending.pop_front() {
                state.cursor = event.sequence;
                return Some((Ok(to_sse_event(&event)), state));
            }

            tokio::time::sleep(EVENT_POLL_INTERVAL).await;
            let input = AgentEventsInput {
                thread_id: state.thread_id.clone(),
                after_sequence: Some(state.cursor),
                limit: Some(state.limit),
            };
            let events =
                match agent_actions::events(state.state.runtime(), state.user_id, input).await {
                    Ok(events) => events,
                    Err(error) => {
                        tracing::warn!(
                            thread_id = %state.thread_id,
                            cursor = state.cursor,
                            error = %error,
                            "Agent SSE durable replay failed; closing stream"
                        );
                        return None;
                    }
                };
            if let Err(error) = validate_event_batch(&state.thread_id, state.cursor, &events) {
                tracing::error!(
                    thread_id = %state.thread_id,
                    cursor = state.cursor,
                    error = %error,
                    "Agent SSE durable event invariant failed; closing stream"
                );
                return None;
            }
            state.pending.extend(events);
        }
    })
}

fn to_sse_event(event: &AgentEvent) -> Event {
    let (id, data) = agent_event_payload(event);
    Event::default().event("agent_event").id(id).data(data)
}

fn agent_event_payload(event: &AgentEvent) -> (String, String) {
    (
        event.sequence.to_string(),
        serde_json::to_string(event).expect("AgentEvent must remain serializable"),
    )
}

fn accepts_event_stream(headers: &HeaderMap) -> Result<bool, ApiError> {
    let Some(value) = headers.get(header::ACCEPT) else {
        return Ok(false);
    };
    let raw = value
        .to_str()
        .map_err(|_| ApiError::bad_request("Accept 头不是有效 ASCII 字符串"))?;
    Ok(raw.split(',').any(|media_range| {
        media_range
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    }))
}
