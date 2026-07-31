const WORKFLOW_STATUS_LABELS: Record<string, string> = {
  idle: "未运行",
  draft: "草稿",
  imported: "已导入",
  ready: "就绪",
  pending: "等待中",
  analyzing: "分析中",
  reviewing: "审核中",
  queued: "排队中",
  running: "运行中",
  completed: "已完成",
  success: "成功",
  failed: "失败",
  error: "错误",
  interrupted: "已中断",
  cancelled: "已取消",
  canceled: "已取消",
  paused: "已暂停",
  blocked: "已阻塞",
  generating: "生成中",
  generated: "已生成",
  deleted: "已删除",
  approved: "已通过",
  accepted: "已接受",
  rejected: "已拒绝",
  modified: "已修改",
  stale: "已过期",
};

export const getOperationStatusLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return WORKFLOW_STATUS_LABELS[normalized] ?? "未知状态";
};

const TASK_TYPE_LABELS: Record<string, string> = {
  llm_call: "语言模型调用",
  chapter_generation: "章节生成",
  image_generation: "图片生成",
  video_generation: "视频生成",
  video_poll: "视频轮询",
  source_sync: "源文本同步",
  assembly: "成片合成",
  embedding_call: "文本向量计算",
  rag_ingestion: "知识库入库",
  import_analysis: "导入分析",
  export_generation: "导出生成",
  embedding_download: "模型下载",
};

const TASK_PRIORITY_LABELS: Record<string, string> = {
  high: "高优先级",
  medium: "中优先级",
  low: "低优先级",
};

const WORKFLOW_KIND_LABELS: Record<string, string> = {
  video_provider_job: "视频任务",
  video_canvas_graph: "视频画布",
  chapter_analysis: "章节分析",
  source_sync: "源文本同步",
  import_analysis: "导入分析",
  rag_ingestion: "RAG 入库",
};

const WORKFLOW_MODE_LABELS: Record<string, string> = {
  full: "完整运行",
  fresh: "重新运行",
  start: "首次运行",
  resume: "续传",
  retry: "重试",
  incremental: "增量更新",
  boundary_frames: "首尾帧",
  first_frame_chain: "首帧串联",
  storyboard: "分镜",
  chat_video: "Chat 视频",
  reference_media: "参考媒体",
};

const mappedLabel = (
  value: string | null | undefined,
  labels: Record<string, string>,
  fallback = "未知",
): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return labels[normalized] ?? fallback;
};

export const getTaskTypeLabel = (value: string | null | undefined): string =>
  mappedLabel(value, TASK_TYPE_LABELS, "任务");

export const getTaskPriorityLabel = (value: string | null | undefined): string =>
  mappedLabel(value, TASK_PRIORITY_LABELS, "普通优先级");

export const getWorkflowKindLabel = (value: string | null | undefined): string =>
  mappedLabel(value, WORKFLOW_KIND_LABELS, "工作流");

export const getWorkflowModeLabel = (value: string | null | undefined): string =>
  mappedLabel(value, WORKFLOW_MODE_LABELS, "默认模式");
