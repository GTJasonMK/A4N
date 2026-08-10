import { describe, expect, it } from "vitest";
import { normalizeHistoryMessage } from "./inspirationChatUtils";
import { parseInspirationResponse } from "./responseParser";

const response = (conversationState: Record<string, unknown>) => JSON.stringify({
  ai_message: "概念已确认，可以生成蓝图。",
  next_question: null,
  progress_summary: "已确认全部核心信息",
  ui_control: { type: "info_display" },
  conversation_state: conversationState,
  is_complete: true,
});

describe("inspiration completion contract", () => {
  it("requires canonical genre and chapter_count", () => {
    expect(parseInspirationResponse(response({ genre: "科幻悬疑", chapter_count: 80 })))
      .toMatchObject({ isComplete: true });
    expect(() => parseInspirationResponse(response({ chapter_count: 80 })))
      .toThrow("conversation_state.genre");
  });

  it("fails on invalid persisted history instead of dropping it", () => {
    expect(() => normalizeHistoryMessage({
      id: 7,
      role: "tool",
      content: "invalid",
      created_at: "2026-08-09T00:00:00Z",
    })).toThrow("role 无效");
    expect(() => normalizeHistoryMessage({
      id: 8,
      role: "user",
      content: "继续",
      created_at: "not-a-date",
    })).toThrow("created_at 无效");
  });
});
