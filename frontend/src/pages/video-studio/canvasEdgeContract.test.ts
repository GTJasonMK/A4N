import { describe, expect, it } from "vitest";
import { canvasGraphSaveValidationError } from "./canvasEdgeContract";
import type { StudioNodeContractMap } from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

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

    expect(canvasGraphSaveValidationError(nodes, edges, contracts)).toContain("端口配置不匹配");
  });
});
