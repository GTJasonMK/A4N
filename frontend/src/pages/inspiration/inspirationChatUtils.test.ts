import { describe, expect, it } from "vitest";
import {
  buildInspirationTranscript,
  normalizeHistoryMessage,
} from "./inspirationChatUtils";
import {
  getStreamingResponsePreview,
  parseInspirationResponse,
} from "./responseParser";

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

  it("does not hide an invalid raw assistant response behind rendered content", () => {
    expect(() => buildInspirationTranscript([{
      id: "assistant-1",
      role: "assistant",
      content: "已渲染正文",
      rawContent: "   ",
      createdAt: 1,
    }])).toThrow("空消息");
  });

  it("does not synthesize UI identifiers for streamed options", () => {
    const preview = getStreamingResponsePreview(
      '{"ai_message":"请选择","ui_control":{"type":"inspired_options","options":'
      + '[{"label":"方向一","description":"说明","key_elements":["要素一","要素二"]}]}',
    );

    expect(preview.options?.[0]).toMatchObject({
      label: "方向一",
      value: "选择：方向一\n说明\n关键要素：要素一、要素二",
    });
    expect(preview.options?.[0]).not.toHaveProperty("id");
  });

  it("validates and discards persisted option identifiers", () => {
    const parsed = parseInspirationResponse(JSON.stringify({
      ai_message: "请选择一个方向。",
      next_question: "你更偏好哪一个？",
      progress_summary: null,
      ui_control: {
        type: "inspired_options",
        options: [{
          id: "legacy-option",
          label: "方向一",
          description: "说明",
          key_elements: ["要素一", "要素二"],
        }],
      },
      conversation_state: {},
      is_complete: false,
    }));

    expect(parsed.options?.[0]).toMatchObject({
      label: "方向一",
      value: "选择：方向一\n说明\n关键要素：要素一、要素二",
    });
    expect(parsed.options?.[0]).not.toHaveProperty("id");

    expect(() => parseInspirationResponse(JSON.stringify({
      ai_message: "请选择一个方向。",
      next_question: "你更偏好哪一个？",
      progress_summary: null,
      ui_control: {
        type: "inspired_options",
        options: [{
          label: "方向一",
          description: "说明",
          key_elements: ["要素一", "要素二"],
        }],
      },
      conversation_state: {},
      is_complete: false,
    }))).toThrow("ui_control.options[0].id");
  });

  it("keeps next_question in the guide instead of duplicating it in the bubble", () => {
    const parsed = parseInspirationResponse(JSON.stringify({
      ai_message: "请选择一个主角方向。",
      next_question: "主角为什么留在这里？",
      progress_summary: "已确认核心火花",
      ui_control: { type: "text_input", placeholder: "说说主角的动机" },
      conversation_state: {},
      is_complete: false,
    }));

    expect(parsed.content).toBe("请选择一个主角方向。");
    expect(parsed.nextQuestion).toBe("主角为什么留在这里？");
  });
});
