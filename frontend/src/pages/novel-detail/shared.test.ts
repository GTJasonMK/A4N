import { describe, expect, it } from "vitest";

import { translateMessage, type MessageTranslator } from "../../i18n";
import { profileChapterRangeText } from "./shared";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

describe("profileChapterRangeText", () => {
  it("marks profiles without current evidence as pending confirmation", () => {
    expect(profileChapterRangeText(0, 0, [], t)).toBe(
      "出场章节待正文确认",
    );
  });

  it("keeps the chapter range when evidence is present", () => {
    expect(profileChapterRangeText(2, 4, ["原文证据"], t)).toBe(
      "第 2-4 章",
    );
  });
});
