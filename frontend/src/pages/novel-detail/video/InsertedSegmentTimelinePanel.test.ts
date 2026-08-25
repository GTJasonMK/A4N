import { describe, expect, it } from "vitest";
import {
  buildInsertedSegmentGenerationOptions,
  gapLabel,
  timelineGapsForMove,
} from "./InsertedSegmentTimelinePanel";
import type { VideoTimelineItemSummary } from "../../../api/video";
import { translateMessage, type MessageTranslator } from "../../../i18n";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

describe("InsertedSegmentTimelinePanel", () => {
  it("flattens the selected gap into the strict generation contract", () => {
    const options = buildInsertedSegmentGenerationOptions(7, 3, {
      gap: { previous_item_id: 11, next_item_id: 12 },
      user_prompt: "补一个反应镜头",
      style: "电影感",
      prompt_output_language: "zh_cn",
      duration_secs: 6,
      duration_secs_options: [6, 10],
      aspect_ratio: "16:9",
      reference_image_ids: [21],
      allow_without_visual_anchor: false,
    });

    expect(options).toMatchObject({
      timeline_id: 7,
      expected_timeline_revision: 3,
      previous_item_id: 11,
      next_item_id: 12,
      duration_secs: 6,
      duration_secs_options: [6, 10],
    });
    expect(options).not.toHaveProperty("gap");
  });

  it("builds move gaps after removing the moving item", () => {
    expect(timelineGapsForMove([{ id: 1 }, { id: 8 }, { id: 2 }], 8)).toEqual([
      { previous_item_id: null, next_item_id: 1 },
      { previous_item_id: 1, next_item_id: 2 },
      { previous_item_id: 2, next_item_id: null },
    ]);
  });

  it("keeps the first timeline item when formatting a gap", () => {
    const item = (
      id: number,
      title: string,
      display_index: number,
    ): VideoTimelineItemSummary => ({
      id,
      title,
      display_index,
      generated_video_id: null,
      kind: "inserted",
      plan_prompt_id: null,
      segment_number: null,
      source_shot_id: null,
      status: "active",
    });
    expect(
      gapLabel(
        { previous_item_id: 1, next_item_id: null },
        [item(1, "首个镜头", 1), item(1, "重复镜头", 2)],
        t,
      ),
    ).toContain("首个镜头");
  });
});
