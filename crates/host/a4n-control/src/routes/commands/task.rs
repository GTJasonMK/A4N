mod completion;
mod identity;
mod registration;
mod reporters;

pub(in crate::routes::commands) use completion::{finish_web_task, string_action_result};
pub(in crate::routes::commands) use identity::{sanitize_optional_task_id, sanitize_task_id};
pub(in crate::routes::commands) use registration::register_web_task;
pub(in crate::routes::commands) use reporters::{
    web_fire_and_forget_progress_reporter, web_progress_reporter, web_stream_delta_reporter,
    web_stream_reporter,
};

#[cfg(test)]
mod tests;
