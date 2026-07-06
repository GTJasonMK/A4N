import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { FileText, Loader2, Maximize2, Network, RefreshCw, ZoomIn, ZoomOut } from "lucide-react";
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
import { BookButton } from "../../../components/ui/BookButton";
import { THEME_APPLIED_EVENT } from "../../../theme/activeTheme";
import {
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
export const AssetDependencyGraphDialog: React.FC<AssetDependencyGraphDialogProps> = ({
  open,
  projectId,
  onClose,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const networkScrollRef = useRef<HTMLDivElement | null>(null);
  const graphRequestSeqRef = useRef(0);
  const detailRequestSeqRef = useRef(0);
  const graphLoadingRef = useRef(false);
  const [graph, setGraph] = useState<AssetDependencyGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [nodeDetail, setNodeDetail] = useState<AssetGraphNodeDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [selectedRecordId, setSelectedRecordId] = useState<string | null>(null);
  const [promptViewer, setPromptViewer] = useState<InputViewerState | null>(null);
  const [themeVersion, setThemeVersion] = useState(0);
  const [zoom, setZoom] = useState(1);

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
        if (current && next.nodes.some((node) => node.id === current)) return current;
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
  }, [open, projectId]);

  useEffect(() => {
    if (!open) return;
    void loadGraph();
  }, [loadGraph, open]);

  useEffect(() => {
    if (!open) setPromptViewer(null);
  }, [open]);

  useEffect(() => {
    if (!open) return undefined;
    const handleThemeApplied = () => setThemeVersion((version) => version + 1);
    window.addEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
    return () => window.removeEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
  }, [open]);

  const layout = useMemo(() => buildLayout(graph), [graph]);
  const nodeById = useMemo(() => {
    const map = new Map<string, PositionedNode>();
    layout.nodes.forEach((node) => map.set(node.id, node));
    return map;
  }, [layout.nodes]);
  const selectedNode = selectedId ? nodeById.get(selectedId) ?? null : null;
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
          if (current && detail.records.some((record) => record.id === current)) return current;
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
    if (hit) setSelectedId(hit.id);
  };

  const handleCanvasMove = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * layout.width;
    const y = ((event.clientY - rect.top) / rect.height) * layout.height;
    canvas.style.cursor = hitTestNode(layout.nodes, x, y) ? "pointer" : "default";
  };

  const updateZoom = useCallback((next: number | ((current: number) => number)) => {
    setZoom((current) => {
      const value = typeof next === "function" ? next(current) : next;
      return Number(clamp(value, MIN_ZOOM, MAX_ZOOM).toFixed(2));
    });
  }, []);

  const handleCanvasWheel = (event: React.WheelEvent<HTMLCanvasElement>) => {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    const direction = event.deltaY > 0 ? -1 : 1;
    updateZoom((current) => current + direction * ZOOM_STEP);
  };

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title="项目依赖状态"
      subtitle="从灵感、蓝图、正文、档案到漫画、图片、视频的当前拓扑和下游处理状态。"
      icon={<Network size={15} />}
      maxWidthClassName="max-w-[min(1280px,96vw)]"
      bodyClassName="p-0"
      headerExtra={
        <button
          type="button"
          onClick={loadGraph}
          disabled={loading}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title="刷新依赖图"
        >
          {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
        </button>
      }
    >
      <div className="relative grid h-[min(82dvh,760px)] min-h-[360px] grid-rows-[minmax(0,1fr)_minmax(12rem,18rem)] overflow-hidden md:h-[min(74vh,760px)] md:min-h-[520px] md:grid-cols-[minmax(0,1fr)_340px] md:grid-rows-1">
        <div
          ref={networkScrollRef}
          className="relative min-w-0 overflow-auto border-b border-book-border bg-book-bg md:border-b-0 md:border-r custom-scrollbar"
        >
          {loading && !graph ? (
            <div className="flex h-full items-center justify-center text-sm text-book-text-muted">
              <Loader2 size={18} className="mr-2 animate-spin" />
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
                onZoomIn={() => updateZoom((current) => current + ZOOM_STEP)}
                onZoomOut={() => updateZoom((current) => current - ZOOM_STEP)}
                onReset={() => updateZoom(1)}
              />
              <div className="grid min-h-full min-w-max place-items-center p-4 pt-12">
                <canvas
                  ref={canvasRef}
                  width={layout.width}
                  height={layout.height}
                  onClick={handleCanvasClick}
                  onMouseMove={handleCanvasMove}
                  onWheel={handleCanvasWheel}
                  className="block"
                  style={{ width: scaledCanvasWidth, height: scaledCanvasHeight }}
                />
              </div>
            </>
          )}
        </div>

        <aside className="min-w-0 overflow-y-auto bg-book-bg-paper px-3 py-3 custom-scrollbar">
          <div className="grid grid-cols-3 gap-2 text-center">
            <Metric label="文字" value={summary.text} />
            <Metric label="图片" value={summary.image} />
            <Metric label="视频" value={summary.video} />
          </div>

          <AssetNodeIndex
            nodes={layout.nodes}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />

          <section className="mt-3">
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
          </section>

          <div className="mt-4 flex justify-end">
            <BookButton variant="secondary" size="sm" onClick={onClose}>
              关闭
            </BookButton>
          </div>
        </aside>

        <PromptViewerOverlay
          viewer={promptViewer}
          onClose={() => setPromptViewer(null)}
        />
      </div>
    </Modal>
  );
};

const ZoomControls: React.FC<{
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onReset: () => void;
}> = ({ zoom, onZoomIn, onZoomOut, onReset }) => (
  <div className="sticky left-3 top-3 z-10 flex w-max items-center gap-1 rounded-sm border border-book-border bg-book-bg-paper/90 px-1.5 py-1 shadow-sm backdrop-blur">
    <button
      type="button"
      onClick={onZoomOut}
      className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
      title="缩小"
      aria-label="缩小依赖图"
    >
      <ZoomOut size={13} />
    </button>
    <button
      type="button"
      onClick={onReset}
      className="inline-flex h-9 min-w-16 items-center justify-center gap-1 rounded-sm border border-book-border bg-book-bg px-2 font-mono text-[10px] font-semibold text-book-text-muted hover:border-book-primary/50 hover:text-book-primary sm:h-7 sm:min-w-14"
      title="重置缩放"
      aria-label="重置依赖图缩放"
    >
      <Maximize2 size={11} />
      {Math.round(zoom * 100)}%
    </button>
    <button
      type="button"
      onClick={onZoomIn}
      className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
      title="放大"
      aria-label="放大依赖图"
    >
      <ZoomIn size={13} />
    </button>
  </div>
);

const Metric: React.FC<{ label: string; value: number }> = ({ label, value }) => (
  <div className="rounded-sm border border-book-border bg-book-bg px-2 py-2">
    <div className="text-base font-semibold text-book-text-main">{value}</div>
    <div className="mt-0.5 text-[10px] text-book-text-muted">{label}</div>
  </div>
);

const AssetNodeIndex: React.FC<{
  nodes: PositionedNode[];
  selectedId: string | null;
  onSelect: (nodeId: string) => void;
}> = ({ nodes, selectedId, onSelect }) => {
  const visibleNodes = nodes.filter((node) => node.id !== "project" && node.count > 0);
  const videoNodes = visibleNodes.filter(isVideoChainNode);
  const primaryNodes = videoNodes.length ? videoNodes : visibleNodes.slice(0, 8);
  if (!primaryNodes.length) return null;

  return (
    <section className="mt-3 rounded-sm border border-book-border bg-book-bg px-2.5 py-2">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold text-book-text-main">
          {videoNodes.length ? "视频链路" : "资产索引"}
        </div>
        <div className="font-mono text-[10px] text-book-text-subtle">{primaryNodes.length}</div>
      </div>
      <div className="grid gap-1">
        {primaryNodes.map((node) => {
          const selected = node.id === selectedId;
          return (
            <button
              key={node.id}
              type="button"
              onClick={() => onSelect(node.id)}
              className={`flex min-w-0 items-center justify-between gap-2 rounded-sm border px-2 py-1.5 text-left transition ${
                selected
                  ? "border-book-primary bg-book-bg-paper text-book-text-main"
                  : "border-book-border/70 bg-book-bg-paper/70 text-book-text-muted hover:border-book-primary/50 hover:text-book-primary"
              }`}
              title={`${node.label} / ${node.asset_type}`}
            >
              <span className="min-w-0 truncate text-[11px] font-semibold">{node.label}</span>
              <span className="shrink-0 rounded-sm border border-book-border bg-book-bg px-1.5 py-0.5 font-mono text-[10px]">
                x{node.count}
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
};

const DetailTerm: React.FC<{ label: string; value: React.ReactNode; title?: string }> = ({
  label,
  value,
  title,
}) => (
  <>
    <dt className="text-book-text-subtle">{label}</dt>
    <dd className="min-w-0 truncate text-right text-book-text-main" title={title}>
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
}> = ({ node, detail, loading, error, selectedRecordId, onSelectRecord, onOpenInputs }) => {
  if (!node) {
    return (
      <div className="rounded-sm border border-book-border bg-book-bg px-2.5 py-3 text-[11px] text-book-text-subtle">
        在画布中选择一个节点查看信息。
      </div>
    );
  }

  const records = detail?.records ?? [];
  const selectedRecord = records.find((record) => record.id === selectedRecordId) ?? records[0] ?? null;
  const inputCount = records.reduce((sum, record) => sum + record.generation_inputs.length, 0);
  const warningCount = (detail?.warnings.length ?? 0)
    + records.reduce((sum, record) => sum + record.warnings.length, 0);

  return (
    <div className="rounded-sm border border-book-border bg-book-bg px-2.5 py-2.5">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h4 className="truncate text-xs font-semibold text-book-text-main" title={node.label}>
            选中资产详情
          </h4>
          <div className="mt-1 truncate text-[12px] font-semibold text-book-text-main" title={node.label}>
            {node.label}
          </div>
          <div className="mt-0.5 truncate font-mono text-[10px] text-book-text-subtle" title={node.id}>
            {node.id}
          </div>
        </div>
        <span className="shrink-0 rounded-sm border border-book-border bg-book-bg-paper px-1.5 py-0.5 text-[10px] text-book-text-muted">
          x{node.count}
        </span>
      </div>

      {node.summary ? (
        <p className="mt-2 rounded-sm border border-book-border/70 bg-book-bg-paper px-2 py-1.5 text-[11px] leading-relaxed text-book-text-muted">
          {node.summary}
        </p>
      ) : null}

      <dl className="mt-2 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-[11px]">
        <DetailTerm
          label="类型"
          value={`${kindLabel(node.kind)} / ${node.asset_type}`}
          title={`${node.kind} / ${node.asset_type}`}
        />
        <DetailTerm label="状态" value={node.status ?? "未标记"} title={node.status ?? "未标记"} />
        <DetailTerm label="记录数" value={detail ? records.length : node.count} />
        {node.chapter_id != null ? <DetailTerm label="章节" value={node.chapter_id} /> : null}
        {node.profile_id != null ? <DetailTerm label="档案" value={node.profile_id} /> : null}
        {node.record_id != null ? <DetailTerm label="记录" value={node.record_id} /> : null}
      </dl>

      {loading ? (
        <div className="mt-3 flex items-center rounded-sm border border-book-border/70 bg-book-bg-paper px-2 py-2 text-[11px] text-book-text-muted">
          <Loader2 size={13} className="mr-1.5 animate-spin" />
          正在加载来源记录…
        </div>
      ) : error ? (
        <div className="mt-3 rounded-sm border border-error/40 bg-book-bg-paper px-2 py-2 text-[11px] leading-relaxed text-error">
          {error}
        </div>
      ) : detail ? (
        <>
          <div className="mt-3 grid grid-cols-3 gap-1.5 text-center">
            <MiniMetric label="来源记录" value={records.length} />
            <MiniMetric label="生成输入" value={inputCount} />
            <MiniMetric label="异常" value={warningCount} />
          </div>

          <WarningsBlock warnings={[...detail.warnings, ...(selectedRecord?.warnings ?? [])]} />

          <RecordPicker
            records={records}
            selectedRecordId={selectedRecord?.id ?? null}
            onSelectRecord={onSelectRecord}
          />

          <LineageRecordDetails record={selectedRecord} onOpenInputs={onOpenInputs} />
        </>
      ) : (
        <div className="mt-3 rounded-sm border border-book-border/70 bg-book-bg-paper px-2 py-2 text-[11px] leading-relaxed text-book-text-subtle">
          当前节点没有返回来源详情。
        </div>
      )}
    </div>
  );
};

const MiniMetric: React.FC<{ label: string; value: number }> = ({ label, value }) => (
  <div className="rounded-sm border border-book-border bg-book-bg-paper px-1.5 py-1.5">
    <div className="font-mono text-[12px] font-semibold text-book-text-main">{value}</div>
    <div className="mt-0.5 text-[10px] text-book-text-subtle">{label}</div>
  </div>
);

const WarningsBlock: React.FC<{ warnings: string[] }> = ({ warnings }) => {
  const unique = [...new Set(warnings.filter(Boolean))].slice(0, 4);
  if (!unique.length) return null;
  return (
    <div className="mt-2 space-y-1 rounded-sm border border-book-border bg-book-bg-paper px-2 py-1.5">
      {unique.map((warning) => (
        <div key={warning} className="text-[11px] leading-relaxed text-book-text-muted">
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
      <div className="mt-2 rounded-sm border border-book-border/70 bg-book-bg-paper px-2 py-2 text-[11px] text-book-text-subtle">
        没有可展示的具体记录。
      </div>
    );
  }

  return (
    <div className="mt-3">
      <div className="mb-1.5 text-[11px] font-semibold text-book-text-main">记录</div>
      <div className="max-h-36 space-y-1 overflow-auto pr-1 custom-scrollbar">
        {records.map((record) => {
          const selected = record.id === selectedRecordId;
          return (
            <button
              key={record.id}
              type="button"
              onClick={() => onSelectRecord(record.id)}
              className={`block w-full rounded-sm border px-2 py-1.5 text-left transition ${
                selected
                  ? "border-book-primary bg-book-bg text-book-text-main"
                  : "border-book-border bg-book-bg-paper text-book-text-muted hover:border-book-primary/50"
              }`}
              title={record.label}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="min-w-0 truncate text-[11px] font-semibold">{record.label}</span>
                <span className="shrink-0 font-mono text-[10px]">
                  {record.generation_inputs.length}i
                </span>
              </div>
              <div className="mt-0.5 flex items-center justify-between gap-2 text-[10px]">
                <span className="min-w-0 truncate">{originLabel(record.origin)}</span>
                <span className="shrink-0 truncate">{record.status ?? record.asset_kind}</span>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
};

const LineageRecordDetails: React.FC<{
  record: AssetLineageRecord | null;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({ record, onOpenInputs }) => {
  if (!record) return null;
  return (
    <div className="mt-3 space-y-2">
      <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-[11px]">
        <DetailTerm label="来源" value={originLabel(record.origin)} title={record.origin} />
        <DetailTerm label="资产" value={record.asset_kind} title={record.asset_kind} />
        {record.asset_id != null ? <DetailTerm label="ID" value={record.asset_id} /> : null}
        <DetailTerm label="状态" value={record.status ?? "未标记"} title={record.status ?? "未标记"} />
      </dl>

      <RefList title="上游" refs={record.source_refs} emptyText="没有记录上游来源。" />
      <GenerationInputAccess record={record} onOpenInputs={onOpenInputs} />
      <RefList title="媒体引用" refs={record.media_sources} emptyText="没有媒体引用。" />

      <JsonBlock title="结果" value={record.result} defaultOpen />

      {record.request_snapshot ? (
        <JsonBlock title="请求快照" value={record.request_snapshot} />
      ) : null}
    </div>
  );
};

const RefList: React.FC<{ title: string; refs: AssetLineageRef[]; emptyText: string }> = ({
  title,
  refs,
  emptyText,
}) => (
  <div className="rounded-sm border border-book-border bg-book-bg-paper">
    <div className="border-b border-book-border/70 px-2 py-1 text-[11px] font-semibold text-book-text-main">
      {title}
    </div>
    {refs.length ? (
      <div className="space-y-1 px-2 py-1.5">
        {refs.slice(0, 6).map((ref) => (
          <div key={ref.id} className="flex min-w-0 items-center justify-between gap-2 text-[11px]">
            <span className="min-w-0 truncate text-book-text-main" title={ref.label}>
              {ref.label}
            </span>
            <span className="shrink-0 font-mono text-[10px] text-book-text-subtle">{ref.asset_type}</span>
          </div>
        ))}
      </div>
    ) : (
      <div className="px-2 py-1.5 text-[11px] text-book-text-subtle">{emptyText}</div>
    )}
  </div>
);

const GenerationInputAccess: React.FC<{
  record: AssetLineageRecord;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({ record, onOpenInputs }) => {
  const inputs = record.generation_inputs;
  return (
    <div className="rounded-sm border border-book-border bg-book-bg-paper">
      <div className="flex items-center justify-between gap-2 border-b border-book-border/70 px-2 py-1">
        <span className="text-[11px] font-semibold text-book-text-main">生成输入</span>
        <span className="font-mono text-[10px] text-book-text-subtle">{inputStatusLabel(record.input_status)}</span>
      </div>
      {inputs.length ? (
        <div className="px-2 py-1.5">
          <button
            type="button"
            onClick={() => onOpenInputs(record)}
            className="inline-flex h-7 w-full items-center justify-center gap-1.5 rounded-sm border border-book-border bg-book-bg px-2 text-[11px] font-semibold text-book-text-muted transition hover:border-book-primary/50 hover:text-book-primary"
            title="查看生成该资产的完整输入包"
          >
            <FileText size={12} />
            查看生成输入
          </button>
        </div>
      ) : (
        <div className="px-2 py-1.5 text-[11px] text-book-text-subtle">
          {generationInputEmptyText(record.input_status)}
        </div>
      )}
    </div>
  );
};

const PromptViewerOverlay: React.FC<{
  viewer: InputViewerState | null;
  onClose: () => void;
}> = ({ viewer, onClose }) => {
  if (!viewer) return null;
  return (
    <div className="absolute inset-0 z-20 flex items-center justify-center bg-black/35 p-4">
      <div className="flex max-h-full w-full max-w-5xl flex-col overflow-hidden rounded-sm border border-book-border bg-book-bg-paper shadow-xl">
        <div className="flex shrink-0 items-start justify-between gap-3 border-b border-book-border bg-book-bg-tertiary px-3 py-2.5">
          <div className="min-w-0">
            <h4 className="truncate text-sm font-semibold text-book-text-main" title={viewer.recordLabel}>
              生成输入
            </h4>
            <div className="mt-0.5 truncate text-[11px] text-book-text-muted" title={viewer.recordLabel}>
              {viewer.recordLabel}
            </div>
          </div>
          <BookButton variant="secondary" size="sm" onClick={onClose}>
            关闭
          </BookButton>
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-3 custom-scrollbar">
          <div className="space-y-3">
            {groupGenerationInputs(viewer.inputs).map(([group, inputs]) => (
              <section key={group} className="rounded-sm border border-book-border bg-book-bg">
                <div className="border-b border-book-border/70 px-2.5 py-1.5 text-[11px] font-semibold text-book-text-main">
                  {inputGroupLabel(group)}
                </div>
                <div className="space-y-2 p-2">
                  {inputs.map((input) => (
                    <GenerationInputCard key={input.id} input={input} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

const GenerationInputCard: React.FC<{ input: GenerationInputSource }> = ({ input }) => {
  const title = input.label || inputKindLabel(input.kind);
  return (
    <div className="overflow-hidden rounded-sm border border-book-border/70 bg-book-bg-paper">
      <div className="flex items-center justify-between gap-2 border-b border-book-border/70 px-2 py-1">
        <div className="min-w-0 truncate text-[10px] font-semibold text-book-text-muted" title={title}>
          {title}
        </div>
        <div className="shrink-0 font-mono text-[9px] text-book-text-subtle">
          {input.content_type}
        </div>
      </div>
      <div className="max-h-[58vh] overflow-auto px-2 py-2 custom-scrollbar">
        <GenerationInputBody input={input} />
      </div>
    </div>
  );
};

const GenerationInputBody: React.FC<{ input: GenerationInputSource }> = ({ input }) => {
  const value = generationInputValue(input);
  if (input.kind === "rendered_messages") {
    return <RenderedMessagesView value={value} />;
  }
  if (input.kind === "rendered_prompts") {
    return <RenderedPromptsView value={value} />;
  }
  if (typeof value === "string") {
    return <TextBlock text={value} />;
  }
  return <StructuredValueView value={value} />;
};

const RenderedMessagesView: React.FC<{ value: unknown }> = ({ value }) => {
  if (Array.isArray(value) && value.every(isGenerationStageMessages)) {
    return (
      <div className="space-y-3">
        {value.map((stage, index) => (
          <section key={`${String(stage.stage ?? index)}-${index}`} className="space-y-1.5">
            <div className="flex items-center gap-2 text-[11px] font-semibold text-book-text-main">
              <span className="rounded-sm border border-book-border bg-book-bg px-1.5 py-0.5 font-mono text-[10px] text-book-text-muted">
                {index + 1}
              </span>
              <span className="min-w-0 truncate" title={String(stage.stage ?? "阶段")}>
                {String(stage.stage ?? "阶段")}
              </span>
            </div>
            <MessageList value={stage.messages} />
          </section>
        ))}
      </div>
    );
  }
  return <MessageList value={value} />;
};

const MessageList: React.FC<{ value: unknown }> = ({ value }) => {
  if (!Array.isArray(value) || !value.every(isChatMessageLike)) {
    return <StructuredValueView value={value} />;
  }
  if (!value.length) {
    return <span className="text-[12px] text-book-text-subtle">空消息列表</span>;
  }
  return (
    <div className="space-y-2">
      {value.map((message, index) => (
        <MessageBlock key={`${String(message.role ?? "message")}-${index}`} message={message} index={index} />
      ))}
    </div>
  );
};

const MessageBlock: React.FC<{ message: Record<string, unknown>; index: number }> = ({ message, index }) => {
  const role = typeof message.role === "string" && message.role.trim() ? message.role : "message";
  const content = message.content;
  const extraEntries = Object.entries(message).filter(([key]) => key !== "role" && key !== "content");
  return (
    <article className="overflow-hidden rounded-sm border border-book-border bg-book-bg">
      <div className="flex items-center justify-between gap-2 border-b border-book-border/70 px-2 py-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="rounded-sm border border-book-border bg-book-bg-paper px-1.5 py-0.5 font-mono text-[10px] text-book-text-muted">
            {index + 1}
          </span>
          <span className="truncate text-[11px] font-semibold text-book-text-main">{messageRoleLabel(role)}</span>
        </div>
        <span className="shrink-0 font-mono text-[9px] text-book-text-subtle">{role}</span>
      </div>
      <div className="px-2 py-1.5">
        {typeof content === "string" ? (
          <TextBlock text={normalizeDisplayText(content)} />
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

const RenderedPromptsView: React.FC<{ value: unknown }> = ({ value }) => {
  if (typeof value === "string") {
    return <TextBlock text={value} />;
  }
  if (Array.isArray(value)) {
    return (
      <div className="space-y-2">
        {value.map((item, index) => (
          <PromptValueBlock key={index} title={`提示词 ${index + 1}`} value={item} />
        ))}
      </div>
    );
  }
  if (isRecord(value)) {
    return (
      <div className="space-y-2">
        {Object.entries(value).map(([key, item]) => (
          <PromptValueBlock key={key} title={key.replace(/_/g, " ")} value={item} />
        ))}
      </div>
    );
  }
  return <StructuredValueView value={value} />;
};

const PromptValueBlock: React.FC<{ title: string; value: unknown }> = ({ title, value }) => (
  <section className="overflow-hidden rounded-sm border border-book-border bg-book-bg">
    <div className="border-b border-book-border/70 px-2 py-1 text-[11px] font-semibold text-book-text-main">
      {title}
    </div>
    <div className="px-2 py-1.5">
      {typeof value === "string" ? <TextBlock text={value} /> : <StructuredValueView value={value} />}
    </div>
  </section>
);

const StructuredValueView: React.FC<{ value: unknown; depth?: number }> = ({ value, depth = 0 }) => {
  if (value == null) {
    return <span className="text-[12px] text-book-text-subtle">null</span>;
  }
  if (typeof value === "string") {
    return <TextBlock text={value} />;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return <span className="font-mono text-[12px] text-book-text-main">{String(value)}</span>;
  }
  if (Array.isArray(value)) {
    if (!value.length) {
      return <span className="text-[12px] text-book-text-subtle">空数组</span>;
    }
    if (value.every(isChatMessageLike)) {
      return <MessageList value={value} />;
    }
    return (
      <div className="space-y-1.5">
        {value.map((item, index) => (
          <StructuredValueItem key={index} title={`#${index + 1}`} value={item} depth={depth} />
        ))}
      </div>
    );
  }
  if (isRecord(value)) {
    return <StructuredEntriesView entries={Object.entries(value)} depth={depth} />;
  }
  return <TextBlock text={String(value)} />;
};

const StructuredEntriesView: React.FC<{ entries: Array<[string, unknown]>; depth: number }> = ({
  entries,
  depth,
}) => {
  if (!entries.length) {
    return <span className="text-[12px] text-book-text-subtle">空对象</span>;
  }
  return (
    <div className="space-y-1.5">
      {entries.map(([key, value]) => (
        <StructuredValueItem key={key} title={key.replace(/_/g, " ")} value={value} depth={depth} />
      ))}
    </div>
  );
};

const StructuredValueItem: React.FC<{ title: string; value: unknown; depth: number }> = ({
  title,
  value,
  depth,
}) => {
  if (typeof value === "string") {
    const parsed = normalizeSerializedValue(value);
    if (typeof parsed !== "string") {
      const parsedSummary = structuredValueSummary(parsed);
      return (
        <details
          className="overflow-hidden rounded-sm border border-book-border/70 bg-book-bg"
          open={shouldAutoOpenStructuredValue(parsed, depth)}
        >
          <summary className="cursor-pointer px-2 py-1 text-[11px] font-semibold text-book-text-muted">
            <span>{title}</span>
            {parsedSummary ? (
              <span className="ml-2 font-mono text-[10px] font-normal text-book-text-subtle">
                {parsedSummary}
              </span>
            ) : null}
          </summary>
          <div className="border-t border-book-border/70 px-2 py-1.5">
            <StructuredValueView value={parsed} depth={depth + 1} />
          </div>
        </details>
      );
    }
    const text = parsed;
    const compact = !text.includes("\n") && text.length <= INLINE_TEXT_MAX_CHARS;
    return compact ? (
      <div className="grid grid-cols-[9rem_minmax(0,1fr)] gap-2 rounded-sm border border-book-border/70 bg-book-bg px-2 py-1 text-[11px]">
        <div className="truncate font-semibold text-book-text-muted" title={title}>
          {title}
        </div>
        <div className="min-w-0 break-words text-book-text-main">{text}</div>
      </div>
    ) : (
      <details
        className="overflow-hidden rounded-sm border border-book-border/70 bg-book-bg"
        open={text.length <= AUTO_OPEN_TEXT_MAX_CHARS && depth === 0}
      >
        <summary className="cursor-pointer px-2 py-1 text-[11px] font-semibold text-book-text-muted">
          <span>{title}</span>
          <span className="ml-2 font-mono text-[10px] font-normal text-book-text-subtle">
            {textSummary(text)}
          </span>
        </summary>
        <div className="px-2 py-1.5">
          <TextBlock text={text} />
        </div>
      </details>
    );
  }
  if (value == null || typeof value === "number" || typeof value === "boolean") {
    return (
      <div className="grid grid-cols-[9rem_minmax(0,1fr)] gap-2 rounded-sm border border-book-border/70 bg-book-bg px-2 py-1 text-[11px]">
        <div className="truncate font-semibold text-book-text-muted" title={title}>
          {title}
        </div>
        <div className="min-w-0 break-words font-mono text-book-text-main">{String(value)}</div>
      </div>
    );
  }
  if (title === "messages" && Array.isArray(value) && value.every(isChatMessageLike)) {
    return (
      <details
        className="overflow-hidden rounded-sm border border-book-border/70 bg-book-bg"
        open={value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS && depth === 0}
      >
        <summary className="cursor-pointer px-2 py-1 text-[11px] font-semibold text-book-text-muted">
          <span>{title}</span>
          <span className="ml-2 font-mono text-[10px] font-normal text-book-text-subtle">
            {value.length} 条消息
          </span>
        </summary>
        <div className="space-y-1.5 border-t border-book-border/70 px-2 py-1.5">
          <MessageList value={value} />
        </div>
      </details>
    );
  }
  const summary = structuredValueSummary(value);
  return (
    <details
      className="overflow-hidden rounded-sm border border-book-border/70 bg-book-bg"
      open={shouldAutoOpenStructuredValue(value, depth)}
    >
      <summary className="cursor-pointer px-2 py-1 text-[11px] font-semibold text-book-text-muted">
        <span>{title}</span>
        {summary ? (
          <span className="ml-2 font-mono text-[10px] font-normal text-book-text-subtle">
            {summary}
          </span>
        ) : null}
      </summary>
      <div className="border-t border-book-border/70 px-2 py-1.5">
        <StructuredValueView value={value} depth={depth + 1} />
      </div>
    </details>
  );
};

function shouldAutoOpenStructuredValue(value: unknown, depth: number): boolean {
  if (depth > 0) return false;
  if (Array.isArray(value)) return value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS;
  if (isRecord(value)) return Object.keys(value).length <= AUTO_OPEN_OBJECT_MAX_FIELDS;
  return true;
}

function structuredValueSummary(value: unknown): string | null {
  if (Array.isArray(value)) return `${value.length} 项`;
  if (isRecord(value)) return `${Object.keys(value).length} 字段`;
  return null;
}

function textSummary(text: string): string {
  const lines = text.split("\n").length;
  return lines > 1 ? `${text.length} 字符 / ${lines} 行` : `${text.length} 字符`;
}

const TextBlock: React.FC<{ text: string }> = ({ text }) => (
  <pre className="m-0 max-h-[44vh] overflow-auto whitespace-pre-wrap break-words text-[12px] leading-relaxed text-book-text-main custom-scrollbar">
    {normalizeDisplayText(text)}
  </pre>
);

const JsonBlock: React.FC<{ title: string; value: unknown; defaultOpen?: boolean }> = ({
  title,
  value,
  defaultOpen,
}) => (
  <details className="rounded-sm border border-book-border bg-book-bg-paper" open={defaultOpen}>
    <summary className="cursor-pointer px-2 py-1 text-[11px] font-semibold text-book-text-main">
      {title}
    </summary>
    <div className="max-h-[52vh] overflow-auto border-t border-book-border/70 px-2 py-1.5 custom-scrollbar">
      <StructuredValueView value={normalizeSerializedValue(value)} />
    </div>
  </details>
);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function originLabel(origin: string): string {
  const labels: Record<string, string> = {
    derived_text: "文本派生",
    llm_generated: "LLM 生成",
    image_generated: "图片生成",
    video_generated: "视频生成",
    uploaded: "用户上传",
    imported: "导入资产",
    manual: "手动记录",
    extracted_frame: "抽帧生成",
    unknown: "来源不明",
  };
  return labels[origin] ?? origin;
}

function inputStatusLabel(status: string): string {
  if (status === "complete") return "完整";
  if (status === "missing") return "缺失";
  if (status === "run_without_inputs") return "无快照";
  if (status === "structural") return "结构来源";
  if (status === "not_applicable") return "不适用";
  return status;
}

function generationInputEmptyText(status: string): string {
  if (status === "structural") return "该记录已有结构化上游，但没有单独的生成输入快照。";
  if (status === "not_applicable") return "该记录不需要生成输入快照。";
  if (status === "run_without_inputs") return "生成运行存在，但缺少输入快照。";
  return "缺少生成输入记录。";
}

function groupGenerationInputs(inputs: GenerationInputSource[]): Array<[string, GenerationInputSource[]]> {
  const order = ["prompt", "variables", "source_text", "media", "request", "model", "response"];
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
    model: "模型配置",
    response: "响应摘要",
  };
  return labels[group] ?? group.replace(/_/g, " ");
}

function inputKindLabel(kind: string): string {
  const labels: Record<string, string> = {
    character_portrait_prompt: "立绘提示词",
    dialogue_text: "对白",
    image_generation_prompt: "图片提示词",
    input_variables: "输入变量",
    manga_image_prompt: "漫画提示词",
    manual_description: "描述",
    media_refs: "参考媒体",
    prompt_artifact: "统一提示词",
    rendered_messages: "渲染消息",
    rendered_prompts: "渲染提示词",
    request: "模型请求",
    request_snapshot: "模型请求快照",
    response_summary: "响应摘要",
    source_refs: "上游引用",
    stored_generated_video_prompt: "视频记录提示词",
    video_motion_prompt: "运动提示词",
    video_negative_prompt: "负面提示词",
    video_visual_prompt: "视觉提示词",
  };
  return labels[kind] ?? kind.replace(/_/g, " ");
}

function generationInputValue(input: GenerationInputSource): unknown {
  if (input.value != null) return normalizeSerializedValue(input.value);
  if (input.text != null && input.text.trim()) return normalizeSerializedValue(input.text);
  return "";
}

function normalizeSerializedValue(value: unknown): unknown {
  if (typeof value !== "string") return value;
  let current: unknown = value;
  for (let index = 0; index < 3 && typeof current === "string"; index += 1) {
    const parsed = parseJsonLikeString(current);
    if (parsed === undefined) break;
    current = parsed;
  }
  if (typeof current === "string") return normalizeDisplayText(current);
  return current;
}

function parseJsonLikeString(text: string): unknown | undefined {
  const trimmed = text.trim();
  if (!trimmed) return undefined;
  const startsLikeJson =
    (trimmed.startsWith("{") && trimmed.endsWith("}"))
    || (trimmed.startsWith("[") && trimmed.endsWith("]"))
    || (trimmed.startsWith("\"") && trimmed.endsWith("\""));
  if (!startsLikeJson) return undefined;
  try {
    return JSON.parse(trimmed);
  } catch {
    return undefined;
  }
}

function normalizeDisplayText(text: string): string {
  const parsed = parseJsonLikeString(text);
  if (typeof parsed === "string") return normalizeDisplayText(parsed);
  let next = text;
  if (!next.includes("\n") && /\\r|\\n|\\t/.test(next)) {
    next = next
      .replace(/\\r\\n/g, "\n")
      .replace(/\\n/g, "\n")
      .replace(/\\r/g, "\n")
      .replace(/\\t/g, "  ");
  }
  if (next.includes('\\"') && !next.includes('"')) {
    next = next.replace(/\\"/g, '"');
  }
  return next;
}

function isChatMessageLike(value: unknown): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  return "content" in value || "role" in value;
}

function isGenerationStageMessages(value: unknown): value is { stage?: unknown; messages: unknown } {
  return isRecord(value) && "messages" in value && Array.isArray(value.messages);
}

function messageRoleLabel(role: string): string {
  const labels: Record<string, string> = {
    system: "系统",
    user: "用户",
    assistant: "助手",
    tool: "工具",
    message: "消息",
  };
  return labels[role] ?? role;
}
