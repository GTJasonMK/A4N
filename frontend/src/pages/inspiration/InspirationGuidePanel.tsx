import React from "react";
import { HelpCircle } from "lucide-react";

interface InspirationGuidePanelProps {
  progressSummaryLines: string[];
  nextQuestion?: string;
  isReady: boolean;
  sending: boolean;
}

export const InspirationGuidePanel: React.FC<InspirationGuidePanelProps> = ({
  progressSummaryLines,
  nextQuestion,
  isReady,
  sending,
}) => (
  <aside className="hidden lg:flex h-full min-h-0 w-[320px] shrink-0 border-l border-book-border bg-book-bg-paper">
    <div className="flex h-full min-h-0 w-full flex-col p-3">
      <header className="border-b border-book-border/55 px-1 pb-3 pt-1">
        <h2 className="text-sm font-semibold text-book-text-main">创作进度</h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          已确认的信息会随对话自动整理
        </p>
      </header>

      <div className="mt-3 min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        <InspirationProgressContent
          progressSummaryLines={progressSummaryLines}
          nextQuestion={nextQuestion}
          isReady={isReady}
          sending={sending}
        />
      </div>
    </div>
  </aside>
);

export const InspirationProgressContent: React.FC<InspirationGuidePanelProps> = ({
  progressSummaryLines,
  nextQuestion,
  isReady,
  sending,
}) => (
  <div>
    <section>
      {progressSummaryLines.length > 0 ? (
        <ul className="divide-y divide-book-border/45 border-y border-book-border/45 px-1">
          {progressSummaryLines.slice(0, 8).map((line, index) => (
            <li
              key={`${index}-${line}`}
              className="flex gap-2 py-2 text-xs leading-relaxed text-book-text-main"
            >
              <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-book-accent" />
              <span className="min-w-0 break-words">{line}</span>
            </li>
          ))}
        </ul>
      ) : (
        <div className="rounded-[12px] border border-dashed border-book-border bg-book-bg px-3 py-3 text-xs leading-relaxed text-book-text-muted">
          {sending
            ? "本轮回复完成后，将在这里汇总故事信息。"
            : "每轮对话后会自动汇总已确认的故事信息。"}
        </div>
      )}
    </section>

    <section className="mt-4 border-t border-book-border/45 pt-3">
      <div className="mb-2 text-caption font-semibold text-book-text-muted">
        下一步
      </div>
      <div className="px-1 py-1">
        <div className="flex items-start gap-2 text-xs leading-relaxed text-book-text-main">
          <HelpCircle size={13} className="mt-0.5 shrink-0 text-book-accent" />
          <span className="min-w-0 break-words">
            {sending
              ? "等待本轮回复完成后查看下一步建议。"
              : nextQuestion ||
                (isReady
                  ? "可以先生成首版蓝图，再回到详情页继续细化。"
                  : "继续回答创作助手的追问，把核心冲突、人物动机和世界规则说清楚。")}
          </span>
        </div>
      </div>
    </section>
  </div>
);
