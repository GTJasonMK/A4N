import React from "react";
import { AlertTriangle, ArrowRight } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { usePacedStreamText } from "../../hooks/usePacedStreamText";
import { MarkdownText } from "../../components/ui/MarkdownText";
import type { Message, MessageOption } from "./shared";
import { getStreamingResponsePreview } from "./responseParser";
import { useI18n } from "../../i18n";

interface ChatBubbleProps {
  message: Message;
  onSendOption: (text: string) => void;
  canSelectOptions: boolean;
}

const OptionCard: React.FC<{
  option: MessageOption;
  onSend: () => void;
  interactive: boolean;
}> = ({ option, onSend, interactive }) => {
  return (
    <button
      type="button"
      onClick={onSend}
      disabled={!interactive}
      className={`inspiration-option-card w-full rounded-[8px] border px-3.5 py-3 text-left text-book-text-main ${
        interactive
          ? "border-book-primary/45 bg-book-bg-paper transition-[border-color,background-color,box-shadow] duration-150 ease-out hover:border-book-primary/70 hover:bg-book-bg-paper hover:shadow-[var(--shadow-card)]"
          : "cursor-not-allowed border-book-border/35 bg-book-bg-tertiary/35 opacity-55"
      }`}
    >
      <div className="flex items-start gap-2">
        <ArrowRight
          size={12}
          className={`mt-0.5 shrink-0 ${interactive ? "text-book-accent" : "text-book-text-muted"}`}
        />
        <div className="min-w-0">
          <div className="text-body font-semibold leading-snug">{option.label}</div>
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
 *  - 选项卡片：只有当前回复的已完成选项可点，历史选项保留展示但不可再次提交
 */
export const ChatBubble: React.FC<ChatBubbleProps> = ({
  message,
  onSendOption,
  canSelectOptions,
}) => {
  const { t } = useI18n();
  const isUser = message.role === "user";
  const isSystem = message.role === "system";
  const streamSource = message.rawContent ?? message.content;
  const pacedStream = usePacedStreamText(
    streamSource,
    Boolean(message.streaming),
    message.rawContent != null,
  );
  const streamingPreview = message.rawContent != null && pacedStream.displayedText
    ? getStreamingResponsePreview(pacedStream.displayedText)
    : null;
  const displayedContent = message.rawContent != null
    ? streamingPreview?.content ?? ""
    : message.content;
  const streamDisplaying = Boolean(message.streaming || pacedStream.hasPendingText);

  if (isSystem) {
    return (
      <div className="self-center text-small text-book-text-muted bg-book-bg-tertiary/80 rounded-full px-3 py-1">
        {message.content}
      </div>
    );
  }

  // 流式期间用 streamingOptions；完成后用 options
  const displayOptions = streamDisplaying
    ? streamingPreview?.options
    : message.options;
  const showPendingSkeleton = Boolean(message.streaming && streamingPreview?.optionsPending);

  return (
    <div className={`flex min-w-0 flex-col gap-1.5 ${isUser ? "items-end" : "items-start"}`}>
      <div className="min-w-0 max-w-[80%]">
        <div
          className={`text-small font-semibold mb-1 px-1 ${
            isUser ? "text-book-primary" : "text-book-text-muted"
          }`}
        >
          {isUser ? t("inspiration.you") : t("inspiration.creativeAssistant")}
        </div>
        <div
          className={`px-4 py-3 text-body-lg leading-relaxed whitespace-pre-wrap break-words ${
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
          {isUser || streamDisplaying ? (
            displayedContent
          ) : (
            <MarkdownText
              className="text-body-lg leading-relaxed tracking-tight"
              text={displayedContent}
            />
          )}
          {streamDisplaying && (
            pacedStream.isThinking ? (
              <LoadingSpinner
                size="sm"
                text={!displayedContent ? t("inspiration.organizingInspiration") : undefined}
                className="ml-1 align-middle"
              />
            ) : (
              <span className="ml-[1px] inline-block h-[14px] w-[7px] animate-pulse bg-book-accent align-text-bottom" />
            )
          )}
        </div>
      </div>
      {!isUser && ((displayOptions?.length ?? 0) > 0 || showPendingSkeleton) && (
        <div className="flex min-w-0 w-full max-w-[520px] flex-col gap-1.5 pl-3 sm:w-[80%] sm:min-w-[240px]">
          {displayOptions?.map((opt, index) => (
            <OptionCard
              key={`${opt.label}-${index}`}
              option={opt}
              onSend={() => onSendOption(opt.value)}
              interactive={canSelectOptions}
            />
          ))}
          {showPendingSkeleton && (
            <div
              aria-hidden="true"
              className="inspiration-option-card inspiration-option-card-skeleton w-full rounded-[8px] border border-dashed border-book-accent/35 bg-book-accent/5 px-3.5 py-3 text-left"
            >
              <div className="flex items-start gap-2">
                <div className="mt-0.5 h-3 w-3 shrink-0 rounded-md bg-book-accent/30" />
                <div className="min-w-0 flex-1 space-y-1.5">
                  <div className="h-3.5 w-3/5 rounded-md bg-book-accent/20" />
                  <div className="h-3 w-full rounded-md bg-book-accent/15" />
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
