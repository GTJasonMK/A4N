import React from "react";
import { ListChecks } from "lucide-react";

export const QueueInfoCard: React.FC = () => (
  <div className="pyqt-card p-4 text-xs text-book-text-sub leading-relaxed">
    <div className="font-semibold text-book-text-main mb-2 inline-flex items-center gap-1.5">
      <ListChecks size={14} /> 关于任务队列
    </div>
    <p className="mb-1.5">
      A4N 后端使用基于
      <code className="mx-1 px-1.5 py-0.5 bg-book-bg-tertiary rounded text-[11px] font-mono">
        tokio::Semaphore
      </code>
      的优先级调度。
    </p>
    <p className="mb-1.5">
      <span className="pyqt-pill mr-1">High</span>
      用户交互类任务（章节生成、灵感对话）
      <span className="pyqt-pill mx-1 ml-3">Medium</span>
      一般工作流
      <span className="pyqt-pill mx-1 ml-3">Low</span>
      后台任务（RAG 入库、导入分析）
    </p>
    <p className="text-book-text-muted">
      任务记录写入
      <code className="mx-1 px-1.5 py-0.5 bg-book-bg-tertiary rounded text-[11px] font-mono">
        task_queue
      </code>
      表，设置页只提供运行中/历史任务观测与取消入口。
    </p>
  </div>
);
