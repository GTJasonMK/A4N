use a4n_core::{
    error::AppError,
    event::bridge::{EventBus, InternalEvent},
    state::status::ProjectStatus,
};
use chrono::Utc;

use crate::OrchestratorContext;

use super::super::{FreeformVideoGeneratedDataResetResult, project_reset_error_to_app_error};

impl OrchestratorContext {
    /// 重置项目生成态，并在事务完成后发布唯一的状态变更事件。
}
