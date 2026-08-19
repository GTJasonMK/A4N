import React from "react";
import { AlertTriangle, ArrowDown, ArrowUp, Eye } from "lucide-react";
import {
  CANVAS_PROMPT_KIND,
  CANVAS_TEXT_OUTPUT_KIND_DIALOGUE,
  CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT,
  CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN,
  CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT,
  CANVAS_TEXT_OUTPUT_KIND_STORYBOARD,
  CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION,
  CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT,
} from "../../api/canvas";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import {
  canvasDataTypeLabel,
  edgeRoleLabel,
  nodeDisplayTitle,
  nodePreviewSrc,
  nodeValidationIssues,
} from "./StudioCanvasNode";
import { nodeHasContentPreview } from "./CanvasNodeContentPreviewDialog";
import {
  configNumber,
  configObject,
  configOptionLabel,
  configPositiveNumber,
  configText,
  firstLine,
  numberOrNull,
  type StudioConfigOption,
  type StudioConfigOptionsByKind,
} from "./canvasConfig";
import { assembleClipEdges, edgeOrderIndex } from "./canvasAssemblyEdges";
import { isProbeNodeKind } from "./canvasNodeKinds";
import {
  CANVAS_REF_TYPE_GENERATED_IMAGE,
  CANVAS_REF_TYPE_GENERATED_VIDEO,
  CANVAS_REF_TYPE_MEDIA_ASSET,
  defaultRefTypeForNode,
  nodeGeneratedImageRefId,
  nodeGeneratedVideoRefId,
  nodeMediaAssetRefId,
} from "./canvasNodeRefs";
import {
  parseGenerateVideoInputMode,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  useStudioNodeContract,
} from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

const INSPECTOR_CONTROL_CLASS =
  "pyqt-input mt-1 w-full !px-2.5 !py-1.5 text-xs";
const INSPECTOR_TEXTAREA_CLASS = `${INSPECTOR_CONTROL_CLASS} resize-none leading-relaxed`;
const CANVAS_INPUT_REFERENCE_LABELS: Record<string, string> = {
  [CANVAS_REF_TYPE_GENERATED_IMAGE]: "图片资源编号",
  [CANVAS_REF_TYPE_GENERATED_VIDEO]: "视频资源编号",
  [CANVAS_REF_TYPE_MEDIA_ASSET]: "音频资源编号",
};

interface NodeConfigEditorProps {
  node: StudioNode;
  nodes: StudioNode[];
  edges: StudioEdge[];
  disabled: boolean;
  onChange: (patch: Record<string, unknown>) => void;
  onRefChange: (refType: string | null, refId: number | null) => void;
  onMoveAssembleClip: (edgeId: string, direction: -1 | 1) => void;
  configOptions: StudioConfigOptionsByKind;
}

interface NodeInspectorProps extends NodeConfigEditorProps {
  node: StudioNode;
  onPreviewContent: (node: StudioNode) => void;
}

export const NodeInspector: React.FC<NodeInspectorProps> = ({
  node,
  nodes,
  edges,
  disabled,
  onChange,
  onRefChange,
  onMoveAssembleClip,
  configOptions,
  onPreviewContent,
}) => {
  const issues = nodeValidationIssues(node.data);
  const previewSrc = nodePreviewSrc(node.data);
  return (
    <div className="space-y-3 text-xs">
      {previewSrc ? (
        <img
          src={previewSrc}
          alt=""
          className="max-h-44 w-full rounded-2xl border border-book-border/55 bg-book-bg-tertiary object-contain"
        />
      ) : null}
      {issues.length > 0 ? (
        <div role="alert" className="space-y-1 rounded-xl border border-warning/45 bg-warning-bg p-2.5 text-small text-warning">
          {issues.map((issue) => (
            <div key={issue} className="flex gap-1.5">
              <AlertTriangle size={12} className="mt-0.5 shrink-0" />
              <span>{issue}</span>
            </div>
          ))}
        </div>
      ) : null}
      <InspectorSection title="输入">
        <NodeInputSummary node={node} nodes={nodes} edges={edges} />
      </InspectorSection>
      <InspectorSection title="参数">
        <NodeConfigEditor
          node={node}
          nodes={nodes}
          edges={edges}
          disabled={disabled}
          onChange={onChange}
          onRefChange={onRefChange}
          onMoveAssembleClip={onMoveAssembleClip}
          configOptions={configOptions}
        />
      </InspectorSection>
      <InspectorSection title="输出">
        <NodeOutputSummary node={node} onPreviewContent={onPreviewContent} />
      </InspectorSection>
      <details className="border-t border-book-border/70 pt-3 text-caption text-book-text-muted">
        <summary className="cursor-pointer">诊断信息</summary>
        <div className="mt-2 grid grid-cols-[64px_minmax(0,1fr)] gap-x-2 gap-y-1 rounded-xl bg-book-bg-tertiary/45 p-2.5">
          <span>节点编号</span>
          <span className="font-mono text-book-text-sub">{node.id}</span>
          <span>节点类型</span>
          <span className="break-all font-mono text-book-text-sub">{node.data.nodeKind}</span>
        </div>
      </details>
    </div>
  );
};

const InspectorSection: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <section className="space-y-2 border-t border-book-border/70 pt-3 first:border-t-0 first:pt-0">
    <h3 className="text-small font-medium text-book-text-muted">{title}</h3>
    {children}
  </section>
);

const NodeInputSummary: React.FC<{ node: StudioNode; nodes: StudioNode[]; edges: StudioEdge[] }> = ({
  node,
  nodes,
  edges,
}) => {
  const contract = useStudioNodeContract(node.data.nodeKind);
  if (!contract) {
    return <div className="text-small text-warning">节点合同缺失：{node.data.nodeKind}</div>;
  }
  const inputPorts = studioNodeInputPortsForData(node.data, contract);
  if (inputPorts.length === 0) {
    return <div className="text-small text-book-text-muted">该节点是输入源，不需要上游输入。</div>;
  }
  const nodesById = new Map(nodes.map((item) => [item.id, item]));
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
      {inputPorts.map((port) => {
        const incoming = edges.filter((edge) => edge.target === node.id && edge.targetHandle === port.id);
        return (
          <section key={port.id} className="p-2.5">
            <div className="mb-1 flex items-center justify-between gap-2">
              <span className="font-medium text-book-text-main">{port.label}</span>
              <span className="text-caption text-book-text-muted">{canvasDataTypeLabel(port.data_type)}</span>
            </div>
            {incoming.length > 0 ? (
              <div className="space-y-1">
                {incoming.map((edge) => {
                  const source = nodesById.get(edge.source);
                  return (
                    <div
                      key={edge.id}
                      className="truncate text-small text-book-text-sub"
                      title={`${source ? nodeDisplayTitle(source.data) : edge.source} · ${edge.sourceHandle ?? "默认输出"}`}
                    >
                      {source ? nodeDisplayTitle(source.data) : edge.source}
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className={`text-small ${port.required ? "text-warning" : "text-book-text-muted"}`}>
                {port.required ? "缺少必需输入" : "未连接"}
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
};

const NodeOutputSummary: React.FC<{ node: StudioNode; onPreviewContent: (node: StudioNode) => void }> = ({
  node,
  onPreviewContent,
}) => {
  const contract = useStudioNodeContract(node.data.nodeKind);
  const renderedPrompt = configText(node.data.config, "prompt_text").trim();
  const filePath = configText(node.data.config, "file_path").trim();
  const thumbnailPath = configText(node.data.config, "thumbnail_path").trim();
  const reference = node.data.refType ? "已关联" : "";
  const mediaAssetId =
    configPositiveNumber(node.data.config, "media_asset_id") ??
    nodeMediaAssetRefId(node.data);
  const generatedImageId =
    configPositiveNumber(node.data.config, "generated_image_id") ??
    nodeGeneratedImageRefId(node.data);
  const generatedVideoId =
    configPositiveNumber(node.data.config, "generated_video_id") ??
    nodeGeneratedVideoRefId(node.data);
  const outputPorts = contract ? studioNodeOutputPortsForData(node.data, contract) : [];

  if (!contract) {
    return <div className="text-small text-warning">节点合同缺失：{node.data.nodeKind}</div>;
  }
  if (contract.category === "operation") {
    return <FunctionalNodeOutputSummary outputPorts={outputPorts} />;
  }
  const output = configObject(node.data.config, "output");
  const outputRows = [
    ["引用", reference],
    ["媒体", mediaAssetId ? "已绑定" : ""],
    ["图片", generatedImageId ? "已生成" : ""],
    ["视频", generatedVideoId ? "已生成" : ""],
    ["文件", filePath ? fileName(filePath) : ""],
    ["缩略图", thumbnailPath ? fileName(thumbnailPath) : ""],
  ].filter((row): row is [string, string] => typeof row[1] === "string" && row[1].trim().length > 0);
  const technicalRows = [
    ["引用", node.data.refType ? `${node.data.refType} #${node.data.refId ?? "-"}` : ""],
    ["媒体编号", mediaAssetId ? String(mediaAssetId) : ""],
    ["图片编号", generatedImageId ? String(generatedImageId) : ""],
    ["视频编号", generatedVideoId ? String(generatedVideoId) : ""],
    ["文件路径", filePath],
    ["缩略图路径", thumbnailPath],
  ].filter((row): row is [string, string] => row[1].trim().length > 0);
  if (outputPorts.length === 0 && contract.outputs.length > 0) {
    return <div className="text-small text-book-text-muted">完成参数配置后显示可用输出端口。</div>;
  }
  if (outputPorts.length === 0 && outputRows.length === 0 && Object.keys(output).length === 0 && !renderedPrompt) {
    return <div className="text-small text-book-text-muted">该节点是终点节点，运行后在这里显示最终交付结果。</div>;
  }
  return (
    <div className="space-y-2">
      {nodeHasContentPreview(node) ? (
        <BookButton variant="secondary" size="sm" onClick={() => onPreviewContent(node)}>
          <Eye size={13} />
          查看内容
        </BookButton>
      ) : null}
      {outputPorts.length > 0 ? (
        <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 text-small">
          {outputPorts.map((port) => (
            <React.Fragment key={port.id}>
              <span className="text-book-text-muted">{port.label}</span>
              <span className="text-book-text-main">{canvasDataTypeLabel(port.data_type)}</span>
            </React.Fragment>
          ))}
        </div>
      ) : null}
      {renderedPrompt ? (
        <div className="a4n-flat-surface whitespace-pre-wrap break-words p-2.5 font-mono text-small leading-relaxed text-book-text-sub">
          {renderedPrompt}
        </div>
      ) : null}
      {outputRows.length > 0 ? (
        <div className="grid grid-cols-[80px_minmax(0,1fr)] gap-x-2 gap-y-1 text-small">
          {outputRows.map(([label, value]) => (
            <React.Fragment key={label}>
              <span className="text-book-text-muted">{label}</span>
              <span className="truncate text-book-text-main" title={value}>
                {value}
              </span>
            </React.Fragment>
          ))}
        </div>
      ) : null}
      {technicalRows.length > 0 ? (
        <details className="rounded-xl bg-book-bg-tertiary/35">
          <summary className="cursor-pointer px-2.5 py-2 text-caption text-book-text-muted">
            诊断信息
          </summary>
          <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 border-t border-book-border/40 px-2.5 py-2 text-caption">
            {technicalRows.map(([label, value]) => (
              <React.Fragment key={label}>
                <span className="text-book-text-muted">{label}</span>
                <span className="break-all font-mono text-book-text-sub">{value}</span>
              </React.Fragment>
            ))}
          </div>
        </details>
      ) : null}
    </div>
  );
};

const FunctionalNodeOutputSummary: React.FC<{
  outputPorts: Array<{ id: string; label: string; data_type: string }>;
}> = ({ outputPorts }) => {
  return (
    <div className="space-y-2">
      {outputPorts.length > 0 ? (
        <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 text-small">
          {outputPorts.map((port) => (
            <React.Fragment key={port.id}>
              <span className="text-book-text-muted">{port.label}</span>
              <span className="text-book-text-main">{canvasDataTypeLabel(port.data_type)}</span>
            </React.Fragment>
          ))}
        </div>
      ) : null}
      <div className="text-small text-book-text-muted">操作节点只传递运行输出；最终结果在输出节点中展示。</div>
    </div>
  );
};

export const EdgeInspector: React.FC<{ edge: StudioEdge }> = ({ edge }) => (
  <div className="space-y-2 text-xs">
    <div className="border-b border-book-border/70 pb-3">
      <div className="text-sm font-semibold text-book-text-main">{edgeRoleLabel(edge.data?.role ?? "")}</div>
      <div className="mt-1 text-small text-book-text-muted">连接两个节点并传递运行结果</div>
    </div>
    <details className="rounded-xl border border-book-border/65 bg-book-bg-paper/65">
      <summary className="cursor-pointer px-2.5 py-2 text-small text-book-text-muted">诊断信息</summary>
      <div className="grid grid-cols-[70px_minmax(0,1fr)] gap-x-2 gap-y-1 border-t border-book-border/50 px-2.5 py-2 text-small">
        <span className="text-book-text-muted">连接类型</span>
        <span className="font-mono text-book-text-main">{edge.data?.role ?? "-"}</span>
        <span className="text-book-text-muted">来源端口</span>
        <span className="truncate font-mono text-book-text-main">{edge.sourceHandle ?? "-"}</span>
        <span className="text-book-text-muted">目标端口</span>
        <span className="truncate font-mono text-book-text-main">{edge.targetHandle ?? "-"}</span>
        <span className="text-book-text-muted">顺序</span>
        <span className="truncate font-mono text-book-text-main">
          {edgeOrderIndex(edge) ?? "-"}
        </span>
      </div>
    </details>
  </div>
);

const NodeConfigEditor: React.FC<NodeConfigEditorProps> = ({
  node,
  nodes,
  edges,
  disabled,
  onChange,
  onRefChange,
  onMoveAssembleClip,
  configOptions,
}) => {
  const config = node.data.config;
  const inputRefType = defaultRefTypeForNode(node.data.nodeKind);
  const inputReferenceLabel = inputRefType
    ? CANVAS_INPUT_REFERENCE_LABELS[inputRefType]
    : undefined;
  if (inputRefType && inputReferenceLabel) {
    return (
      <ReferenceEditor
        label={inputReferenceLabel}
        refType={inputRefType}
        refId={node.data.refId}
        disabled={disabled}
        onChange={onRefChange}
      />
    );
  }
  if (node.data.nodeKind === "TextInputNode") {
    return (
      <label className="block text-caption text-book-text-muted">
        文本
        <textarea
          value={configText(config, "content")}
          onChange={(event) => onChange({ content: event.target.value, title: firstLine(event.target.value) })}
          disabled={disabled}
          rows={6}
          className={INSPECTOR_TEXTAREA_CLASS}
        />
      </label>
    );
  }
  if (isProbeNodeKind(node.data.nodeKind)) {
    return (
      <div className="space-y-3">
        <label className="block text-caption text-book-text-muted">
          标题
          <input
            type="text"
            value={configText(config, "title")}
            disabled={disabled}
            onChange={(event) => onChange({ title: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          />
        </label>
        <label className="block text-caption text-book-text-muted">
          展示模式
          <select
            value={configText(config, "display_mode") || "summary"}
            disabled={disabled}
            onChange={(event) => onChange({ display_mode: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value="summary">摘要</option>
            <option value="detail">详细</option>
          </select>
        </label>
        <ProbeObservationSummary config={config} displayMode={configText(config, "display_mode") || "summary"} />
      </div>
    );
  }
  if (node.data.nodeKind === "RenderTextNode") {
    return (
      <div className="space-y-2">
        <label className="block text-caption text-book-text-muted">
          模板
          <textarea
            value={configText(config, "template")}
            onChange={(event) => onChange({ template: event.target.value, title: firstLine(event.target.value) || "渲染提示词" })}
            disabled={disabled}
            rows={7}
            className={INSPECTOR_TEXTAREA_CLASS}
          />
        </label>
        <label className="block text-caption text-book-text-muted">
          文本类型
          <select
            value={configText(config, "output_kind")}
            disabled={disabled}
            onChange={(event) => onChange({ output_kind: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value={CANVAS_PROMPT_KIND}>画布提示词</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT}>视频文本</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT}>图片文本</option>
          </select>
        </label>
      </div>
    );
  }
  if (node.data.nodeKind === "LlmTextNode") {
    return (
      <div className="space-y-3">
        <ConfigSelect
          label="LLM配置"
          value={configPositiveNumber(config, "llm_config_id")}
          options={configOptions.llm}
          disabled={disabled}
          onChange={(value) => onChange({ llm_config_id: value })}
        />
        <label className="block text-caption text-book-text-muted">指令</label>
        <textarea
          value={configText(config, "instruction")}
          onChange={(event) => onChange({ instruction: event.target.value, title: firstLine(event.target.value) || "LLM改写" })}
          disabled={disabled}
          rows={5}
          className={`${INSPECTOR_TEXTAREA_CLASS} !mt-0`}
        />
        <label className="block text-caption text-book-text-muted">
          输出类型
          <select
            value={configText(config, "output_kind")}
            disabled={disabled}
            onChange={(event) => onChange({ output_kind: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value={CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT}>镜头脚本</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_STORYBOARD}>分镜</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION}>视觉描述</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_DIALOGUE}>对白</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN}>节奏计划</option>
          </select>
        </label>
      </div>
    );
  }
  if (node.data.nodeKind === "GenerateVideoNode") {
    const inputMode = parseGenerateVideoInputMode(config.input_mode);
    return (
      <div className="space-y-2">
        <ConfigSelect
          label="视频配置"
          value={configPositiveNumber(config, "video_config_id")}
          options={configOptions.video}
          disabled={disabled}
          onChange={(value) => onChange({ video_config_id: value })}
        />
        <label className="block text-caption text-book-text-muted">
          输入模式
          <select
            value={inputMode ?? ""}
            disabled={disabled}
            onChange={(event) => onChange({ input_mode: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            {inputMode == null ? <option value="" disabled>缺少输入模式</option> : null}
            <option value="text_only">仅提示词</option>
            <option value="first_frame">首帧</option>
            <option value="first_end_frame">首尾帧</option>
            <option value="reference_images">参考图</option>
          </select>
        </label>
        <label className="block text-caption text-book-text-muted">负面提示词</label>
        <textarea
          value={configText(config, "negative_prompt")}
          onChange={(event) => onChange({ negative_prompt: event.target.value })}
          disabled={disabled}
          rows={3}
          className={`${INSPECTOR_TEXTAREA_CLASS} !mt-0`}
        />
        <div className="grid grid-cols-2 gap-2">
          <NumberField label="宽" value={configNumber(config, "width")} disabled={disabled} onChange={(value) => onChange({ width: value })} />
          <NumberField label="高" value={configNumber(config, "height")} disabled={disabled} onChange={(value) => onChange({ height: value })} />
          <NumberField label="秒" value={configNumber(config, "duration_secs")} disabled={disabled} onChange={(value) => onChange({ duration_secs: value })} />
          <NumberField label="FPS" value={configNumber(config, "fps")} disabled={disabled} onChange={(value) => onChange({ fps: value })} />
        </div>
      </div>
    );
  }
  if (node.data.nodeKind === "GenerateImageNode") {
    return (
      <ConfigSelect
        label="图片配置"
        value={configPositiveNumber(config, "image_config_id")}
        options={configOptions.image}
        disabled={disabled}
        onChange={(value) => onChange({ image_config_id: value })}
      />
    );
  }
  if (node.data.nodeKind === "VideoOutputNode") {
    return (
      <label className="block text-caption text-book-text-muted">
        输出标题
        <input
          type="text"
          value={configText(config, "title")}
          disabled={disabled}
          onChange={(event) => onChange({ title: event.target.value })}
          className={INSPECTOR_CONTROL_CLASS}
        />
      </label>
    );
  }
  if (node.data.nodeKind === "TrimVideoNode") {
    return (
      <div className="grid grid-cols-2 gap-2">
        <NumberField label="开始秒" value={configNumber(config, "start_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ start_secs: value })} />
        <NumberField label="结束秒" value={configNumber(config, "end_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ end_secs: value })} />
      </div>
    );
  }
  if (node.data.nodeKind === "ExtractFrameNode") {
    const frameKind = configText(config, "frame_kind");
    return (
      <div className="space-y-3">
        <label className="block text-caption text-book-text-muted">
          抽帧类型
          <select
            value={frameKind}
            disabled={disabled}
            onChange={(event) => onChange({ frame_kind: event.target.value, at_secs: event.target.value === "time" ? configNumber(config, "at_secs") : undefined })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value="first">首帧</option>
            <option value="tail">尾帧</option>
            <option value="time">指定时间</option>
          </select>
        </label>
        {frameKind === "time" ? (
          <NumberField label="时间秒" value={configNumber(config, "at_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ at_secs: value })} />
        ) : null}
      </div>
    );
  }
  if (node.data.nodeKind === "AssembleVideoNode") {
    return (
      <div className="space-y-3">
        <label className="block text-caption text-book-text-muted">
          标题
          <input
            type="text"
            value={configText(config, "title")}
            disabled={disabled}
            onChange={(event) => onChange({ title: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          />
        </label>
        <AssembleClipOrderEditor
          assembleNode={node}
          nodes={nodes}
          edges={edges}
          disabled={disabled}
          onMove={onMoveAssembleClip}
        />
      </div>
    );
  }
  return null;
};

const ProbeObservationSummary: React.FC<{ config: Record<string, unknown>; displayMode: string }> = ({ config, displayMode }) => {
  const dataType = configText(config, "observed_data_type");
  const observedText = configText(config, "observed_text").trim();
  const filePath = configText(config, "file_path").trim();
  const thumbnailPath = configText(config, "thumbnail_path").trim();
  const mediaAssetId = configPositiveNumber(config, "media_asset_id");
  const generatedImageId = configPositiveNumber(config, "generated_image_id");
  const generatedVideoId = configPositiveNumber(config, "generated_video_id");
  const durationSecs = configNumber(config, "duration_secs");
  if (!dataType && !observedText && !filePath && !thumbnailPath && !mediaAssetId && !generatedImageId && !generatedVideoId) {
    return <div className="text-small text-book-text-muted">尚未观察到运行结果</div>;
  }
  return (
    <div className="a4n-flat-surface p-2.5 text-small">
      <div className="grid grid-cols-[64px_minmax(0,1fr)] gap-x-2 gap-y-1">
        <span className="text-book-text-muted">类型</span>
        <span className="text-book-text-main">{canvasDataTypeLabel(dataType)}</span>
        {generatedImageId ? (
          <>
            <span className="text-book-text-muted">图片</span>
            <span className="text-book-text-main">已生成</span>
          </>
        ) : null}
        {generatedVideoId ? (
          <>
            <span className="text-book-text-muted">视频</span>
            <span className="text-book-text-main">已生成</span>
          </>
        ) : null}
        {mediaAssetId ? (
          <>
            <span className="text-book-text-muted">媒体</span>
            <span className="text-book-text-main">已绑定</span>
          </>
        ) : null}
        {durationSecs !== "" ? (
          <>
            <span className="text-book-text-muted">时长</span>
            <span className="text-book-text-main">{durationSecs}s</span>
          </>
        ) : null}
        {filePath || thumbnailPath ? (
          <>
            <span className="text-book-text-muted">文件</span>
            <span className="truncate text-book-text-main" title={filePath || thumbnailPath}>
              {(filePath || thumbnailPath).split("/").pop()}
            </span>
          </>
        ) : null}
      </div>
      {observedText ? (
        displayMode === "detail" ? (
          <div className="mt-2 whitespace-pre-wrap border-t border-book-border/40 pt-2 leading-relaxed text-book-text-sub">
            {observedText}
          </div>
        ) : (
          <div className="mt-2 line-clamp-3 border-t border-book-border/40 pt-2 leading-relaxed text-book-text-sub">
            {observedText}
          </div>
        )
      ) : null}
    </div>
  );
};

interface AssembleClipOrderEditorProps {
  assembleNode: StudioNode;
  nodes: StudioNode[];
  edges: StudioEdge[];
  disabled: boolean;
  onMove: (edgeId: string, direction: -1 | 1) => void;
}

const AssembleClipOrderEditor: React.FC<AssembleClipOrderEditorProps> = ({
  assembleNode,
  nodes,
  edges,
  disabled,
  onMove,
}) => {
  const clips = assembleClipEdges(assembleNode.id, nodes, edges);
  return (
    <div className="space-y-2">
      <div className="text-caption font-medium text-book-text-muted">成片轨道</div>
      {clips.length === 0 ? (
        <div className="text-small text-book-text-muted">暂无片段连线</div>
      ) : (
        <ol className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
          {clips.map(({ edge, source, orderIndex }, index) => (
            <li
              key={edge.id}
              className="grid grid-cols-[24px_minmax(0,1fr)_72px] items-center gap-2 px-2.5 py-2"
            >
              <span className="font-mono text-small text-book-text-muted">{orderIndex ?? index + 1}</span>
              <div className="min-w-0">
                <div className="truncate text-small text-book-text-main" title={source.data.label}>
                  {source.data.label}
                </div>
              </div>
              <div className="flex items-center justify-end gap-1">
                <BookIconButton
                  label="上移片段"
                  variant="ghost"
                  size="sm"
                  disabled={disabled || index === 0}
                  onClick={() => onMove(edge.id, -1)}
                  className="!h-8 !w-8"
                >
                  <ArrowUp size={12} />
                </BookIconButton>
                <BookIconButton
                  label="下移片段"
                  variant="ghost"
                  size="sm"
                  disabled={disabled || index === clips.length - 1}
                  onClick={() => onMove(edge.id, 1)}
                  className="!h-8 !w-8"
                >
                  <ArrowDown size={12} />
                </BookIconButton>
              </div>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
};

interface ReferenceEditorProps {
  label: string;
  refType: string;
  refId: number | null;
  disabled: boolean;
  onChange: (refType: string | null, refId: number | null) => void;
}

const ReferenceEditor: React.FC<ReferenceEditorProps> = ({ label, refType, refId, disabled, onChange }) => (
  <details className="a4n-flat-surface overflow-hidden">
    <summary className="cursor-pointer px-2.5 py-2.5 text-small text-book-text-muted">高级引用</summary>
    <div className="border-t border-book-border/60 p-2">
      <NumberField
        label={label}
        value={refId ?? ""}
        disabled={disabled}
        onChange={(value) => onChange(refType, value)}
      />
    </div>
  </details>
);

interface NumberFieldProps {
  label: string;
  value: number | "";
  disabled: boolean;
  min?: number;
  onChange: (value: number | null) => void;
}

const NumberField: React.FC<NumberFieldProps> = ({ label, value, disabled, min = 1, onChange }) => (
  <label className="block text-caption text-book-text-muted">
    {label}
    <input
      type="number"
      min={min}
      value={value}
      disabled={disabled}
      onChange={(event) => onChange(numberOrNull(event.target.value, min))}
      className={INSPECTOR_CONTROL_CLASS}
    />
  </label>
);

interface ConfigSelectProps {
  label: string;
  value: number | null;
  options: StudioConfigOption[];
  disabled: boolean;
  onChange: (value: number | null) => void;
}

const ConfigSelect: React.FC<ConfigSelectProps> = ({ label, value, options, disabled, onChange }) => {
  const selectedOption = value == null ? null : options.find((option) => option.id === value) ?? null;
  const selectedMissing = value != null && selectedOption == null;
  return (
    <label className="block text-caption text-book-text-muted">
      {label}
      <select
        value={value ?? ""}
        disabled={disabled || (options.length === 0 && !selectedMissing)}
        onChange={(event) => onChange(numberOrNull(event.target.value, 1))}
        className={INSPECTOR_CONTROL_CLASS}
      >
        <option value="">{options.length === 0 ? "暂无可用配置" : "使用默认配置"}</option>
        {selectedMissing ? (
          <option value={value}>已删除或不可用配置</option>
        ) : null}
        {options.map((option) => (
          <option key={option.id} value={option.id}>
            {configOptionLabel(option)}
          </option>
        ))}
      </select>
      {selectedOption ? (
        <div className="mt-1 truncate text-caption text-book-text-sub" title={configOptionLabel(selectedOption)}>
          当前：{configOptionLabel(selectedOption)}
        </div>
      ) : selectedMissing ? (
        <div className="mt-1 text-caption text-warning" title={`配置编号 ${value}`}>
          当前配置已不可用，请重新选择。
        </div>
      ) : null}
    </label>
  );
};

function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
