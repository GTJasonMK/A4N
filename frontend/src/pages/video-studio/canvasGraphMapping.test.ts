import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasTextAssetInput } from "../../api/generated/host";
import { translateMessage, type MessageTranslator } from "../../i18n";
import type { StudioNode } from "./types";

const mocks = vi.hoisted(() => ({
  saveCanvasTextAsset: vi.fn(),
}));

vi.mock("../../api/canvas", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api/canvas")>()),
  saveCanvasTextAsset: mocks.saveCanvasTextAsset,
}));

import { persistCanvasTextAssets, toCanvasGraphInput } from "./canvasGraphMapping";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

const textNode = (id: string, content: string): StudioNode =>
  ({
    id,
    position: { x: 0, y: 0 },
    data: {
      label: id,
      nodeKind: "TextInputNode",
      refType: null,
      refId: null,
      state: "ready",
      config: { content },
    },
  }) as StudioNode;

describe("persistCanvasTextAssets", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("saves independent text nodes concurrently and preserves node mapping", async () => {
    let active = 0;
    let maxActive = 0;
    let nextId = 0;
    mocks.saveCanvasTextAsset.mockImplementation(
      async (_projectId: number, input: CanvasTextAssetInput) => {
        active += 1;
        maxActive = Math.max(maxActive, active);
        await new Promise((resolve) => setTimeout(resolve, 0));
        active -= 1;
        nextId += 1;
        return {
          id: nextId,
          text_kind: input.text_kind,
          content: input.content,
        };
      },
    );

    const result = await persistCanvasTextAssets(7, [
      textNode("a", "第一段"),
      textNode("b", "第二段"),
      textNode("c", "第三段"),
    ], t);

    expect(maxActive).toBe(3);
    expect(result.map((node) => node.data.refId)).toEqual([1, 2, 3]);
    expect(result.map((node) => node.data.config.content)).toEqual([
      "第一段",
      "第二段",
      "第三段",
    ]);
  });
});

describe("toCanvasGraphInput", () => {
  it("preserves a generated text result when saving the canvas again", () => {
    const node = {
      id: "7",
      position: { x: 0, y: 0 },
      data: {
        label: "LLM",
        nodeKind: "LlmTextNode",
        refType: "prompt_artifact",
        refId: 12,
        state: "completed",
        config: {
          canvas_runtime_result: true,
          text: "已生成的提示词",
          prompt_artifact_id: 12,
        },
      },
    } as StudioNode;
    const input = toCanvasGraphInput(
      {
        graph: {
          id: 3,
          name: "画布",
          graph_kind: "video_studio",
          version: 1,
          viewport_json: null,
        },
        nodes: [],
        edges: [],
      },
      [node],
      [],
    );

    expect(input.nodes[0]?.config_json).toMatchObject({
      canvas_runtime_result: true,
      text: "已生成的提示词",
    });
  });
});
