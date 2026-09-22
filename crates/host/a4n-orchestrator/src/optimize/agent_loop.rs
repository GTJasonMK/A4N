mod contracts;
mod decision;
mod observations;
mod request;

pub(in crate::optimize) use contracts::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, AgentLoopDecision, ThinkingObservation,
};
pub(in crate::optimize) use decision::parse_agent_loop_decision_response;
pub(in crate::optimize) use observations::{
    failed_tool_observation, final_issues, final_reply, insert_current_chapter_content,
    run_tool_arguments, thinking_observation, tool_observation,
};
pub(in crate::optimize) use request::{
    agent_loop_request_payload, build_agent_loop_chat_request, build_first_agent_loop_chat_request,
};

#[cfg(test)]
use request::build_agent_loop_step_user_message;

#[cfg(test)]
mod tests;
