import React from "react";
import type { QueueStats } from "../../../api/task";
import { MetricCard } from "./MetricCard";

interface QueueStatsGridProps {
  stats: QueueStats | null;
}

export const QueueStatsGrid: React.FC<QueueStatsGridProps> = ({ stats }) => (
  <>
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-5">
      <MetricCard
        label="LLM 等待"
        value={stats?.llm_queue.waiting}
        tone="muted"
        loading={!stats}
      />
      <MetricCard
        label="LLM 执行中"
        value={stats?.llm_queue.running}
        tone="active"
        loading={!stats}
      />
      <MetricCard
        label="图片 等待"
        value={stats?.image_queue.waiting}
        tone="muted"
        loading={!stats}
      />
      <MetricCard
        label="图片 执行中"
        value={stats?.image_queue.running}
        tone="active"
        loading={!stats}
      />
      <MetricCard
        label="视频 等待"
        value={stats?.video_queue.waiting}
        tone="muted"
        loading={!stats}
      />
      <MetricCard
        label="视频 执行中"
        value={stats?.video_queue.running}
        tone="active"
        loading={!stats}
      />
      <MetricCard
        label="Embedding 等待"
        value={stats?.embedding_queue.waiting}
        tone="muted"
        loading={!stats}
      />
      <MetricCard
        label="Embedding 执行中"
        value={stats?.embedding_queue.running}
        tone="active"
        loading={!stats}
      />
      <MetricCard
        label="后台 等待"
        value={stats?.background_queue.waiting}
        tone="muted"
        loading={!stats}
      />
      <MetricCard
        label="后台 执行中"
        value={stats?.background_queue.running}
        tone="active"
        loading={!stats}
      />
    </div>

    <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
      <MetricCard
        label="高优先级 等待"
        value={stats?.high_priority_waiting}
        tone="danger"
        loading={!stats}
      />
      <MetricCard
        label="中优先级 等待"
        value={stats?.medium_priority_waiting}
        tone="active"
        loading={!stats}
      />
      <MetricCard
        label="低优先级 等待"
        value={stats?.low_priority_waiting}
        tone="muted"
        loading={!stats}
      />
    </div>
  </>
);
