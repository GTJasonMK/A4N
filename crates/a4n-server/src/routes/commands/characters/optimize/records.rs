use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, optimize_actions, strict_command_input, to_value,
};
use super::adapters::optimize_web_input_command;

optimize_web_input_command!(
    list_optimize_sessions,
    optimize_actions::ListOptimizeSessionsInput,
    optimize_actions::list_optimize_sessions,
);
optimize_web_input_command!(
    get_optimize_session,
    optimize_actions::OptimizeSessionInput,
    optimize_actions::get_optimize_session,
);
optimize_web_input_command!(
    list_optimize_turns,
    optimize_actions::OptimizeSessionInput,
    optimize_actions::list_optimize_turns,
);
optimize_web_input_command!(
    list_optimize_patch_sets,
    optimize_actions::OptimizeSessionInput,
    optimize_actions::list_optimize_patch_sets,
);
optimize_web_input_command!(
    discard_pending_optimize_patch_sets,
    optimize_actions::OptimizeSessionInput,
    optimize_actions::discard_pending_optimize_patch_sets,
);
optimize_web_input_command!(
    delete_optimize_session,
    optimize_actions::OptimizeSessionInput,
    optimize_actions::delete_optimize_session,
);
optimize_web_input_command!(
    record_optimize_agent_event,
    optimize_actions::RecordOptimizeAgentEventInput,
    optimize_actions::record_optimize_agent_event,
);
