import { uiFixtures } from "./fixtures.mjs";
import { authoringChapters } from "../media-authoring/fixtures.mjs";

const project = uiFixtures.projectRecords[0];
const records = [
  ...uiFixtures.projectRecords.map((item) => item.id === 3 ? { ...item, lifecycle_gate: "active" } : item),
  { ...project, id: 5, title: "山海之间 · 交互叙事", project_kind: "galgame", root_project_id: 1, root_project_title: project.title, source_mode: "derived" },
  { ...project, id: 6, title: "山海之间 · 影像画布", project_kind: "canvas", project_type: "canvas", source_mode: "freeform" },
];
const port = (id, direction, dataType) => ({ id, label: id, direction, data_type: dataType, multiple: false, required: false });
export const visualFixtures = {
  projectRecords: records,
  list_projects: {
    ...uiFixtures.list_projects,
    items: uiFixtures.list_projects.items.map((item) => item.id === 4
      ? { ...item, title: "只读协作项目 · 关于那些未曾说出口的话与漫长归途的创作记录", owner_username: "共同完成这段旅程的创作者" }
      : item),
  },
  list_chapters: uiFixtures.list_chapters.map((chapter) => chapter.id === 2
    ? { ...chapter, content: "第 2 章 未寄出的信\n\n" + Array.from({ length: 8 }, () => "雨从旧书店的屋檐落下。她把那封信放在桌上，纸页的边缘已经微微卷起。窗外的灯光映着街道，来往的人都有自己的归途，而她还在等待一个足以启程的答案。\n\n老人没有催促，只是翻开书，把一枚旧车票夹在他们都熟悉的那一页。那些没有说出口的话，终于有了可以停留的地方。").join("\n\n") }
    : chapter),
  list_source_snapshot_chapters: authoringChapters(1),
  get_inspiration_history: [],
  list_portraits_by_project: [],
  list_images_by_project: [],
  list_images_by_projects: [],
  list_images_by_chapter: [],
  list_image_configs: [],
  list_video_configs: [],
  list_canvas_text_assets: [],
  list_workflow_runs: [],
  get_galgame_pipeline_status: { run: null, checkpoints: [] },
  get_galgame_script: { id: 5, root_project_id: 1, title: "山海之间", scenes: [], media_bindings: [], speaker_profiles: [], text_anchors: [] },
  get_canvas_graph: {
    graph: { id: 6, graph_kind: "video_studio", name: "影像画布", version: 1, viewport_json: null },
    nodes: [
      { id: 1, node_kind: "TextInputNode", position_json: { x: 40, y: 100 }, ref_id: null, ref_type: null, state: "ready", config_json: { title: "场景构思", content: "雨夜的旧书店，门口有一封未寄出的信。" } },
      { id: 2, node_kind: "GenerateImageNode", position_json: { x: 380, y: 100 }, ref_id: null, ref_type: null, state: "ready", config_json: { title: "定格画面" } },
    ],
    edges: [{ id: 1, source_node_id: 1, source_port: "text", target_node_id: 2, target_port: "text", role: "context", config_json: {} }],
  },
  get_canvas_node_contracts: [
    { kind: "TextInputNode", label: "文本输入", category: "input", runnable: false, inputs: [], outputs: [port("text", "output", "text")] },
    { kind: "GenerateImageNode", label: "生成图片", category: "operation", runnable: true, inputs: [port("text", "input", "text")], outputs: [port("image", "output", "image")] },
    { kind: "ImageOutputNode", label: "图片输出", category: "output", runnable: true, inputs: [port("image", "input", "image")], outputs: [] },
  ],
};
