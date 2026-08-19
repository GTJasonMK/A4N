import React from "react";
import { AlertTriangle, ArrowRight } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import type { Message, MessageOption } from "./shared";

interface ChatBubbleProps {
  message: Message;
  onSendOption: (text: string) => void;
}

const optionTextValue = (option: MessageOption) => option.value || option.label;

const OptionCard: React.FC<{
  option: MessageOption;
  onSend: () => void;
  skeleton?: boolean;
}> = ({ option, onSend, skeleton }) => {
  if (skeleton) {
    return (
      <div className="inspiration-option-card inspiration-option-card-skeleton w-full text-left border border-dashed border-book-accent/35 bg-book-accent/5 rounded-2xl px-3.5 py-3">
        <div className="flex items-start gap-2">
          <div className="mt-0.5 shrink-0 w-3 h-3 rounded-md bg-book-accent/30" />
          <div className="min-w-0 flex-1 space-y-1.5">
            <div className="h-3.5 bg-book-accent/20 rounded-md w-3/5" />
            <div className="h-3 bg-book-accent/15 rounded-md w-full" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={onSend}
      className="inspiration-option-card w-full rounded-2xl border border-book-border/55 bg-book-bg-paper/70 px-3.5 py-3 text-left text-book-text-main transition-[transform,border-color,background-color,box-shadow] duration-150 ease-out hover:-translate-y-px hover:border-book-primary/25 hover:bg-book-bg-paper hover:shadow-[var(--shadow-card)] active:translate-y-0"
    >
      <div className="flex items-start gap-2">
        <ArrowRight size={12} className="mt-0.5 shrink-0 text-book-accent" />
        <div className="min-w-0">
          <div className="text-body font-semibold leading-snug tracking-tight">{option.label}</div>
          {option.description && (
            <div className="mt-1 text-xs leading-relaxed text-book-text-muted">
              {option.description}
            </div>
          )}
          {option.keyElements && option.keyElements.length > 0 && (
            <div className="mt-1.5 flex flex-wrap gap-1">
              {option.keyElements.map((item) => (
                <span
                  key={item}
                  className="rounded-full bg-book-bg-tertiary px-2 py-0.5 text-small text-book-text-muted"
                >
                  {item}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </button>
  );
};

/**
 *  - 用户气泡：主色浅底 + 深字 + 大气泡圆角（右下略尖）
 *  - 助手气泡：轻纸面填充，不形成卡片墙
 *  - 头部 sender 标签：「你」/「创作助手」
 *  - 流式选项：已完成的直接可点，正在生成的显示骨架动画
 */
export const ChatBubble: React.FC<ChatBubbleProps> = ({ message, onSendOption }) => {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";

  if (isSystem) {
    return (
      <div className="self-center text-small text-book-text-muted bg-book-bg-tertiary/80 rounded-full px-3 py-1">
        {message.content}
      </div>
    );
  }

  // 流式期间用 streamingOptions；完成后用 options
  const displayOptions = message.streaming
    ? message.streamingOptions
    : message.options;
  const showPendingSkeleton = message.streaming && message.optionsPending;

  return (
    <div className={`flex min-w-0 flex-col gap-1.5 ${isUser ? "items-end" : "items-start"}`}>
      <div className="min-w-0 max-w-[80%]">
        <div
          className={`text-small font-semibold mb-1 px-1 ${
            isUser ? "text-book-primary" : "text-book-text-muted"
          }`}
          style={{ letterSpacing: "0.04em" }}
        >
          {isUser ? "你" : "创作助手"}
        </div>
        <div
          className={`px-4 py-3 text-body-lg leading-relaxed whitespace-pre-wrap break-words tracking-tight ${
            isUser
              ? "rounded-[18px] rounded-br-md bg-book-primary text-[rgb(var(--color-button-text))] shadow-sm"
              : "rounded-[18px] rounded-bl-md border border-book-border/45 bg-book-bg-paper/65 text-book-text-main"
          }`}
          role={message.streaming ? "status" : undefined}
        >
          {message.consistencyWarning ? (
            <div className="mb-2 flex items-start gap-1.5 border-b border-book-border/50 pb-2 text-xs font-medium text-danger">
              <AlertTriangle size={13} className="mt-0.5 shrink-0" />
              <span>{message.consistencyWarning}</span>
            </div>
          ) : null}
          {message.content}
          {message.streaming && (
            <span
              className={
                message.content
                  ? "inline-block w-[7px] h-[14px] ml-[1px] bg-book-accent animate-pulse align-text-bottom"
                  : "inline-flex items-center gap-1 text-book-text-muted ml-1"
              }
            >
              {!message.content && (
                <>
                  <LoadingSpinner size="sm" />
                  <span>正在整理你的灵感…</span>
                </>
              )}
            </span>
          )}
        </div>
      </div>
      {!isUser && (displayOptions?.length || showPendingSkeleton) && (
        <div className="flex min-w-0 w-full max-w-[520px] flex-col gap-1.5 pl-3 sm:w-[80%] sm:min-w-[240px]">
          {displayOptions?.map((opt, i) => (
            <OptionCard
              key={opt.id || i}
              option={opt}
              onSend={() => onSendOption(optionTextValue(opt))}
            />
          ))}
          {showPendingSkeleton && (
            <OptionCard
              option={{ label: "正在生成选项…" }}
              onSend={() => {}}
              skeleton
            />
          )}
        </div>
      )}
    </div>
  );
};
