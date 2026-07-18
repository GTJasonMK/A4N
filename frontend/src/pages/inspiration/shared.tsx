import React from "react";

/**
 * InspirationChat 各子组件共用的类型与小组件。
 */

export interface MessageOption {
  id?: string;
  label: string;
  description?: string;
  keyElements?: string[];
  value?: string;
}

export interface Message {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  rawContent?: string;
  options?: MessageOption[];
  isComplete?: boolean;
  conversationState?: Record<string, unknown>;
  progressSummary?: string;
  nextQuestion?: string;
  streaming?: boolean;
  streamingOptions?: MessageOption[];
  optionsPending?: boolean;
  createdAt: number;
}

export const Section: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div>
    <div className="mb-1.5 text-[11px] font-medium text-book-text-muted">{label}</div>
    <div className="text-sm text-book-text-main">{children}</div>
  </div>
);
