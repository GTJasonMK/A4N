import { describe, expect, it } from "vitest";

import type { ProgressMeta, ProgressTimelineItem } from "../api/progress";
import { translateMessage, type Locale, type MessageTranslator } from "../i18n";
import { streamLineFromBuffer } from "./progressDisplay";
import { compactProgressEvents, progressDisplayModel } from "./progressPresentation";

const translator = (locale: Locale): MessageTranslator =>
  (key, params) => translateMessage(locale, key, params);

const meta: ProgressMeta = {
  taskKind: "llm_call",
  stageCode: "llm_call",
  stageLabel: "模型生成",
  phaseKind: "model_stream",
  detail: "模型已开始输出片段规划",
  unitLabel: "片段",
  itemIndex: 1,
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
        locale: "zh-CN",
        t: translator("zh-CN"),
      }),
    ).toMatchObject({
      stage: "语言模型调用",
      detail: "模型已开始输出片段规划",
      countLabel: "第 2/5 项",
      etaMode: "hidden",
    });
  });

  it("uses the explicit stage instead of guessing one from message text", () => {
    expect(
      progressDisplayModel({
        current: 1,
        total: 2,
        message: "正在保存生成结果",
        fallbackStage: "后台任务",
        locale: "zh-CN",
        t: translator("zh-CN"),
      }),
    ).toMatchObject({
      stage: "后台任务",
      detail: "正在保存生成结果",
    });
  });

  it("localizes structured stages instead of rendering backend Chinese", () => {
    const display = progressDisplayModel({
      current: 1,
      total: 2,
      message: "正在连接模型",
      meta: { ...meta, stageCode: "provider_connect", phaseKind: "model_connect" },
      locale: "en-US",
      t: translator("en-US"),
    });

    expect(display).toMatchObject({
      stage: "Connecting to model",
      detail: "Connecting to the model",
      countLabel: "2/5",
    });
    expect(display.stage).not.toContain("模型");
    expect(display.detail).not.toContain("正在");
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
