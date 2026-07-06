import React from "react";
import { CheckCircle2, CircleDashed, HelpCircle, Loader2 } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";

interface InspirationGuidePanelProps {
  progressSummaryLines: string[];
  nextQuestion?: string;
  userTurns: number;
  isReady: boolean;
  sending: boolean;
  generating: boolean;
  onFocusConversation: () => void;
}

export const InspirationGuidePanel: React.FC<InspirationGuidePanelProps> = ({
  progressSummaryLines,
  nextQuestion,
  userTurns,
  isReady,
  sending,
  generating,
  onFocusConversation,
}) => {
  const StatusIcon = isReady ? CheckCircle2 : CircleDashed;

  return (
    <aside className="hidden lg:flex h-full min-h-0 w-[320px] shrink-0 border-l border-book-border bg-book-bg-paper">
      <div className="flex h-full min-h-0 w-full flex-col p-3">
        <div className="flex items-center justify-between gap-2 border-b border-book-border pb-2">
          <div className="min-w-0">
            <div className="text-[10px] font-semibold uppercase text-book-text-muted">
              引导区
            </div>
            <div className="mt-0.5 flex items-center gap-1.5 text-xs text-book-text-main">
              <StatusIcon size={13} className={isReady ? "text-success" : "text-book-text-muted"} />
              <span>{isReady ? "可以生成蓝图" : "继续补充设定"}</span>
            </div>
          </div>
          <BookButton variant="secondary" size="sm" onClick={onFocusConversation}>
            去回答
          </BookButton>
        </div>

        <div className="mt-3 grid grid-cols-2 gap-2 text-[11px]">
          <div className="rounded-sm border border-book-border bg-book-bg px-2 py-1.5">
            <div className="text-book-text-muted">对话轮次</div>
            <div className="mt-0.5 font-mono text-book-text-main">{userTurns}</div>
          </div>
          <div className="rounded-sm border border-book-border bg-book-bg px-2 py-1.5">
            <div className="text-book-text-muted">状态</div>
            <div className="mt-0.5 text-book-text-main">
              {generating ? "生成中" : sending ? "整理中" : isReady ? "就绪" : "收集中"}
            </div>
          </div>
        </div>

        <div className="mt-3 min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          <section>
            <div className="mb-2 text-[10px] font-semibold uppercase text-book-text-muted">
              当前进度
            </div>
            {progressSummaryLines.length > 0 ? (
              <ul className="space-y-1.5">
                {progressSummaryLines.slice(0, 8).map((line, index) => (
                  <li
                    key={`${index}-${line}`}
                    className="flex gap-2 rounded-sm border border-book-border bg-book-bg px-2 py-1.5 text-xs leading-relaxed text-book-text-main"
                  >
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-book-accent" />
                    <span className="min-w-0 break-words">{line}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="rounded-sm border border-dashed border-book-border bg-book-bg px-2.5 py-3 text-xs leading-relaxed text-book-text-muted">
                {sending ? (
                  <span className="inline-flex items-center gap-1.5">
                    <Loader2 size={12} className="animate-spin" />
                    AI 正在整理当前进度…
                  </span>
                ) : (
                  "每轮对话后会自动汇总已确认的故事信息。"
                )}
              </div>
            )}
          </section>

          <section className="mt-4">
            <div className="mb-2 text-[10px] font-semibold uppercase text-book-text-muted">
              下一步
            </div>
            <div className="rounded-sm border border-book-border bg-book-bg px-2.5 py-2">
              <div className="flex items-start gap-2 text-xs leading-relaxed text-book-text-main">
                <HelpCircle size={13} className="mt-0.5 shrink-0 text-book-accent" />
                <span className="min-w-0 break-words">
                  {nextQuestion || (isReady ? "可以先生成首版蓝图，再回到详情页继续细化。" : "继续回答 AI 的追问，把核心冲突、人物动机和世界规则说清楚。")}
                </span>
              </div>
            </div>
          </section>
        </div>
      </div>
    </aside>
  );
};
