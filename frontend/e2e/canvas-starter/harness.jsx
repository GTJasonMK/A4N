import React from "react";
import { createRoot } from "react-dom/client";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { I18nProvider } from "../../src/i18n";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { PromptDialogHost } from "../../src/components/feedback/PromptDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { DirtyRegistryProvider } from "../../src/components/navigation/dirtyRegistry";
import { ContextMenuHost } from "../../src/components/ui/ContextMenu";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { StatusBarProvider } from "../../src/components/ui/StatusBar";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { VideoStudioCanvas } from "../../src/pages/VideoStudioCanvas";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import { installTauriEventMock } from "../tauriEventMock.js";
import "../../src/index.css";

const clone = (value) => JSON.parse(JSON.stringify(value));
const calls = [];
let nextNodeId = 1;
let nextEdgeId = 1;
let graph = {
  graph: {
    id: 17,
    name: "视频画布",
    graph_kind: "video_studio",
    version: 1,
    viewport_json: null,
  },
  nodes: [],
  edges: [],
};
let textAssets = [];
let textAssetVersions = new Map();

const project = {
  id: 1,
  title: "独立视频工作流",
  description: "",
  project_kind: "canvas",
  project_type: "script",
  lifecycle_gate: "active",
  collaboration_root_project_id: 1,
  current_chapter: 0,
  chapters_per_part: null,
  fact_feedback_interval_chapters: 1,
  member_count: 1,
  my_role: "owner",
  owner_username: "canvas-e2e",
  pending_invite_count: 0,
  progress: {},
  root_project_id: null,
  root_project_title: null,
  source_mode: "freeform",
  source_sync_status: "current",
  status: "active",
  target_chapters: 0,
  total_chapters: null,
  updated_at: "2026-10-03T00:00:00Z",
};

const port = (id, label, dataType, direction, required = false, multiple = true) => ({
  id,
  label,
  data_type: dataType,
  direction,
  required,
  multiple,
});

const contracts = [
  {
    kind: "TextInputNode",
    label: "文本输入",
    category: "input",
    runnable: false,
    inputs: [],
    outputs: [port("text", "文本", "text", "output")],
  },
  {
    kind: "LlmTextNode",
    label: "LLM 文本处理",
    category: "operation",
    runnable: true,
    inputs: [port("input", "文本", "text", "input", true)],
    outputs: [port("text", "文本", "text", "output")],
  },
  {
    kind: "GenerateVideoNode",
    label: "生成视频",
    category: "operation",
    runnable: true,
    inputs: [
      port("text", "文本", "text", "input"),
      port("first_frame", "首帧", "image", "input", false, false),
      port("end_frame", "尾帧", "image", "input", false, false),
      port("reference_image", "参考图", "image", "input"),
    ],
    outputs: [port("video", "视频", "video", "output")],
  },
  {
    kind: "VideoOutputNode",
    label: "视频输出",
    category: "output",
    runnable: true,
    inputs: [port("video", "视频", "video", "input", true, false)],
    outputs: [],
  },
];

const hashContent = (content) => `browser-sha256-${content.length}`;

async function invoke(command, args = {}) {
  if (command === "invoke_host_command") {
    if (args.userId !== 1) throw new Error(`unexpected caller ${args.userId}`);
    return invoke(args.command, args.args ?? {});
  }
  calls.push({ command, args: clone(args) });
  switch (command) {
    case "get_all_frontend_state":
      return { "a4n:current_user_id": "1" };
    case "set_frontend_state_many":
    case "remove_frontend_state_many":
      return null;
    case "get_project":
      return clone(project);
    case "get_canvas_graph":
      return clone(graph);
    case "get_canvas_node_contracts":
      return clone(contracts);
    case "list_canvas_text_assets":
      return clone(textAssets);
    case "list_canvas_text_asset_versions":
      return clone(textAssetVersions.get(args.input?.asset_id) ?? []);
    case "save_canvas_text_asset": {
      const input = args.input?.text_asset;
      if (!input) throw new Error("missing text_asset");
      const previous = input.id == null
        ? null
        : textAssets.find((asset) => asset.id === input.id) ?? null;
      if (input.id != null && !previous) throw new Error("text asset not found");
      if (previous && previous.current_version !== input.expected_version) {
        throw new Error("text asset version conflict");
      }
      const version = previous ? previous.current_version + 1 : 1;
      const asset = {
        id: previous?.id ?? (textAssets.reduce((max, item) => Math.max(max, item.id), 0) + 1),
        project_id: args.input.project_id,
        title: input.title,
        text_kind: input.text_kind,
        content: input.content,
        content_hash: hashContent(input.content),
        current_version: version,
        updated_at: new Date().toISOString(),
      };
      textAssets = previous
        ? textAssets.map((item) => item.id === asset.id ? asset : item)
        : [...textAssets, asset];
      const versions = textAssetVersions.get(asset.id) ?? [];
      textAssetVersions.set(asset.id, [
        { version, content: asset.content, content_hash: asset.content_hash, created_at: asset.updated_at },
        ...versions,
      ]);
      return clone(asset);
    }
    case "save_canvas_graph": {
      const { expected_version: expectedVersion, graph: inputGraph } = args.input;
      if (expectedVersion !== graph.graph.version) throw new Error("graph version conflict");
      const nodeIds = {};
      const nodes = inputGraph.nodes.map((node) => {
        const id = nextNodeId++;
        nodeIds[node.client_id] = id;
        return {
          id,
          node_kind: node.node_kind,
          ref_type: node.ref_type ?? null,
          ref_id: node.ref_id ?? null,
          config_json: node.config_json,
          position_json: node.position_json,
          state: node.state,
        };
      });
      const edges = inputGraph.edges.map((edge) => ({
        id: nextEdgeId++,
        source_node_id: nodeIds[edge.source_client_id],
        target_node_id: nodeIds[edge.target_client_id],
        source_port: edge.source_port,
        target_port: edge.target_port,
        role: edge.role,
        config_json: edge.config_json,
      }));
      graph = {
        graph: {
          id: inputGraph.id ?? graph.graph.id,
          name: inputGraph.name,
          graph_kind: inputGraph.graph_kind,
          version: graph.graph.version + 1,
          viewport_json: inputGraph.viewport_json ?? null,
        },
        nodes,
        edges,
      };
      return { ...clone(graph), node_ids: nodeIds };
    }
    case "list_llm_config":
    case "list_image_configs":
    case "list_video_configs":
    case "list_model_provider_catalog":
    case "list_active_generation_tasks":
    case "list_workflow_runs":
      return [];
    default:
      console.warn(`[canvas-starter-e2e] unhandled command: ${command}`);
      return [];
  }
}

window.__TAURI_INTERNALS__ = {
  invoke,
  convertFileSrc: (path) => path,
  transformCallback: () => 1,
  unregisterCallback: () => undefined,
};
installTauriEventMock(window.__TAURI_INTERNALS__);
window.__A4N_CANVAS_STARTER_E2E__ = {
  calls,
  getGraph: () => clone(graph),
  getTextAssets: () => clone(textAssets),
};

await initializePortableStorage();
const router = createMemoryRouter(
  [{ path: "/video/:id/canvas", element: <VideoStudioCanvas /> }],
  { initialEntries: ["/video/1/canvas"] },
);
createRoot(document.getElementById("root")).render(
  <I18nProvider initialLocale="zh-CN">
    <StatusBarProvider>
      <ProgressTaskProvider>
        <ToastContainer />
        <ConfirmDialogHost />
        <PromptDialogHost />
        <ImageLightboxProvider>
          <ContextMenuHost />
          <DirtyRegistryProvider>
            <RouterProvider router={router} />
          </DirtyRegistryProvider>
        </ImageLightboxProvider>
      </ProgressTaskProvider>
    </StatusBarProvider>
  </I18nProvider>,
);
