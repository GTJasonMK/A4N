import React from "react";
import { Eye, Loader2, Send } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
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
  <div className="h-full flex flex-col">
    {/* 消息流 */}
    <div className="flex-1 overflow-y-auto custom-scrollbar px-4 py-3">
      <div className="max-w-3xl mx-auto flex flex-col gap-2">
        {messages.map((m) => (
          <ChatBubble key={m.id} message={m} onSendOption={onSendOption} />
        ))}
        {showThinking && (
          <div className="flex items-center gap-1.5 text-xs text-book-text-muted pl-1">
            <Loader2 size={11} className="animate-spin" /> AI 正在思考…
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
    </div>

    {/* 输入栏 */}
    <footer
      className={`border-t border-book-border bg-book-bg-tertiary px-3 pt-2 shrink-0 transition-[padding] duration-150 ${
        inputActive ? "pb-16" : "pb-2"
      }`}
    >
      <div className="mx-auto flex max-w-3xl items-end gap-2">
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
          rows={Math.min(6, Math.max(1, input.split("\n").length))}
          spellCheck={false}
          lang="zh-CN"
          className="pyqt-input min-h-[36px] min-w-0 flex-1 resize-none px-2 py-1.5 text-xs leading-5 max-h-[160px]"
        />
        <BookButton
          variant="secondary"
          onClick={onPreview}
          disabled={!input.trim() || sending}
          className="shrink-0 !px-3"
          title="预览提示词"
        >
          <span className="inline-flex items-center gap-1">
            <Eye size={12} /> <span className="hidden sm:inline">预览提示词</span>
          </span>
        </BookButton>
        <BookButton
          variant="primary"
          onClick={onSend}
          disabled={!input.trim() || sending}
          className="shrink-0 !px-3"
        >
          <span className="inline-flex items-center gap-1">
            <Send size={12} /> <span className="hidden sm:inline">发送</span>
          </span>
        </BookButton>
      </div>
    </footer>
  </div>
);
