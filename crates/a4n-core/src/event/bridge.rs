//! 内部事件总线
//!
//! 服务间通过事件通信实现松耦合。基于 tokio::sync::broadcast 实现发布/订阅模式。

use serde::Serialize;

/// 内部事件类型
///
/// 服务通过内部事件总线发布事件，当前订阅者记录这些跨服务事件日志。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum InternalEvent {
    /// 项目状态变更（记录历史、通知 UI）
    ProjectStatusChanged {
        project_id: i64,
        from: String,
        to: String,
        timestamp: chrono::NaiveDateTime,
    },
    /// 蓝图已更新
    BlueprintUpdated { project_id: i64, version: String },
}

/// 基于 `tokio::sync::broadcast` 的事件总线实现
pub struct BroadcastEventBus {
    tx: tokio::sync::broadcast::Sender<InternalEvent>,
}

impl BroadcastEventBus {
    /// 创建事件总线
    ///
    /// `capacity` 指定通道缓冲区大小，超出时最旧的消息会被丢弃。
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(capacity);
        Self { tx }
    }

    /// 发布事件到所有订阅者。
    pub fn publish(&self, event: InternalEvent) -> Result<usize, crate::error::AppError> {
        match self.tx.send(event) {
            Ok(count) => Ok(count),
            Err(_) if self.tx.receiver_count() == 0 => Ok(0),
            Err(e) => Err(crate::error::AppError::Internal(e.into())),
        }
    }

    /// 订阅事件流，返回独立的 Receiver。
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
            from: "draft".to_string(),
            to: "blueprint_ready".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
        }
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_successful_no_delivery() {
        let bus = BroadcastEventBus::new(16);

        let delivered = bus.publish(status_event()).unwrap();

        assert_eq!(delivered, 0);
    }

    #[tokio::test]
    async fn publish_reports_subscriber_count_when_receivers_exist() {
        let bus = BroadcastEventBus::new(16);
        let mut rx = bus.subscribe();

        let delivered = bus.publish(status_event()).unwrap();
        let event = rx.recv().await.unwrap();

        assert_eq!(delivered, 1);
        assert!(matches!(event, InternalEvent::ProjectStatusChanged { .. }));
    }
}
