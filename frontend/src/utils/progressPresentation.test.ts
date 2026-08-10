import { describe, expect, it } from "vitest";

import type { ProgressMeta, ProgressTimelineItem } from "../api/progress";
import { streamLineFromBuffer } from "./progressDisplay";
import { compactProgressEvents, progressDisplayModel } from "./progressPresentation";

const meta: ProgressMeta = {
  taskKind: "llm_call",
  stageCode: "llm_call",
  stageLabel: "模型生成",
  phaseKind: "model_stream",
  detail: "模型已开始输出片段规划",
  unitLabel: "片段",
  itemIndex: 2,
  itemTotal: 5,
  parentStageCode: null,
  severity: "info",
  estimated: false,
  persistent: false,
};

describe("progress presentation", () => {
  it("compacts cumulative model output to the latest single-line tail", () => {
    expect(streamLineFromBuffer(`  start\n${"尾".repeat(200)}  `)).toBe(
      `…${"尾".repeat(180)}`,
    );
  });

  it("uses structured metadata for the rich progress display", () => {
    expect(
      progressDisplayModel({
        current: 40,
        total: 100,
        message: "fallback",
        meta,
      }),
    ).toMatchObject({
      stage: "模型生成",
      detail: "模型已开始输出片段规划",
      countLabel: "第 2/5 个片段",
      etaMode: "hidden",
    });
  });

  it("keeps distinct backend stages while compacting duplicate snapshots", () => {
    const event = (id: string, detail: string): ProgressTimelineItem => ({
      id,
      status: "running",
      current: 40,
      total: 100,
      message: detail,
      meta: { ...meta, detail },
      timestamp: 1,
    });

    expect(
      compactProgressEvents(
        [event("1", "连接模型"), event("2", "连接模型"), event("3", "模型输出")],
        12,
      ),
    ).toHaveLength(2);
  });
});
