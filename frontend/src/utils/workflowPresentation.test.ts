import { describe, expect, it } from "vitest";
import { getChapterSyncFailurePresentation } from "./workflowPresentation";
import { translateMessage } from "../i18n";

const t = (
  key: Parameters<typeof translateMessage>[1],
  params?: Parameters<typeof translateMessage>[2],
) => translateMessage("zh-CN", key, params);

describe("getChapterSyncFailurePresentation", () => {
  it("turns vector dimension failures into an actionable embedding settings message", () => {
    const detail = "Vector dimension error: expected dim: 384, got 768";

    expect(getChapterSyncFailurePresentation(detail, "同步失败", t)).toEqual({
      summary:
        "资料索引与当前嵌入模型不兼容。正文已经保存，请检查嵌入模型配置后重试同步。",
      detail,
      recommendedAction: "embedding_settings",
    });
  });

  it("keeps unknown diagnostics while showing the caller-owned recovery message", () => {
    expect(
      getChapterSyncFailurePresentation(
        "provider unavailable",
        "正文已保存，请重试同步",
        t,
      ),
    ).toEqual({
      summary: "正文已保存，请重试同步",
      detail: "provider unavailable",
      recommendedAction: null,
    });
  });
});
