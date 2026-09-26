import { ThemedSelect } from "../../components/ui/ThemedSelect";
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
import { ZoomableImage } from "../../components/ui/ImageLightbox";
import { ImageSizeSelect } from "../../components/image/ImageSizeSelect";
import { useI18n } from "../../i18n";
import {
  canvasDataTypeLabel,
  edgeRoleLabel,
  nodeDisplayTitle,
  nodePreviewSrc,
  nodeValidationIssues,
} from "./StudioCanvasNode";
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
import { isProbeNodeKind, nodeHasContentPreview } from "./canvasNodeKinds";
import {
  CANVAS_REF_TYPE_GENERATED_IMAGE,
  CANVAS_REF_TYPE_GENERATED_VIDEO,
  CANVAS_REF_TYPE_MEDIA_ASSET,
  defaultRefTypeForNode,
  nodeGeneratedImageRefId,
  nodeGeneratedVideoRefId,
  nodeMediaAssetRefId,
  nodeRefId,
} from "./canvasNodeRefs";
import {
  parseGenerateVideoInputMode,
  studioPortLabel,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  useStudioNodeContract,
} from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

const INSPECTOR_CONTROL_CLASS =
  "pyqt-input mt-1 w-full !px-2.5 !py-1.5 text-xs";
const INSPECTOR_TEXTAREA_CLASS = `${INSPECTOR_CONTROL_CLASS} resize-none leading-relaxed`;
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
  const { t } = useI18n();
  const issues = nodeValidationIssues(node.data, t);
  const previewSrc = nodePreviewSrc(node.data);
  return (
    <div className="space-y-3 text-xs">
      {previewSrc ? (
        <ZoomableImage
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
      <InspectorSection title={t("video.canvas.inspector.input")}>
        <NodeInputSummary node={node} nodes={nodes} edges={edges} />
      </InspectorSection>
      <InspectorSection title={t("video.canvas.inspector.parameters")}>
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
      <InspectorSection title={t("video.canvas.inspector.output")}>
        <NodeOutputSummary node={node} onPreviewContent={onPreviewContent} />
      </InspectorSection>
      <details className="border-t border-book-border/70 pt-3 text-caption text-book-text-muted">
        <summary className="cursor-pointer">{t("video.canvas.inspector.diagnostics")}</summary>
        <div className="mt-2 grid grid-cols-[64px_minmax(0,1fr)] gap-x-2 gap-y-1 rounded-xl bg-book-bg-tertiary/45 p-2.5">
          <span>{t("video.canvas.inspector.nodeId")}</span>
          <span className="font-mono text-book-text-sub">{node.id}</span>
          <span>{t("video.canvas.inspector.nodeType")}</span>
          <span className="break-all font-mono text-book-text-sub">{node.data.nodeKind}</span>
        </div>
      </details>
    </div>
  );
};

const InspectorSection: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <section className="space-y-2 border-t border-book-border/70 pt-3 first:border-t-0 first:pt-0 has-[>h3:only-child]:hidden">
    <h3 className="text-small font-medium text-book-text-muted">{title}</h3>
    {children}
  </section>
);

const NodeInputSummary: React.FC<{ node: StudioNode; nodes: StudioNode[]; edges: StudioEdge[] }> = ({
  node,
  nodes,
  edges,
}) => {
  const { t } = useI18n();
  const contract = useStudioNodeContract(node.data.nodeKind);
  if (!contract) {
    return <div className="text-small text-warning">{t("video.canvas.inspector.missingContract", { kind: node.data.nodeKind })}</div>;
  }
  const inputPorts = studioNodeInputPortsForData(node.data, contract);
  if (inputPorts.length === 0) {
    return <div className="text-small text-book-text-muted">{t("video.canvas.inspector.inputNodeHint")}</div>;
  }
  const nodesById = new Map(nodes.map((item) => [item.id, item]));
  const incomingByPort = new Map<string, StudioEdge[]>();
  for (const edge of edges) {
    if (edge.target !== node.id || edge.targetHandle == null) continue;
    const rows = incomingByPort.get(edge.targetHandle) ?? [];
    rows.push(edge);
    incomingByPort.set(edge.targetHandle, rows);
  }
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
      {inputPorts.map((port) => {
        const incoming = incomingByPort.get(port.id) ?? [];
        return (
          <section key={port.id} className="p-2.5">
            <div className="mb-1 flex items-center justify-between gap-2">
              <span className="font-medium text-book-text-main">{studioPortLabel(node.data.nodeKind, port, t)}</span>
              <span className="text-caption text-book-text-muted">{canvasDataTypeLabel(port.data_type, t)}</span>
            </div>
            {incoming.length > 0 ? (
              <div className="space-y-1">
                {incoming.map((edge) => {
                  const source = nodesById.get(edge.source);
                  return (
                    <div
                      key={edge.id}
                      className="truncate text-small text-book-text-sub"
                      title={`${source ? nodeDisplayTitle(source.data, t) : edge.source} · ${edge.sourceHandle ?? t("video.canvas.inspector.defaultOutput")}`}
                    >
                      {source ? nodeDisplayTitle(source.data, t) : edge.source}
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className={`text-small ${port.required ? "text-warning" : "text-book-text-muted"}`}>
                {port.required ? t("video.canvas.inspector.requiredInput") : t("video.canvas.inspector.notConnected")}
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
  const { t } = useI18n();
  const contract = useStudioNodeContract(node.data.nodeKind);
  const renderedPrompt = configText(node.data.config, "prompt_text").trim();
  const filePath = configText(node.data.config, "file_path").trim();
  const thumbnailPath = configText(node.data.config, "thumbnail_path").trim();
  const reference = nodeRefId(node.data) != null ? t("video.canvas.inspector.linked") : "";
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
    return <div className="text-small text-warning">{t("video.canvas.inspector.missingContract", { kind: node.data.nodeKind })}</div>;
  }
  if (contract.category === "operation") {
    return <FunctionalNodeOutputSummary nodeKind={node.data.nodeKind} outputPorts={outputPorts} />;
  }
  const output = configObject(node.data.config, "output");
  const outputRows = [
    [t("video.canvas.inspector.reference"), reference],
    [t("video.canvas.inspector.media"), mediaAssetId ? t("video.canvas.inspector.bound") : ""],
    [t("video.canvas.inspector.image"), generatedImageId ? t("video.canvas.inspector.generated") : ""],
    [t("video.canvas.inspector.video"), generatedVideoId ? t("video.canvas.inspector.generated") : ""],
    [t("video.canvas.inspector.file"), filePath ? fileName(filePath) : ""],
    [t("video.canvas.inspector.thumbnail"), thumbnailPath ? fileName(thumbnailPath) : ""],
  ].filter((row): row is [string, string] => typeof row[1] === "string" && row[1].trim().length > 0);
  const technicalRows = [
    [t("video.canvas.inspector.reference"), node.data.refType ? `${node.data.refType} #${node.data.refId ?? "-"}` : ""],
    [t("video.canvas.inspector.mediaId"), mediaAssetId ? String(mediaAssetId) : ""],
    [t("video.canvas.inspector.imageId"), generatedImageId ? String(generatedImageId) : ""],
    [t("video.canvas.inspector.videoId"), generatedVideoId ? String(generatedVideoId) : ""],
    [t("video.canvas.inspector.filePath"), filePath],
    [t("video.canvas.inspector.thumbnailPath"), thumbnailPath],
  ].filter((row): row is [string, string] => row[1].trim().length > 0);
  if (outputPorts.length === 0 && contract.outputs.length > 0) {
    return <div className="text-small text-book-text-muted">{t("video.canvas.inspector.outputPortsReady")}</div>;
  }
  if (outputPorts.length === 0 && outputRows.length === 0 && Object.keys(output).length === 0 && !renderedPrompt) {
    return <div className="text-small text-book-text-muted">{t("video.canvas.inspector.terminalOutput")}</div>;
  }
  return (
    <div className="space-y-2">
      {nodeHasContentPreview(node) ? (
        <BookButton variant="secondary" size="sm" onClick={() => onPreviewContent(node)}>
          <Eye size={13} />
          {t("video.canvas.inspector.viewContent")}
        </BookButton>
      ) : null}
      {outputPorts.length > 0 ? (
        <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 text-small">
          {outputPorts.map((port) => (
            <React.Fragment key={port.id}>
              <span className="text-book-text-muted">{studioPortLabel(node.data.nodeKind, port, t)}</span>
              <span className="text-book-text-main">{canvasDataTypeLabel(port.data_type, t)}</span>
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
            {t("video.canvas.inspector.diagnostics")}
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
  nodeKind: string;
  outputPorts: Array<{ id: string; label: string; data_type: string }>;
}> = ({ nodeKind, outputPorts }) => {
  const { t } = useI18n();
  return (
    <div className="space-y-2">
      {outputPorts.length > 0 ? (
        <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 text-small">
          {outputPorts.map((port) => (
            <React.Fragment key={port.id}>
              <span className="text-book-text-muted">{studioPortLabel(nodeKind, port, t)}</span>
              <span className="text-book-text-main">{canvasDataTypeLabel(port.data_type, t)}</span>
            </React.Fragment>
          ))}
        </div>
      ) : null}
      <div className="text-small text-book-text-muted">{t("video.canvas.inspector.operationOutput")}</div>
    </div>
  );
};

export const EdgeInspector: React.FC<{ edge: StudioEdge; nodes: StudioNode[] }> = ({ edge, nodes }) => {
  const { t } = useI18n();
  const source = nodes.find((node) => node.id === edge.source);
  const target = nodes.find((node) => node.id === edge.target);
  return (
    <div className="space-y-2 text-xs">
      <div className="border-b border-book-border/70 pb-3">
        <div className="text-sm font-semibold text-book-text-main">{edgeRoleLabel(edge.data?.role ?? "", t)}</div>
        <div className="mt-1 break-words text-small leading-relaxed text-book-text-main">{t("video.canvas.inspector.edgeDescription", {
          source: source ? nodeDisplayTitle(source.data, t) : edge.source,
          target: target ? nodeDisplayTitle(target.data, t) : edge.target,
        })}</div>
      </div>
      <details className="rounded-xl border border-book-border/65 bg-book-bg-paper/65">
        <summary className="cursor-pointer px-2.5 py-2 text-small text-book-text-muted">{t("video.canvas.inspector.diagnostics")}</summary>
        <div className="grid grid-cols-[70px_minmax(0,1fr)] gap-x-2 gap-y-1 border-t border-book-border/50 px-2.5 py-2 text-small">
          <span className="text-book-text-muted">{t("video.canvas.inspector.connectionType")}</span>
          <span className="font-mono text-book-text-main">{edge.data?.role ?? "-"}</span>
          <span className="text-book-text-muted">{t("video.canvas.inspector.sourcePort")}</span>
          <span className="truncate font-mono text-book-text-main">{edge.sourceHandle ?? "-"}</span>
          <span className="text-book-text-muted">{t("video.canvas.inspector.targetPort")}</span>
          <span className="truncate font-mono text-book-text-main">{edge.targetHandle ?? "-"}</span>
          <span className="text-book-text-muted">{t("video.canvas.inspector.order")}</span>
          <span className="truncate font-mono text-book-text-main">{edgeOrderIndex(edge) ?? "-"}</span>
        </div>
      </details>
    </div>
  );
};

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
  const { t } = useI18n();
  const config = node.data.config;
  const inputRefType = defaultRefTypeForNode(node.data.nodeKind);
  const inputReferenceLabel = inputRefType ? canvasInputReferenceLabel(inputRefType, t) : undefined;
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
        {t("video.canvas.inspector.text")}
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
          {t("video.canvas.inspector.title")}
          <input
            type="text"
            value={configText(config, "title")}
            disabled={disabled}
            onChange={(event) => onChange({ title: event.target.value })}
            className={INSPECTOR_CONTROL_CLASS}
          />
        </label>
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.displayMode")}
          <ThemedSelect
            value={configText(config, "display_mode") || "summary"}
            disabled={disabled}
            onChange={(selectedValue) => onChange({ display_mode: selectedValue })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value="summary">{t("video.canvas.inspector.summary")}</option>
            <option value="detail">{t("video.canvas.inspector.detail")}</option>
          </ThemedSelect>
        </label>
        <ProbeObservationSummary config={config} displayMode={configText(config, "display_mode") || "summary"} />
      </div>
    );
  }
  if (node.data.nodeKind === "RenderTextNode") {
    return (
      <div className="space-y-2">
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.template")}
          <textarea
            value={configText(config, "template")}
            onChange={(event) => onChange({ template: event.target.value, title: firstLine(event.target.value) || t("video.canvas.inspector.canvasPrompt") })}
            disabled={disabled}
            rows={7}
            className={INSPECTOR_TEXTAREA_CLASS}
          />
        </label>
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.textType")}
          <ThemedSelect
            value={configText(config, "output_kind")}
            disabled={disabled}
            onChange={(selectedValue) => onChange({ output_kind: selectedValue })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value={CANVAS_PROMPT_KIND}>{t("video.canvas.inspector.canvasPrompt")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT}>{t("video.canvas.inspector.videoText")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT}>{t("video.canvas.inspector.imageText")}</option>
          </ThemedSelect>
        </label>
      </div>
    );
  }
  if (node.data.nodeKind === "LlmTextNode") {
    return (
      <div className="space-y-3">
        <ConfigSelect
          label={t("video.canvas.inspector.llmConfig")}
          value={configPositiveNumber(config, "llm_config_id")}
          options={configOptions.llm}
          disabled={disabled}
          onChange={(value) => onChange({ llm_config_id: value })}
        />
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.instruction")}
          <textarea
            value={configText(config, "instruction")}
            onChange={(event) => onChange({ instruction: event.target.value, title: firstLine(event.target.value) || t("video.canvas.inspector.instruction") })}
            disabled={disabled}
            rows={5}
            className={INSPECTOR_TEXTAREA_CLASS}
          />
        </label>
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.outputType")}
          <ThemedSelect
            value={configText(config, "output_kind")}
            disabled={disabled}
            onChange={(selectedValue) => onChange({ output_kind: selectedValue })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value={CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT}>{t("video.canvas.inspector.shotScript")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_STORYBOARD}>{t("video.canvas.inspector.storyboard")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION}>{t("video.canvas.inspector.visualDescription")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_DIALOGUE}>{t("video.canvas.inspector.dialogue")}</option>
            <option value={CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN}>{t("video.canvas.inspector.rhythmPlan")}</option>
          </ThemedSelect>
        </label>
      </div>
    );
  }
  if (node.data.nodeKind === "GenerateVideoNode") {
    const inputMode = parseGenerateVideoInputMode(config.input_mode);
    return (
      <div className="space-y-2">
        <ConfigSelect
          label={t("video.canvas.inspector.videoConfig")}
          value={configPositiveNumber(config, "video_config_id")}
          options={configOptions.video}
          disabled={disabled}
          onChange={(value) => onChange({ video_config_id: value })}
        />
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.inputMode")}
          <ThemedSelect
            value={inputMode ?? ""}
            disabled={disabled}
            onChange={(selectedValue) => onChange({ input_mode: selectedValue })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            {inputMode == null ? <option value="" disabled>{t("video.canvas.inspector.missingInputMode")}</option> : null}
            <option value="text_only">{t("video.canvas.inspector.textOnly")}</option>
            <option value="first_frame">{t("video.canvas.inspector.firstFrame")}</option>
            <option value="first_end_frame">{t("video.canvas.inspector.firstEndFrame")}</option>
            <option value="reference_images">{t("video.canvas.inspector.referenceImages")}</option>
          </ThemedSelect>
        </label>
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.negativePrompt")}
          <textarea
            value={configText(config, "negative_prompt")}
            onChange={(event) => onChange({ negative_prompt: event.target.value })}
            disabled={disabled}
            rows={3}
            className={INSPECTOR_TEXTAREA_CLASS}
          />
        </label>
        <div className="grid grid-cols-2 gap-2">
          <NumberField label={t("video.canvas.inspector.width")} value={configNumber(config, "width")} disabled={disabled} onChange={(value) => onChange({ width: value })} />
          <NumberField label={t("video.canvas.inspector.height")} value={configNumber(config, "height")} disabled={disabled} onChange={(value) => onChange({ height: value })} />
          <NumberField label={t("video.canvas.inspector.seconds")} value={configNumber(config, "duration_secs")} disabled={disabled} onChange={(value) => onChange({ duration_secs: value })} />
          <NumberField label="FPS" value={configNumber(config, "fps")} disabled={disabled} onChange={(value) => onChange({ fps: value })} />
        </div>
      </div>
    );
  }
  if (node.data.nodeKind === "GenerateImageNode") {
    const imageConfigId = configPositiveNumber(config, "image_config_id");
    const imageConfig =
      imageConfigId == null
        ? configOptions.image.find((option) => option.isDefault) ?? null
        : configOptions.image.find((option) => option.id === imageConfigId) ?? null;
    const supportedSizes = imageConfig?.supportedSizes ?? [];
    const width = configPositiveNumber(config, "width");
    const height = configPositiveNumber(config, "height");
    const selectedSize =
      supportedSizes.find((size) => size.width === width && size.height === height) ?? null;
    return (
      <div className="space-y-2">
        <ConfigSelect
          label={t("video.canvas.inspector.imageConfig")}
          value={imageConfigId}
          options={configOptions.image}
          disabled={disabled}
          onChange={(value) => {
            const nextConfig =
              value == null
                ? configOptions.image.find((option) => option.isDefault) ?? null
                : configOptions.image.find((option) => option.id === value) ?? null;
            const nextSize =
              nextConfig?.supportedSizes?.find(
                (size) => size.width === width && size.height === height,
              );
            onChange({
              image_config_id: value,
              ...(nextSize ? { width: nextSize.width, height: nextSize.height } : {}),
            });
          }}
        />
        <ImageSizeSelect
          label={t("settingsForm.selectImageSize")}
          value={selectedSize}
          sizes={supportedSizes}
          disabled={disabled || imageConfig == null}
          onChange={(size) => onChange({ width: size.width, height: size.height })}
        />
      </div>
    );
  }
  if (node.data.nodeKind === "VideoOutputNode") {
    return (
      <label className="block text-caption text-book-text-muted">
        {t("video.canvas.inspector.outputTitle")}
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
        <NumberField label={t("video.canvas.inspector.startSeconds")} value={configNumber(config, "start_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ start_secs: value })} />
        <NumberField label={t("video.canvas.inspector.endSeconds")} value={configNumber(config, "end_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ end_secs: value })} />
      </div>
    );
  }
  if (node.data.nodeKind === "ExtractFrameNode") {
    const frameKind = configText(config, "frame_kind");
    return (
      <div className="space-y-3">
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.frameType")}
          <ThemedSelect
            value={frameKind}
            disabled={disabled}
            onChange={(selectedValue) => onChange({ frame_kind: selectedValue, at_secs: selectedValue === "time" ? configNumber(config, "at_secs") : undefined })}
            className={INSPECTOR_CONTROL_CLASS}
          >
            <option value="first">{t("video.canvas.inspector.firstFrame")}</option>
            <option value="tail">{t("video.canvas.inspector.tailFrame")}</option>
            <option value="time">{t("video.canvas.inspector.specifiedTime")}</option>
          </ThemedSelect>
        </label>
        {frameKind === "time" ? (
          <NumberField label={t("video.canvas.inspector.timeSeconds")} value={configNumber(config, "at_secs")} disabled={disabled} min={0} onChange={(value) => onChange({ at_secs: value })} />
        ) : null}
      </div>
    );
  }
  if (node.data.nodeKind === "AssembleVideoNode") {
    return (
      <div className="space-y-3">
        <label className="block text-caption text-book-text-muted">
          {t("video.canvas.inspector.title")}
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
  const { t } = useI18n();
  const dataType = configText(config, "observed_data_type");
  const observedText = configText(config, "observed_text").trim();
  const filePath = configText(config, "file_path").trim();
  const thumbnailPath = configText(config, "thumbnail_path").trim();
  const mediaAssetId = configPositiveNumber(config, "media_asset_id");
  const generatedImageId = configPositiveNumber(config, "generated_image_id");
  const generatedVideoId = configPositiveNumber(config, "generated_video_id");
  const durationSecs = configNumber(config, "duration_secs");
  if (!dataType && !observedText && !filePath && !thumbnailPath && !mediaAssetId && !generatedImageId && !generatedVideoId) {
    return <div className="text-small text-book-text-muted">{t("video.canvas.inspector.observedEmpty")}</div>;
  }
  return (
    <div className="a4n-flat-surface p-2.5 text-small">
      <div className="grid grid-cols-[64px_minmax(0,1fr)] gap-x-2 gap-y-1">
        <span className="text-book-text-muted">{t("video.canvas.preview.type")}</span>
        <span className="text-book-text-main">{canvasDataTypeLabel(dataType, t)}</span>
        {generatedImageId ? (
          <>
            <span className="text-book-text-muted">{t("video.canvas.inspector.image")}</span>
            <span className="text-book-text-main">{t("video.canvas.inspector.generated")}</span>
          </>
        ) : null}
        {generatedVideoId ? (
          <>
            <span className="text-book-text-muted">{t("video.canvas.inspector.video")}</span>
            <span className="text-book-text-main">{t("video.canvas.inspector.generated")}</span>
          </>
        ) : null}
        {mediaAssetId ? (
          <>
            <span className="text-book-text-muted">{t("video.canvas.inspector.media")}</span>
            <span className="text-book-text-main">{t("video.canvas.inspector.bound")}</span>
          </>
        ) : null}
        {durationSecs !== "" ? (
          <>
            <span className="text-book-text-muted">{t("video.canvas.inspector.duration")}</span>
            <span className="text-book-text-main">{durationSecs}s</span>
          </>
        ) : null}
        {filePath || thumbnailPath ? (
          <>
            <span className="text-book-text-muted">{t("video.canvas.inspector.file")}</span>
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
  const { t } = useI18n();
  const clips = assembleClipEdges(assembleNode.id, nodes, edges);
  return (
    <div className="space-y-2">
      <div className="text-caption font-medium text-book-text-muted">{t("video.canvas.inspector.finishedTrack")}</div>
      {clips.length === 0 ? (
        <div className="text-small text-book-text-muted">{t("video.canvas.inspector.noClipEdges")}</div>
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
                  label={t("video.canvas.inspector.moveClipUp")}
                  variant="ghost"
                  size="sm"
                  disabled={disabled || index === 0}
                  onClick={() => onMove(edge.id, -1)}
                  className="!h-8 !w-8"
                >
                  <ArrowUp size={12} />
                </BookIconButton>
                <BookIconButton
                  label={t("video.canvas.inspector.moveClipDown")}
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

const ReferenceEditor: React.FC<ReferenceEditorProps> = ({ label, refType, refId, disabled, onChange }) => {
  const { t } = useI18n();
  return (
    <details className="a4n-flat-surface overflow-hidden">
      <summary className="cursor-pointer px-2.5 py-2.5 text-small text-book-text-muted">{t("video.canvas.inspector.advancedReference")}</summary>
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
};

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
  const { t } = useI18n();
  const selectedOption = value == null ? null : options.find((option) => option.id === value) ?? null;
  const selectedMissing = value != null && selectedOption == null;
  return (
    <label className="block text-caption text-book-text-muted">
      {label}
      <ThemedSelect
        value={value ?? ""}
        disabled={disabled || (options.length === 0 && !selectedMissing)}
        onChange={(selectedValue) => onChange(numberOrNull(selectedValue, 1))}
        className={INSPECTOR_CONTROL_CLASS}
      >
        <option value="">{options.length === 0 ? t("video.canvas.inspector.noConfigAvailable") : t("video.canvas.inspector.defaultConfig")}</option>
        {selectedMissing ? (
          <option value={value}>{t("video.canvas.inspector.unavailableConfig")}</option>
        ) : null}
        {options.map((option) => (
          <option key={option.id} value={option.id}>
            {configOptionLabel(option, t)}
          </option>
        ))}
      </ThemedSelect>
      {selectedOption ? (
        <div className="mt-1 truncate text-caption text-book-text-sub" title={configOptionLabel(selectedOption, t)}>
          {t("video.canvas.inspector.currentConfig", { name: configOptionLabel(selectedOption, t) })}
        </div>
      ) : selectedMissing ? (
        <div className="mt-1 text-caption text-warning" title={String(value)}>
          {t("video.canvas.inspector.unavailableCurrentConfig")}
        </div>
      ) : null}
    </label>
  );
};

function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function canvasInputReferenceLabel(refType: string, t: ReturnType<typeof useI18n>["t"]): string | undefined {
  switch (refType) {
    case CANVAS_REF_TYPE_GENERATED_IMAGE:
      return t("video.canvas.inspector.generatedImageReference");
    case CANVAS_REF_TYPE_GENERATED_VIDEO:
      return t("video.canvas.inspector.generatedVideoReference");
    case CANVAS_REF_TYPE_MEDIA_ASSET:
      return t("video.canvas.inspector.mediaAssetReference");
    default:
      return undefined;
  }
}
