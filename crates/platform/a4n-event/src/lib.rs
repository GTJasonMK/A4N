//! Host 内部事件合同与进程内广播实现。
//!
//! 事件只描述跨模块通知，不拥有数据库写入、业务编排或 UI 状态。

use a4n_domain::project::ProjectStatus;
use serde::Serialize;

/// 可被 Host 内部订阅者消费的事件。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum InternalEvent {
    /// 项目状态变更通知。
    ProjectStatusChanged {
        project_id: i64,
        from: ProjectStatus,
        to: ProjectStatus,
        timestamp: chrono::NaiveDateTime,
    },
    /// 蓝图已更新。
    BlueprintUpdated { project_id: i64, version: String },
}

/// 基于 `tokio::sync::broadcast` 的进程内事件总线。
pub struct BroadcastEventBus {
    tx: tokio::sync::broadcast::Sender<InternalEvent>,
}

impl BroadcastEventBus {
    /// 创建事件总线。容量耗尽时由 Tokio 丢弃最旧事件。
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(capacity);
        Self { tx }
    }

    /// 发布事件到所有订阅者。
    pub fn publish(&self, event: InternalEvent) -> Result<usize, a4n_error::AppError> {
        match self.tx.send(event) {
            Ok(count) => Ok(count),
            Err(_) if self.tx.receiver_count() == 0 => Ok(0),
            Err(error) => Err(a4n_error::AppError::Internal(error.into())),
        }
    }

    /// 订阅事件流。
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<InternalEvent> {
        self.tx.subscribe()
    }
}

impl std::fmt::Debug for BroadcastEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroadcastEventBus")
            .field("subscriber_count", &self.tx.receiver_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_event() -> InternalEvent {
        InternalEvent::ProjectStatusChanged {
            project_id: 1,
            from: ProjectStatus::Draft,
            to: ProjectStatus::BlueprintReady,
            timestamp: chrono::Utc::now().naive_utc(),
        }
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_successful() {
        let bus = BroadcastEventBus::new(16);
        assert_eq!(bus.publish(status_event()).unwrap(), 0);
    }

    #[tokio::test]
    async fn publish_delivers_to_subscribers() {
        let bus = BroadcastEventBus::new(16);
        let mut receiver = bus.subscribe();

        assert_eq!(bus.publish(status_event()).unwrap(), 1);
        assert!(matches!(
            receiver.recv().await.unwrap(),
            InternalEvent::ProjectStatusChanged { .. }
        ));
    }
}
