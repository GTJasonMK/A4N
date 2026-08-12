use a4n_api_contract::task::{QueueSlotStatsSummary, QueueStatsSummary};
use a4n_queue::priority::{QueueSlotStats, QueueStats};

pub(in crate::actions::task_records) fn queue_stats_summary(
    stats: &QueueStats,
) -> QueueStatsSummary {
    QueueStatsSummary {
        llm_queue: queue_slot_stats_summary(&stats.llm_queue),
        image_queue: queue_slot_stats_summary(&stats.image_queue),
        video_queue: queue_slot_stats_summary(&stats.video_queue),
        embedding_queue: queue_slot_stats_summary(&stats.embedding_queue),
        background_queue: queue_slot_stats_summary(&stats.background_queue),
        high_priority_waiting: stats.high_priority_waiting,
        medium_priority_waiting: stats.medium_priority_waiting,
        low_priority_waiting: stats.low_priority_waiting,
    }
}

fn queue_slot_stats_summary(stats: &QueueSlotStats) -> QueueSlotStatsSummary {
    QueueSlotStatsSummary {
        running: stats.running,
        waiting: stats.waiting,
    }
}
