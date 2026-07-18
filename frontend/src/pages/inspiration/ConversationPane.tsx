import React from "react";
import { Eye, Loader2, Send } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { ChatBubble } from "./ChatBubble";
import type { Message } from "./shared";

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
  showThinking: boolean;
  messagesEndRef: React.RefObject<HTMLDivElement>;
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
  showThinking,
  messagesEndRef,
  placeholder,
  inputActive,
}) => (
  <div className="grid h-full min-h-0 grid-rows-[minmax(0,1fr)_auto]">
    <div className="min-h-0 overflow-y-auto custom-scrollbar px-4 py-4 sm:px-5">
      <div className="mx-auto flex max-w-3xl flex-col justify-start gap-3.5">
        {messages.map((m) => (
          <ChatBubble key={m.id} message={m} onSendOption={onSendOption} />
        ))}
        {showThinking && (
          <div className="flex items-center gap-1.5 pl-1 text-sm text-book-text-muted">
            <Loader2 size={13} className="animate-spin" /> 创作助手正在整理…
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
    </div>

    <footer
      className={`chat-composer shrink-0 border-t border-book-border/40 bg-book-bg-paper/90 px-3 pt-2.5 backdrop-blur-xl ${
        inputActive ? "pb-4 sm:pb-3" : "pb-2.5"
      }`}
    >
      <div className="mx-auto max-w-3xl">
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          onCompositionStart={onCompositionStart}
          onCompositionEnd={onCompositionEnd}
          onFocus={onInputFocus}
          onBlur={onInputBlur}
          placeholder={placeholder}
          disabled={sending}
          rows={Math.min(5, Math.max(2, input.split("\n").length))}
          spellCheck={false}
          lang="zh-CN"
          className="pyqt-input mb-2 max-h-[140px] min-h-[52px] w-full resize-y !rounded-[16px] px-4 py-2.5 text-[15px] leading-5"
        />
        <div className="flex items-center gap-2">
          <span className="mr-auto hidden text-[11px] text-book-text-muted sm:inline">
            Enter 发送 · Shift+Enter 换行
          </span>
          <Dropdown
            items={[
              {
                label: "查看本轮模型请求",
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
            className="!min-h-8 !px-3 !text-xs"
          >
            {sending ? (
              <>
                <Loader2 size={14} className="animate-spin" />
                发送中…
              </>
            ) : (
              <>
                <Send size={14} />
                发送
              </>
            )}
          </BookButton>
        </div>
      </div>
    </footer>
  </div>
);
