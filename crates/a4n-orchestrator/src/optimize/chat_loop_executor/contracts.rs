use a4n_core::models::ai::optimization_session;
use serde_json::Value;

use crate::optimize::{OptimizeLlmStream, OptimizeProgress};

pub(in crate::optimize) struct ChatAgentLoopInput<'a, 'progress, 'stream> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session: &'a optimization_session::Model,
    pub(in crate::optimize) system_prompt: String,
    pub(in crate::optimize) base_user_message: String,
    pub(in crate::optimize) chat_context: Value,
    pub(in crate::optimize) workflow_run_id: i64,
    pub(in crate::optimize) progress: &'a mut Option<OptimizeProgress<'progress>>,
    pub(in crate::optimize) stream: &'a mut Option<OptimizeLlmStream<'stream>>,
}
