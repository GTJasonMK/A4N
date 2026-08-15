import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FileText,
  Maximize2,
  Network,
  RefreshCw,
  ZoomIn,
  ZoomOut, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  getProjectAssetDependencyGraph,
  getProjectAssetDependencyGraphNodeDetail,
  type AssetDependencyGraph,
  type AssetGraphNodeDetail,
  type AssetGraphNode,
  type GenerationInputSource,
  type AssetLineageRecord,
  type AssetLineageRef,
} from "../../../api/project";
import { extractErrorMessage } from "../../../api";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { THEME_APPLIED_EVENT } from "../../../theme/activeTheme";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import {
  assetTypeLabel,
  buildLayout,
  clamp,
  drawGraph,
  hitTestNode,
  isVideoChainNode,
  kindLabel,
  nodeCenter,
  preferredInitialNodeId,
  summarizeGraph,
  type PositionedNode,
} from "./graphCanvas";

interface AssetDependencyGraphDialogProps {
  open: boolean;
  projectId: number;
  onClose: () => void;
}

interface InputViewerState {
  recordLabel: string;
  inputs: GenerationInputSource[];
}

const MIN_ZOOM = 0.35;
const MAX_ZOOM = 2.25;
const ZOOM_STEP = 0.15;
const INLINE_TEXT_MAX_CHARS = 120;
const AUTO_OPEN_TEXT_MAX_CHARS = 360;
const AUTO_OPEN_ARRAY_MAX_ITEMS = 4;
const AUTO_OPEN_OBJECT_MAX_FIELDS = 8;
export const AssetDependencyGraphDialog: React.FC<
  AssetDependencyGraphDialogProps
> = ({ open, projectId, onClose }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const networkScrollRef = useRef<HTMLDivElement | null>(null);
  const graphRequestSeqRef = useRef(0);
  const detailRequestSeqRef = useRef(0);
  const graphLoadingRef = useRef(false);
  const [graph, setGraph] = useState<AssetDependencyGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [nodeDetail, setNodeDetail] = useState<AssetGraphNodeDetail | null>(
    null,
  );
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [selectedRecordId, setSelectedRecordId] = useState<string | null>(null);
  const [promptViewer, setPromptViewer] = useState<InputViewerState | null>(
    null,
  );
  const [themeVersion, setThemeVersion] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [mobilePane, setMobilePane] = useState<"graph" | "details">("graph");

  const loadGraph = useCallback(async () => {
    if (graphLoadingRef.current) return;
    graphLoadingRef.current = true;
    const requestSeq = graphRequestSeqRef.current + 1;
    graphRequestSeqRef.current = requestSeq;
    setLoading(true);
    setError(null);
    detailRequestSeqRef.current += 1;
    setGraph(null);
    setSelectedId(null);
    setNodeDetail(null);
    setSelectedRecordId(null);
    setDetailError(null);
    setDetailLoading(false);
    setPromptViewer(null);
    try {
      const next = await getProjectAssetDependencyGraph(projectId);
      if (graphRequestSeqRef.current !== requestSeq) return;
      setGraph(next);
      setSelectedId((current) => {
        if (current && next.nodes.some((node) => node.id === current))
          return current;
        return preferredInitialNodeId(next);
      });
    } catch (err) {
      if (graphRequestSeqRef.current !== requestSeq) return;
      setError(extractErrorMessage(err, "加载依赖图失败"));
    } finally {
      if (graphRequestSeqRef.current === requestSeq) {
        setLoading(false);
        graphLoadingRef.current = false;
      }
    }
  }, [projectId]);

  useLayoutEffect(() => {
    graphRequestSeqRef.current += 1;
    detailRequestSeqRef.current += 1;
    graphLoadingRef.current = false;
    setGraph(null);
    setSelectedId(null);
    setNodeDetail(null);
    setSelectedRecordId(null);
    setDetailError(null);
    setDetailLoading(false);
    setPromptViewer(null);
    setError(null);
    setLoading(false);
    setMobilePane("graph");
  }, [open, projectId]);

  useEffect(() => {
    if (!open) return;
    void loadGraph();
  }, [loadGraph, open]);

  useEffect(() => {
    if (!open) return undefined;
    const handleThemeApplied = () => setThemeVersion((version) => version + 1);
    window.addEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
    return () =>
      window.removeEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
  }, [open]);

  const layout = useMemo(() => buildLayout(graph), [graph]);
  const nodeById = useMemo(() => {
    const map = new Map<string, PositionedNode>();
    layout.nodes.forEach((node) => map.set(node.id, node));
    return map;
  }, [layout.nodes]);
  const selectedNode = selectedId ? (nodeById.get(selectedId) ?? null) : null;
  const summary = useMemo(() => summarizeGraph(graph), [graph]);
  const scaledCanvasWidth = Math.round(layout.width * zoom);
  const scaledCanvasHeight = Math.round(layout.height * zoom);

  useEffect(() => {
    if (!open) return;
    if (!layout.nodes.length) {
      if (selectedId != null) setSelectedId(null);
      return;
    }
    if (!selectedId || !nodeById.has(selectedId)) {
      setSelectedId(layout.nodes[0].id);
    }
  }, [layout.nodes, nodeById, open, selectedId]);

  useEffect(() => {
    if (!open || !selectedNode) {
      detailRequestSeqRef.current += 1;
      setNodeDetail(null);
      setSelectedRecordId(null);
      setDetailError(null);
      setDetailLoading(false);
      return undefined;
    }

    const requestSeq = detailRequestSeqRef.current + 1;
    detailRequestSeqRef.current = requestSeq;
    setDetailLoading(true);
    setDetailError(null);
    setNodeDetail(null);
    setSelectedRecordId(null);
    setPromptViewer(null);
    void getProjectAssetDependencyGraphNodeDetail(projectId, selectedNode.id)
      .then((detail) => {
        if (detailRequestSeqRef.current !== requestSeq) return;
        setNodeDetail(detail);
        setSelectedRecordId((current) => {
          if (current && detail.records.some((record) => record.id === current))
            return current;
          return detail.records[0]?.id ?? null;
        });
      })
      .catch((err) => {
        if (detailRequestSeqRef.current !== requestSeq) return;
        setNodeDetail(null);
        setSelectedRecordId(null);
        setDetailError(extractErrorMessage(err, "加载资产来源失败"));
      })
      .finally(() => {
        if (detailRequestSeqRef.current === requestSeq) setDetailLoading(false);
      });

    return () => {
      if (detailRequestSeqRef.current === requestSeq) {
        detailRequestSeqRef.current += 1;
      }
    };
  }, [open, projectId, selectedNode]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    drawGraph(canvas, layout, graph?.edges ?? [], selectedId);
  }, [graph?.edges, layout, selectedId, themeVersion]);

  useEffect(() => {
    const container = networkScrollRef.current;
    if (!open || !container || !selectedNode) return;
    const center = nodeCenter(selectedNode);
    const left = clamp(
      center.x * zoom - container.clientWidth / 2,
      0,
      Math.max(0, container.scrollWidth - container.clientWidth),
    );
    const top = clamp(
      center.y * zoom - container.clientHeight / 2,
      0,
      Math.max(0, container.scrollHeight - container.clientHeight),
    );
    container.scrollTo({ left, top, behavior: "smooth" });
  }, [open, selectedNode, zoom]);

  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * layout.width;
    const y = ((event.clientY - rect.top) / rect.height) * layout.height;
    const hit = hitTestNode(layout.nodes, x, y);
    if (hit) {
      setSelectedId(hit.id);
      setMobilePane("details");
    }
  };

  const handleCanvasMove = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * layout.width;
    const y = ((event.clientY - rect.top) / rect.height) * layout.height;
    canvas.style.cursor = hitTestNode(layout.nodes, x, y)
      ? "pointer"
      : "default";
  };

  const updateZoom = useCallback(
    (next: number | ((current: number) => number)) => {
      setZoom((current) => {
        const value = typeof next === "function" ? next(current) : next;
        return Number(clamp(value, MIN_ZOOM, MAX_ZOOM).toFixed(2));
      });
    },
    [],
  );

  const handleCanvasWheel = (event: React.WheelEvent<HTMLCanvasElement>) => {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    const direction = event.deltaY > 0 ? -1 : 1;
    updateZoom((current) => current + direction * ZOOM_STEP);
  };

  const summaryText = [
    summary.text > 0 ? `文字 ${summary.text}` : null,
    summary.image > 0 ? `图片 ${summary.image}` : null,
    summary.video > 0 ? `视频 ${summary.video}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <>
      <Modal
        isOpen={open}
        onClose={onClose}
        title="内容关联"
        subtitle="查看文字、图片和视频的来源与下游关系。"
        icon={<Network size={15} />}
        maxWidthClassName="max-w-[min(1280px,96vw)]"
        bodyClassName="!overflow-hidden p-0"
        headerExtra={
          <button
            type="button"
            onClick={loadGraph}
            disabled={loading}
            className="pyqt-btn-icon !h-9 !w-9 sm:!h-8 sm:!w-8"
            title="刷新依赖图"
            aria-label="刷新依赖图"
          >
            {loading ? (
              <LoadingSpinner size="sm" />
            ) : (
              <RefreshCw size={14} />
            )}
          </button>
        }
      >
        <div className="flex h-[min(82dvh,760px)] min-h-[360px] flex-col overflow-hidden md:h-[min(74vh,760px)] md:min-h-[520px]">
          <div className="shrink-0 border-b border-book-border/45 bg-book-bg-paper px-3 py-2 md:hidden">
            <SegmentedControl
              className="mx-auto"
              ariaLabel="依赖状态视图"
              value={mobilePane}
              onChange={setMobilePane}
              options={[
                { id: "graph", label: "关系图" },
                { id: "details", label: "资产详情" },
              ]}
            />
          </div>

          <div className="grid min-h-0 flex-1 md:grid-cols-[minmax(0,1fr)_360px]">
            <div
              ref={networkScrollRef}
              className={`${mobilePane === "graph" ? "block" : "hidden"} relative min-w-0 overflow-auto bg-book-bg md:block md:border-r md:border-book-border/45 custom-scrollbar`}
            >
              {loading && !graph ? (
                <div className="flex h-full items-center justify-center text-sm text-book-text-muted">
                  <LoadingSpinner size="md" className="mr-2" />
                  正在加载依赖状态…
                </div>
              ) : error ? (
                <div className="flex h-full items-center justify-center px-6 text-sm text-error">
                  {error}
                </div>
              ) : (
                <>
                  <ZoomControls
                    zoom={zoom}
                    onZoomIn={() =>
                      updateZoom((current) => current + ZOOM_STEP)
                    }
                    onZoomOut={() =>
                      updateZoom((current) => current - ZOOM_STEP)
                    }
                    onReset={() => updateZoom(1)}
                  />
                  <div className="grid min-h-full min-w-max place-items-center p-4 pt-16">
                    <canvas
                      ref={canvasRef}
                      width={layout.width}
                      height={layout.height}
                      onClick={handleCanvasClick}
                      onMouseMove={handleCanvasMove}
                      onWheel={handleCanvasWheel}
                      className="block"
                      style={{
                        width: scaledCanvasWidth,
                        height: scaledCanvasHeight,
                      }}
                    />
                  </div>
                </>
              )}
            </div>

            <aside
              className={`${mobilePane === "details" ? "block" : "hidden"} min-w-0 overflow-y-auto bg-book-bg-paper px-3 py-3 md:block md:px-4 md:py-4 custom-scrollbar`}
            >
              <p className="text-small leading-relaxed text-book-text-muted">
                {summaryText || "尚无可展示资产"}
              </p>

              <AssetNodeIndex
                nodes={layout.nodes}
                selectedId={selectedId}
                onSelect={setSelectedId}
              />

              <SelectedAssetDetails
                node={selectedNode}
                detail={nodeDetail}
                loading={detailLoading}
                error={detailError}
                selectedRecordId={selectedRecordId}
                onSelectRecord={setSelectedRecordId}
                onOpenInputs={(record) => {
                  setPromptViewer({
                    recordLabel: record.label,
                    inputs: record.generation_inputs,
                  });
                }}
              />
            </aside>
          </div>
        </div>
      </Modal>

      <PromptViewerModal
        viewer={promptViewer}
        onClose={() => setPromptViewer(null)}
      />
    </>
  );
};

const ZoomControls: React.FC<{
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onReset: () => void;
}> = ({ zoom, onZoomIn, onZoomOut, onReset }) => (
  <div className="absolute left-3 top-3 z-10 flex w-max items-center gap-1 rounded-2xl border border-book-border/45 bg-book-bg-paper/90 p-1 shadow-[0_8px_24px_-16px_rgb(0_0_0/0.45)] backdrop-blur-xl">
    <button
      type="button"
      onClick={onZoomOut}
      className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
      title="缩小"
      aria-label="缩小依赖图"
    >
      <ZoomOut size={14} />
    </button>
    {Math.abs(zoom - 1) > 0.01 ? (
      <button
        type="button"
        onClick={onReset}
        className="inline-flex h-9 min-w-14 items-center justify-center gap-1 rounded-xl px-2 text-caption font-medium tabular-nums text-book-text-muted transition hover:bg-book-bg-tertiary hover:text-book-text-main sm:h-8"
        title="恢复原始大小"
        aria-label={`恢复原始大小，当前 ${Math.round(zoom * 100)}%`}
      >
        <Maximize2 size={12} />
        {Math.round(zoom * 100)}%
      </button>
    ) : null}
    <button
      type="button"
      onClick={onZoomIn}
      className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
      title="放大"
      aria-label="放大依赖图"
    >
      <ZoomIn size={14} />
    </button>
  </div>
);

const AssetNodeIndex: React.FC<{
  nodes: PositionedNode[];
  selectedId: string | null;
  onSelect: (nodeId: string) => void;
}> = ({ nodes, selectedId, onSelect }) => {
  const visibleNodes = nodes.filter(
    (node) => node.id !== "project" && node.count > 0,
  );
  const videoNodes = visibleNodes.filter(isVideoChainNode);
  const selectedNode = visibleNodes.find((node) => node.id === selectedId) ?? null;
  const primaryNodes = videoNodes.length
    ? [selectedNode, ...videoNodes].filter(
        (node, index, nodes): node is PositionedNode =>
          Boolean(node) && nodes.findIndex((item) => item?.id === node?.id) === index,
      )
    : visibleNodes.slice(0, 8);
  if (!primaryNodes.length) return null;

  return (
    <section className="mt-4">
      <div className="flex items-center justify-between gap-3 px-1">
        <h3 className="text-xs font-semibold text-book-text-main">
          {videoNodes.length ? "视频链路" : "资产索引"}
        </h3>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {primaryNodes.length} 个节点
        </span>
      </div>
      <div className="a4n-flat-surface mt-2 divide-y divide-book-border/35 overflow-hidden">
        {primaryNodes.map((node) => {
          const selected = node.id === selectedId;
          return (
            <button
              key={node.id}
              type="button"
              onClick={() => onSelect(node.id)}
              className={`relative grid min-h-10 w-full min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2 text-left transition ${
                selected
                  ? "bg-book-primary/8 text-book-text-main shadow-[inset_3px_0_0_rgb(var(--color-primary))]"
                  : "text-book-text-muted hover:bg-book-bg-tertiary/65 hover:text-book-text-main"
              }`}
              title={node.label}
            >
              <span className="min-w-0 truncate text-small font-medium">
                {node.label}
              </span>
              <span className="shrink-0 text-caption tabular-nums text-book-text-subtle">
                {node.count} 项
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
};

const DetailTerm: React.FC<{
  label: string;
  value: React.ReactNode;
  title?: string;
}> = ({ label, value, title }) => (
  <>
    <dt className="text-book-text-subtle">{label}</dt>
    <dd
      className="min-w-0 truncate text-right text-book-text-main"
      title={title}
    >
      {value}
    </dd>
  </>
);

const SelectedAssetDetails: React.FC<{
  node: AssetGraphNode | null;
  detail: AssetGraphNodeDetail | null;
  loading: boolean;
  error: string | null;
  selectedRecordId: string | null;
  onSelectRecord: (recordId: string) => void;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({
  node,
  detail,
  loading,
  error,
  selectedRecordId,
  onSelectRecord,
  onOpenInputs,
}) => {
  if (!node) {
    return (
      <div className="a4n-flat-surface mt-4 px-3 py-4 text-small text-book-text-subtle">
        在画布中选择一个节点查看信息。
      </div>
    );
  }

  const records = detail?.records ?? [];
  const selectedRecord =
    records.find((record) => record.id === selectedRecordId) ??
    records[0] ??
    null;
  const inputCount = records.reduce(
    (sum, record) => sum + record.generation_inputs.length,
    0,
  );
  const warningCount =
    (detail?.warnings.length ?? 0) +
    records.reduce((sum, record) => sum + record.warnings.length, 0);
  const nodeStatus = node.status ? getOperationStatusLabel(node.status) : null;

  return (
    <section className="a4n-flat-surface mt-4 overflow-hidden">
      <div className="px-3 py-3">
        <h3
          className="truncate text-sm font-semibold text-book-text-main"
          title={node.label}
        >
          {node.label}
        </h3>
        <p className="mt-1 text-small leading-relaxed text-book-text-muted">
          {[
            kindLabel(node.kind),
            assetTypeLabel(node.asset_type),
            nodeStatus,
            `${node.count} 项`,
          ]
            .filter(Boolean)
            .join(" · ")}
        </p>
        {node.summary ? (
          <p className="mt-2 text-small leading-relaxed text-book-text-muted">
            {node.summary}
          </p>
        ) : null}
      </div>

      {loading ? (
        <div className="flex items-center border-t border-book-border/35 px-3 py-3 text-small text-book-text-muted">
          <LoadingSpinner size="sm" className="mr-1.5" />
          正在加载来源记录…
        </div>
      ) : error ? (
        <div className="border-t border-error/25 bg-error/5 px-3 py-3 text-small leading-relaxed text-error">
          {error}
        </div>
      ) : detail ? (
        <>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-book-border/35 px-3 py-2.5 text-small text-book-text-muted">
            <span>来源 {records.length}</span>
            {inputCount > 0 ? <span>生成输入 {inputCount}</span> : null}
            {warningCount > 0 ? (
              <span className="text-warning">异常 {warningCount}</span>
            ) : null}
          </div>

          <WarningsBlock
            warnings={[...detail.warnings, ...(selectedRecord?.warnings ?? [])]}
          />

          <RecordPicker
            records={records}
            selectedRecordId={selectedRecord?.id ?? null}
            onSelectRecord={onSelectRecord}
          />

          <LineageRecordDetails
            node={node}
            record={selectedRecord}
            onOpenInputs={onOpenInputs}
          />
        </>
      ) : (
        <div className="border-t border-book-border/35 px-3 py-3 text-small leading-relaxed text-book-text-subtle">
          当前节点没有返回来源详情。
        </div>
      )}
    </section>
  );
};

const WarningsBlock: React.FC<{ warnings: string[] }> = ({ warnings }) => {
  const unique = [...new Set(warnings.filter(Boolean))].slice(0, 4);
  if (!unique.length) return null;
  return (
    <div className="space-y-1 border-t border-warning/20 bg-warning/5 px-3 py-2.5">
      {unique.map((warning) => (
        <div
          key={warning}
          className="text-small leading-relaxed text-book-text-muted"
        >
          {warning}
        </div>
      ))}
    </div>
  );
};

const RecordPicker: React.FC<{
  records: AssetLineageRecord[];
  selectedRecordId: string | null;
  onSelectRecord: (recordId: string) => void;
}> = ({ records, selectedRecordId, onSelectRecord }) => {
  if (!records.length) {
    return (
      <div className="border-t border-book-border/35 px-3 py-3 text-small text-book-text-subtle">
        没有可展示的具体记录。
      </div>
    );
  }

  return (
    <section className="border-t border-book-border/35 px-3 py-3">
      <div className="text-small font-semibold text-book-text-main">
        来源记录
      </div>
      <div className="mt-2 divide-y divide-book-border/35 border-y border-book-border/35">
        {records.map((record) => {
          const selected = record.id === selectedRecordId;
          return (
            <button
              key={record.id}
              type="button"
              onClick={() => onSelectRecord(record.id)}
              className={`block w-full px-3 py-2 text-left transition ${
                selected
                  ? "bg-book-primary/8 text-book-text-main shadow-[inset_3px_0_0_rgb(var(--color-primary))]"
                  : "text-book-text-muted hover:bg-book-bg-tertiary/65"
              }`}
              title={record.label}
            >
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2">
                <span className="min-w-0 truncate text-small font-semibold">
                  {record.label}
                </span>
                {record.generation_inputs.length > 0 ? (
                  <span className="shrink-0 text-caption text-book-text-subtle">
                    输入 {record.generation_inputs.length}
                  </span>
                ) : null}
              </div>
              <div className="mt-0.5 truncate text-caption text-book-text-subtle">
                {[
                  originLabel(record.origin),
                  record.status ? getOperationStatusLabel(record.status) : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </div>
            </button>
          );
        })}
      </div>
    </section>
  );
};

const LineageRecordDetails: React.FC<{
  node: AssetGraphNode;
  record: AssetLineageRecord | null;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({ node, record, onOpenInputs }) => {
  if (!record) return null;
  return (
    <section className="border-t border-book-border/35 px-3 py-3">
      <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-small">
        <DetailTerm label="来源" value={originLabel(record.origin)} />
        <DetailTerm label="资产" value={assetTypeLabel(record.asset_kind)} />
        <DetailTerm
          label="状态"
          value={
            record.status ? getOperationStatusLabel(record.status) : "未标记"
          }
        />
      </dl>

      <RefList
        title="上游"
        refs={record.source_refs}
        emptyText="没有记录上游来源。"
      />
      <RefList
        title="媒体引用"
        refs={record.media_sources}
        emptyText="没有媒体引用。"
      />
      <details className="mt-3 border-t border-book-border/35 pt-1 text-caption text-book-text-muted">
        <summary className="cursor-pointer py-2 hover:text-book-text-main">
          诊断信息
        </summary>
        <div className="space-y-2 border-l border-book-border/35 pl-3">
          <dl className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 py-1.5">
            <DetailTerm label="节点标识" value={node.id} title={node.id} />
            <DetailTerm label="节点类型" value={node.kind} title={node.kind} />
            <DetailTerm
              label="节点资产"
              value={node.asset_type}
              title={node.asset_type}
            />
            {node.chapter_id != null ? (
              <DetailTerm label="章节编号" value={node.chapter_id} />
            ) : null}
            {node.profile_id != null ? (
              <DetailTerm label="档案编号" value={node.profile_id} />
            ) : null}
            {node.record_id != null ? (
              <DetailTerm label="节点记录" value={node.record_id} />
            ) : null}
            <DetailTerm
              label="变更策略"
              value={node.mutation_policy}
              title={node.mutation_policy}
            />
            <DetailTerm label="记录标识" value={record.id} title={record.id} />
            <DetailTerm
              label="资产类型"
              value={record.asset_kind}
              title={record.asset_kind}
            />
            {record.asset_id != null ? (
              <DetailTerm label="资产编号" value={record.asset_id} />
            ) : null}
            <DetailTerm
              label="来源类型"
              value={record.origin}
              title={record.origin}
            />
            {record.status ? (
              <DetailTerm
                label="原始状态"
                value={record.status}
                title={record.status}
              />
            ) : null}
          </dl>
          <GenerationInputAccess record={record} onOpenInputs={onOpenInputs} />
          <JsonBlock title="结果原始记录" value={record.result} />
          {record.request_snapshot ? (
            <JsonBlock title="请求快照" value={record.request_snapshot} />
          ) : null}
        </div>
      </details>
    </section>
  );
};

const RefList: React.FC<{
  title: string;
  refs: AssetLineageRef[];
  emptyText: string;
}> = ({ title, refs, emptyText }) => (
  <section className="mt-3 border-t border-book-border/35 pt-3">
    <div className="text-small font-semibold text-book-text-main">{title}</div>
    {refs.length ? (
      <div className="mt-1.5 divide-y divide-book-border/30">
        {refs.slice(0, 6).map((ref) => (
          <div
            key={ref.id}
            className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-2 py-1.5 text-small"
          >
            <span
              className="min-w-0 truncate text-book-text-main"
              title={ref.label}
            >
              {ref.label}
            </span>
            <span className="shrink-0 text-caption text-book-text-subtle">
              {assetTypeLabel(ref.asset_type)}
            </span>
          </div>
        ))}
      </div>
    ) : (
      <div className="mt-1 text-small text-book-text-subtle">{emptyText}</div>
    )}
  </section>
);

const GenerationInputAccess: React.FC<{
  record: AssetLineageRecord;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({ record, onOpenInputs }) => {
  const inputs = record.generation_inputs;
  return (
    <div className="border-t border-book-border/30 py-2">
      {inputs.length ? (
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 text-caption text-book-text-muted">
            生成输入 · {inputs.length} 项 ·{" "}
            {inputStatusLabel(record.input_status)}
          </div>
          <button
            type="button"
            onClick={() => onOpenInputs(record)}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-lg px-2 py-1 text-caption font-medium text-book-primary transition hover:bg-book-primary/10"
            title="查看生成该资产的完整输入包"
          >
            <FileText size={11} />
            查看
          </button>
        </div>
      ) : (
        <div className="text-caption leading-relaxed text-book-text-subtle">
          {generationInputEmptyText(record.input_status)}
        </div>
      )}
    </div>
  );
};

const PromptViewerModal: React.FC<{
  viewer: InputViewerState | null;
  onClose: () => void;
}> = ({ viewer, onClose }) => {
  if (!viewer) return null;
  return (
    <Modal
      isOpen
      onClose={onClose}
      title="生成输入"
      subtitle={viewer.recordLabel}
      icon={<FileText size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="min-h-0 flex-1 overflow-y-auto p-3 sm:p-4 custom-scrollbar"
      zIndexClassName="z-[70]"
    >
      <div className="space-y-3">
        {groupGenerationInputs(viewer.inputs).map(([group, inputs]) => (
          <section key={group} className="a4n-flat-surface overflow-hidden">
            <div className="border-b border-book-border/35 px-3 py-2.5">
              <div className="text-xs font-semibold text-book-text-main">
                {inputGroupLabel(group)}
              </div>
              <div className="mt-0.5 text-caption tabular-nums text-book-text-subtle">
                {inputs.length} 项
              </div>
            </div>
            <div className="divide-y divide-book-border/35">
              {inputs.map((input, index) => (
                <GenerationInputCard key={`${input.kind}:${index}`} input={input} />
              ))}
            </div>
          </section>
        ))}
      </div>
    </Modal>
  );
};

const GenerationInputCard: React.FC<{ input: GenerationInputSource }> = ({
  input,
}) => {
  return (
    <article className="px-3 py-3">
      <h4
        className="truncate text-small font-semibold text-book-text-main"
        title={input.label}
      >
        {input.label}
      </h4>
      <div className="mt-2">
        <GenerationInputBody input={input} />
      </div>
    </article>
  );
};

const GenerationInputBody: React.FC<{ input: GenerationInputSource }> = ({
  input,
}) => {
  if (input.kind === "rendered_messages") {
    return <MessageList value={input.value} />;
  }
  if (typeof input.value === "string") {
    return <TextBlock text={input.value} />;
  }
  return <StructuredValueView value={input.value} />;
};

const MessageList: React.FC<{ value: unknown }> = ({ value }) => {
  if (!Array.isArray(value) || !value.every(isChatMessageLike)) {
    return <StructuredValueView value={value} />;
  }
  if (!value.length) {
    return (
      <span className="text-body-sm text-book-text-subtle">空消息列表</span>
    );
  }
  return (
    <div className="divide-y divide-book-border/30 border-y border-book-border/30">
      {value.map((message, index) => (
        <MessageBlock
          key={`${String(message.role ?? "message")}-${index}`}
          message={message}
          index={index}
        />
      ))}
    </div>
  );
};

const MessageBlock: React.FC<{
  message: Record<string, unknown>;
  index: number;
}> = ({ message, index }) => {
  const role =
    typeof message.role === "string" && message.role.trim()
      ? message.role
      : "message";
  const content = message.content;
  const extraEntries = Object.entries(message).filter(
    ([key]) => key !== "role" && key !== "content",
  );
  return (
    <article className="py-2.5">
      <div className="flex items-center justify-between gap-3">
        <span className="text-small font-semibold text-book-text-main">
          {messageRoleLabel(role)}
        </span>
        <span className="text-caption tabular-nums text-book-text-subtle">
          消息 {index + 1}
        </span>
      </div>
      <div className="mt-2">
        {typeof content === "string" ? (
          <TextBlock text={content} />
        ) : (
          <StructuredValueView value={content} />
        )}
        {extraEntries.length ? (
          <div className="mt-2 border-t border-book-border/70 pt-1.5">
            <StructuredEntriesView entries={extraEntries} depth={1} />
          </div>
        ) : null}
      </div>
    </article>
  );
};

const StructuredValueView: React.FC<{ value: unknown; depth?: number }> = ({
  value,
  depth = 0,
}) => {
  if (value == null) {
    return <span className="text-body-sm text-book-text-subtle">null</span>;
  }
  if (typeof value === "string") {
    return <TextBlock text={value} />;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return (
      <span className="font-mono text-body-sm text-book-text-main">
        {String(value)}
      </span>
    );
  }
  if (Array.isArray(value)) {
    if (!value.length) {
      return <span className="text-body-sm text-book-text-subtle">空数组</span>;
    }
    if (value.every(isChatMessageLike)) {
      return <MessageList value={value} />;
    }
    return (
      <div className="divide-y divide-book-border/25 border-y border-book-border/30">
        {value.map((item, index) => (
          <StructuredValueItem
            key={index}
            title={`#${index + 1}`}
            value={item}
            depth={depth}
          />
        ))}
      </div>
    );
  }
  if (isRecord(value)) {
    return (
      <StructuredEntriesView entries={Object.entries(value)} depth={depth} />
    );
  }
  return <TextBlock text={String(value)} />;
};

const StructuredEntriesView: React.FC<{
  entries: Array<[string, unknown]>;
  depth: number;
}> = ({ entries, depth }) => {
  if (!entries.length) {
    return <span className="text-body-sm text-book-text-subtle">空对象</span>;
  }
  return (
    <div className="divide-y divide-book-border/25 border-y border-book-border/30">
      {entries.map(([key, value]) => (
        <StructuredValueItem
          key={key}
          title={key.replace(/_/g, " ")}
          value={value}
          depth={depth}
        />
      ))}
    </div>
  );
};

const StructuredValueItem: React.FC<{
  title: string;
  value: unknown;
  depth: number;
}> = ({ title, value, depth }) => {
  if (typeof value === "string") {
    const text = value;
    const compact =
      !text.includes("\n") && text.length <= INLINE_TEXT_MAX_CHARS;
    return compact ? (
      <div className="grid grid-cols-[minmax(6rem,9rem)_minmax(0,1fr)] gap-2 py-2 text-small">
        <div
          className="truncate font-semibold text-book-text-muted"
          title={title}
        >
          {title}
        </div>
        <div className="min-w-0 break-words text-book-text-main">{text}</div>
      </div>
    ) : (
      <details
        open={text.length <= AUTO_OPEN_TEXT_MAX_CHARS && depth === 0}
      >
        <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
          <span>{title}</span>
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {textSummary(text)}
          </span>
        </summary>
        <div className="border-l border-book-border/35 pb-2 pl-3">
          <TextBlock text={text} />
        </div>
      </details>
    );
  }
  if (
    value == null ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return (
      <div className="grid grid-cols-[minmax(6rem,9rem)_minmax(0,1fr)] gap-2 py-2 text-small">
        <div
          className="truncate font-semibold text-book-text-muted"
          title={title}
        >
          {title}
        </div>
        <div className="min-w-0 break-words font-mono text-book-text-main">
          {String(value)}
        </div>
      </div>
    );
  }
  if (
    title === "messages" &&
    Array.isArray(value) &&
    value.every(isChatMessageLike)
  ) {
    return (
      <details
        open={value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS && depth === 0}
      >
        <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
          <span>{title}</span>
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {value.length} 条消息
          </span>
        </summary>
        <div className="border-l border-book-border/35 pb-2 pl-3">
          <MessageList value={value} />
        </div>
      </details>
    );
  }
  const summary = structuredValueSummary(value);
  return (
    <details
      open={shouldAutoOpenStructuredValue(value, depth)}
    >
      <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
        <span>{title}</span>
        {summary ? (
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {summary}
          </span>
        ) : null}
      </summary>
      <div className="border-l border-book-border/35 pb-2 pl-3">
        <StructuredValueView value={value} depth={depth + 1} />
      </div>
    </details>
  );
};

function shouldAutoOpenStructuredValue(value: unknown, depth: number): boolean {
  if (depth > 0) return false;
  if (Array.isArray(value)) return value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS;
  if (isRecord(value))
    return Object.keys(value).length <= AUTO_OPEN_OBJECT_MAX_FIELDS;
  return true;
}

function structuredValueSummary(value: unknown): string | null {
  if (Array.isArray(value)) return `${value.length} 项`;
  if (isRecord(value)) return `${Object.keys(value).length} 字段`;
  return null;
}

function textSummary(text: string): string {
  const lines = text.split("\n").length;
  return lines > 1
    ? `${text.length} 字符 / ${lines} 行`
    : `${text.length} 字符`;
}

const TextBlock: React.FC<{ text: string }> = ({ text }) => (
  <pre className="m-0 whitespace-pre-wrap break-words text-body-sm leading-relaxed text-book-text-main">
    {text}
  </pre>
);

const JsonBlock: React.FC<{ title: string; value: unknown }> = ({
  title,
  value,
}) => (
  <details className="border-t border-book-border/30 py-1">
    <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-main">
      {title}
    </summary>
    <div className="border-l border-book-border/35 pb-2 pl-3">
      <StructuredValueView value={value} />
    </div>
  </details>
);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function originLabel(origin: string): string {
  const labels: Record<string, string> = {
    derived_text: "文本派生",
    llm_generated: "模型生成",
    image_generated: "图片生成",
    video_generated: "视频生成",
    uploaded: "用户上传",
    imported: "导入资产",
    manual: "手动记录",
    extracted_frame: "抽帧生成",
    unknown: "来源不明",
  };
  return labels[origin] ?? "其他来源";
}

function inputStatusLabel(status: string): string {
  if (status === "complete") return "完整";
  if (status === "missing") return "缺失";
  if (status === "run_without_inputs") return "无快照";
  if (status === "structural") return "结构来源";
  if (status === "not_applicable") return "不适用";
  return "状态未知";
}

function generationInputEmptyText(status: string): string {
  if (status === "structural")
    return "该记录已有结构化上游，但没有单独的生成输入快照。";
  if (status === "not_applicable") return "该记录不需要生成输入快照。";
  if (status === "run_without_inputs") return "生成运行存在，但缺少输入快照。";
  return "缺少生成输入记录。";
}

function groupGenerationInputs(
  inputs: GenerationInputSource[],
): Array<[string, GenerationInputSource[]]> {
  const order = [
    "prompt",
    "variables",
    "source_text",
    "media",
    "request",
    "response",
  ];
  const grouped = new Map<string, GenerationInputSource[]>();
  inputs.forEach((input) => {
    const list = grouped.get(input.group) ?? [];
    list.push(input);
    grouped.set(input.group, list);
  });
  return [...grouped.entries()].sort(([a], [b]) => {
    const ai = order.indexOf(a);
    const bi = order.indexOf(b);
    return (ai < 0 ? order.length : ai) - (bi < 0 ? order.length : bi);
  });
}

function inputGroupLabel(group: string): string {
  const labels: Record<string, string> = {
    prompt: "提示词",
    variables: "输入变量",
    source_text: "上游文本",
    media: "参考媒体",
    request: "模型请求",
    response: "响应摘要",
  };
  return labels[group] ?? "其他输入";
}

function isChatMessageLike(value: unknown): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  return "content" in value || "role" in value;
}

function messageRoleLabel(role: string): string {
  const labels: Record<string, string> = {
    system: "系统",
    user: "用户",
    assistant: "助手",
    tool: "工具",
    message: "消息",
  };
  return labels[role] ?? "消息";
}
