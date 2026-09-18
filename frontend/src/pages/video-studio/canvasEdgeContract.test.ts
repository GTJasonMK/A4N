import { describe, expect, it } from "vitest";
import { canvasGraphSaveValidationError, validateCanvasConnection } from "./canvasEdgeContract";
import { translateMessage, type MessageTranslator } from "../../i18n";
import { studioNodeInputPortsForData, type StudioNodeContractMap } from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

const contracts: StudioNodeContractMap = new Map([
  [
    "TextInputNode",
    {
      kind: "TextInputNode",
      label: "文本输入",
      category: "input",
      runnable: false,
      inputs: [],
      outputs: [
        { id: "text", label: "文本", direction: "output", data_type: "text", multiple: false, required: false },
      ],
    },
  ],
  [
    "GenerateImageNode",
    {
      kind: "GenerateImageNode",
      label: "生成图片",
      category: "operation",
      runnable: true,
      inputs: [
        { id: "text", label: "文本", direction: "input", data_type: "text", multiple: false, required: true },
      ],
      outputs: [],
    },
  ],
  [
    "ImageOutputNode",
    {
      kind: "ImageOutputNode",
      label: "图片输出",
      category: "output",
      runnable: true,
      inputs: [
        { id: "image", label: "图片", direction: "input", data_type: "image", multiple: false, required: true },
      ],
      outputs: [
        { id: "image", label: "图片", direction: "output", data_type: "image", multiple: true, required: false },
      ],
    },
  ],
  [
    "GenerateVideoNode",
    {
      kind: "GenerateVideoNode",
      label: "生成视频",
      category: "operation",
      runnable: true,
      inputs: [
        { id: "text", label: "文本", direction: "input", data_type: "text", multiple: true, required: false },
        { id: "first_frame", label: "首帧", direction: "input", data_type: "image", multiple: false, required: false },
        { id: "end_frame", label: "尾帧", direction: "input", data_type: "image", multiple: false, required: false },
        { id: "reference_image", label: "参考图", direction: "input", data_type: "image", multiple: true, required: false },
      ],
      outputs: [],
    },
  ],
]);

const nodes: StudioNode[] = [
  {
    id: "1",
    position: { x: 0, y: 0 },
    data: { label: "输入", nodeKind: "TextInputNode", refType: null, refId: null, state: "ready", config: {} },
  },
  {
    id: "2",
    position: { x: 100, y: 0 },
    data: { label: "图片", nodeKind: "GenerateImageNode", refType: null, refId: null, state: "ready", config: {} },
  },
];

describe("canvasGraphSaveValidationError", () => {
  it("rejects inferable legacy ports instead of rewriting them", () => {
    const edges: StudioEdge[] = [
      {
        id: "edge-1",
        source: "1",
        target: "2",
        sourceHandle: "output",
        targetHandle: "input",
        data: { role: "context", config: {} },
      },
    ];

    expect(canvasGraphSaveValidationError(nodes, edges, contracts, t)).toContain("端口配置不匹配");
  });

  it("allows an image output node to pass its image into a downstream image input", () => {
    const outputNode: StudioNode = {
      id: "image-output",
      position: { x: 200, y: 0 },
      data: { label: "图片输出", nodeKind: "ImageOutputNode", refType: null, refId: null, state: "ready", config: {} },
    };
    const videoNode: StudioNode = {
      id: "video",
      position: { x: 300, y: 0 },
      data: { label: "生成视频", nodeKind: "GenerateVideoNode", refType: null, refId: null, state: "ready", config: { input_mode: "first_frame" } },
    };

    expect(
      validateCanvasConnection(
        {
          source: outputNode.id,
          target: videoNode.id,
          sourceHandle: "image",
          targetHandle: "first_frame",
        },
        [outputNode, videoNode],
        [],
        contracts,
        t,
      ),
    ).toMatchObject({ sourcePort: "image", targetPort: "first_frame", role: "first_frame" });
  });

  it("keeps dynamic video image handles aligned with the selected input mode", () => {
    const contract = contracts.get("GenerateVideoNode");
    expect(contract).toBeDefined();
    expect(
      studioNodeInputPortsForData(
        { nodeKind: "GenerateVideoNode", config: { input_mode: "text_only" } },
        contract!,
      ).map((port) => port.id),
    ).toEqual(["text"]);
    expect(
      studioNodeInputPortsForData(
        { nodeKind: "GenerateVideoNode", config: { input_mode: "first_end_frame" } },
        contract!,
      ).map((port) => port.id),
    ).toEqual(["text", "first_frame", "end_frame"]);
  });
});
