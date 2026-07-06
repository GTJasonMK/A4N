import React from "react";
import { ArrowRight, Loader2 } from "lucide-react";
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
      <div className="inspiration-option-card inspiration-option-card-skeleton w-full text-left border border-dashed border-book-accent/40 bg-book-accent/5 rounded-sm px-2.5 py-2">
        <div className="flex items-start gap-1.5">
          <div className="mt-0.5 shrink-0 w-[11px] h-[11px] rounded-sm bg-book-accent/30" />
          <div className="min-w-0 flex-1 space-y-1.5">
            <div className="h-3 bg-book-accent/20 rounded-sm w-3/5" />
            <div className="h-2.5 bg-book-accent/15 rounded-sm w-full" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={onSend}
      className="inspiration-option-card w-full text-left border border-book-border bg-book-bg-tertiary/40 hover:bg-book-bg-tertiary text-book-text-main rounded-sm px-2.5 py-2 transition-colors"
    >
      <div className="flex items-start gap-1.5">
        <ArrowRight size={11} className="mt-0.5 shrink-0 text-book-accent" />
        <div className="min-w-0">
          <div className="text-[11px] font-semibold leading-snug">{option.label}</div>
          {option.description && (
            <div className="mt-1 text-[10px] leading-relaxed text-book-text-muted">
              {option.description}
            </div>
          )}
          {option.keyElements && option.keyElements.length > 0 && (
            <div className="mt-1.5 flex flex-wrap gap-1">
              {option.keyElements.map((item) => (
                <span
                  key={item}
                  className="rounded-sm border border-book-border px-1.5 py-0.5 text-[10px] text-book-text-muted"
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
 *  - 用户气泡：主色实色 + 白字 + 右上角无圆角
 *  - AI 气泡：极淡背景 + 左侧 3px 装饰线（书香 accent 色），无方框
 *  - 头部 sender 标签：「你」/「AI 助手」+ 12px 加粗 + letter-spacing
 *  - 流式选项：已完成的直接可点，正在生成的显示骨架动画
 */
export const ChatBubble: React.FC<ChatBubbleProps> = ({ message, onSendOption }) => {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";

  if (isSystem) {
    return (
      <div className="self-center text-[10px] text-book-text-muted bg-book-bg-tertiary border border-book-border rounded-sm px-2 py-1">
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
    <div className={`flex flex-col gap-1 ${isUser ? "items-end" : "items-start"}`}>
      <div className={`max-w-[80%] ${isUser ? "" : "pl-3 border-l-[3px] border-book-accent"}`}>
        <div
          className={`text-[10px] font-bold mb-0.5 ${
            isUser ? "text-book-accent" : "text-book-text-muted"
          }`}
          style={{ letterSpacing: "0.06em" }}
        >
          {isUser ? "你" : "AI 助手"}
        </div>
        <div
          className={`px-3 py-2 text-xs leading-relaxed whitespace-pre-wrap break-words ${
            isUser
              ? "rounded-sm border border-book-primary/30 bg-book-primary/10 text-book-text-main"
              : "bg-book-bg-tertiary/30 text-book-text-main"
          }`}
          style={
            isUser
              ? { borderTopRightRadius: 0 }
              : undefined
          }
        >
          {message.content}
          {message.streaming && (
            <span
              className={
                message.content
                  ? "inline-block w-[7px] h-[14px] ml-[1px] bg-book-accent animate-pulse align-text-bottom"
                  : "inline-flex items-center gap-1 text-book-text-muted ml-1"
              }
            >
              {!message.content && <Loader2 size={10} className="animate-spin" />}
            </span>
          )}
        </div>
      </div>
      {!isUser && (displayOptions?.length || showPendingSkeleton) && (
        <div className="flex w-full max-w-[520px] flex-col gap-1.5 pl-3 sm:w-[80%] sm:min-w-[240px]">
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
