import React from "react";
import { ListChecks } from "lucide-react";

export const QueueInfoCard: React.FC = () => (
  <details className="group rounded-xl border border-book-border/40 bg-book-bg-paper/55 text-xs text-book-text-sub">
    <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2.5 font-semibold text-book-text-main">
      <ListChecks size={14} className="text-book-primary" />
      队列说明
      <span className="ml-auto text-small font-normal text-book-text-muted group-open:hidden">展开</span>
      <span className="ml-auto hidden text-small font-normal text-book-text-muted group-open:inline">收起</span>
    </summary>
    <div className="space-y-2 border-t border-book-border/30 px-3 py-3 leading-relaxed">
      <p>任务会根据优先级自动排队，并在可用的处理槽位中依次执行。</p>
      <p className="text-book-text-muted">
        高优先级用于当前操作，中优先级用于一般工作流，低优先级用于知识库入库和导入分析等后台任务。
      </p>
    </div>
  </details>
);
