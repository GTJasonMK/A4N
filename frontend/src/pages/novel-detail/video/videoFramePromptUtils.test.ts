import { describe, expect, it } from "vitest";

import type { MessageTranslator } from "../../../i18n";
import { buildVideoFrameImageRequest } from "./videoFramePromptUtils";

const translate = ((key: string) => key) as MessageTranslator;

describe("video frame image request", () => {
  it("preserves the authored prompt, negative constraints and reference order", () => {
    const metadata = { prompt_snapshot: { negative_prompt: "不要改变角色服装" } };
    const request = buildVideoFrameImageRequest(
      "video_start_frame",
      "  人物站在窗边  ",
      3,
      "16:9",
      metadata,
      [7, 4],
      "参考图 1 是上一帧",
      translate,
    );

    expect(request.prompt).toBe("参考图 1 是上一帧\n\n人物站在窗边");
    expect(request.negative_prompt).toBe("不要改变角色服装");
    expect(request.reference_image_ids).toEqual([7, 4]);
    expect(request.metadata_json).toBe(metadata);
  });
});
