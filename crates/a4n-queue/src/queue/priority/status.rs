use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
    Interrupted { reason: Option<String> },
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed { .. } => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted { .. } => "interrupted",
        }
    }

    pub fn parse_storage(value: &str, error_message: Option<&str>) -> Result<Self, AppError> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted {
                reason: error_message.map(ToString::to_string),
            }),
            "failed" => {
                let error = error_message.ok_or_else(|| {
                    AppError::InvalidArgument("失败任务缺少 error_message".to_string())
                })?;
                Ok(Self::Failed {
                    error: error.to_string(),
                })
            }
            _ => Err(AppError::InvalidArgument(format!("未知任务状态: {value}"))),
        }
    }
}
