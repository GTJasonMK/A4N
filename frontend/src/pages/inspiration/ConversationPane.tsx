import React, { useId } from "react";
import { Eye, Send } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { FollowLatestButton } from "../../components/ui/FollowLatestButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { ChatBubble } from "./ChatBubble";
import type { Message } from "./shared";
import { useI18n } from "../../i18n";

interface ConversationPaneProps {
  messages: Message[];
  input: string;
  setInput: (v: string) => void;
  inputRef: React.RefObject<HTMLTextAreaElement>;
  onSend: () => void;
  onPreview: () => void;
  onSendOption: (text: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onCompositionStart: () => void;
  onCompositionEnd: () => void;
  onInputFocus: () => void;
  onInputBlur: () => void;
  sending: boolean;
  messagesViewportRef: React.Ref<HTMLDivElement>;
  isFollowingLatest: boolean;
  hasNewContent: boolean;
  onScrollToLatest: () => void;
  placeholder: string;
  inputActive: boolean;
}

/**
 * 灵感对话主区：上方 stream、底部独立 composer（对齐预览 chat-composer，不拉高中部）
 */
export const ConversationPane: React.FC<ConversationPaneProps> = ({
  messages,
  input,
  setInput,
  inputRef,
  onSend,
  onPreview,
  onSendOption,
  onKeyDown,
  onCompositionStart,
  onCompositionEnd,
  onInputFocus,
  onInputBlur,
  sending,
  messagesViewportRef,
  isFollowingLatest,
  hasNewContent,
  onScrollToLatest,
  placeholder,
  inputActive,
}) => {
  const inputHintId = useId();
  const { locale, t } = useI18n();
  const inputHint = sending ? t("inspiration.inputHintSending") : null;
  const latestMessageId = messages[messages.length - 1]?.id;

  return (
    <div className="grid h-full min-h-0 min-w-0 grid-rows-[minmax(0,1fr)_auto]">
      <div className="relative min-h-0 min-w-0">
        <div
          ref={messagesViewportRef}
          className="h-full min-h-0 min-w-0 overflow-y-auto custom-scrollbar px-4 py-4 sm:px-5"
          style={{ overflowAnchor: "none" }}
        >
          <div className="mx-auto flex w-full min-w-0 max-w-3xl flex-col justify-start gap-3.5">
            {messages.map((m) => (
              <ChatBubble
                key={m.id}
                message={m}
                onSendOption={onSendOption}
                canSelectOptions={
                  !sending
                  && latestMessageId === m.id
                  && m.role === "assistant"
                  && !m.streaming
                  && !m.consistencyWarning
                }
              />
            ))}
          </div>
        </div>
        <FollowLatestButton
          isFollowingLatest={isFollowingLatest}
          hasNewContent={hasNewContent}
          newContentLabel={t("agent.domain.newMessages")}
          latestLabel={t("agent.domain.backToLatest")}
          onClick={onScrollToLatest}
          className="absolute bottom-3 left-1/2 z-10 -translate-x-1/2"
        />
      </div>

      <footer
        className={`chat-composer min-w-0 w-full shrink-0 border-t border-book-border/40 bg-book-bg-paper/90 px-3 pt-2.5 backdrop-blur-xl ${
        inputActive ? "pb-4 sm:pb-3" : "pb-2.5"
      }`}
      >
        <div className="mx-auto w-full min-w-0 max-w-3xl">
          {inputHint ? (
            <p id={inputHintId} title={inputHint} className="mb-1.5 truncate text-small text-book-text-muted">
              {inputHint}
            </p>
          ) : null}
          <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-end gap-2">
            <textarea
              ref={inputRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={onKeyDown}
              onCompositionStart={onCompositionStart}
              onCompositionEnd={onCompositionEnd}
              onFocus={onInputFocus}
              onBlur={onInputBlur}
              aria-label={t("inspiration.inputConversation")}
              aria-describedby={inputHint ? inputHintId : undefined}
              placeholder={placeholder || t("inspiration.inputPlaceholder")}
              rows={Math.min(5, Math.max(1, input.split("\n").length))}
              spellCheck={false}
              lang={locale}
              className="pyqt-input max-h-[140px] min-h-11 min-w-0 flex-1 resize-none !rounded-[12px] px-4 !py-[7px] text-body-lg leading-5 sm:resize-y"
            />
            <div className="flex shrink-0 items-center gap-2 justify-self-end">
              <Dropdown
                items={[
                  {
                    label: t("inspiration.viewTurnRequest"),
                    icon: <Eye size={12} />,
                    onClick: onPreview,
                    disabled: !input.trim() || sending,
                  },
                ]}
              />
              <BookButton
                variant="primary"
                size="sm"
                onClick={onSend}
                disabled={!input.trim() || sending}
                className="a4n-composer-send !px-3 !text-xs"
              >
                {sending ? (
                  t("inspiration.waitingReply")
                ) : (
                  <>
                    <Send size={14} />
                    {t("inspiration.send")}
                  </>
                )}
              </BookButton>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
};
