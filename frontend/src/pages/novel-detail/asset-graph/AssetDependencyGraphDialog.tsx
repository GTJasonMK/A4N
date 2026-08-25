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
import { formatNumber, useI18n } from "../../../i18n";
import {
  assetTypeLabel,
  buildLayout,
  clamp,
  drawGraph,
  hitTestNode,
  isVideoChainNode,
  kindLabel,
  nodeCenter,
  primaryAssetIndexNodes,
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
  const { t } = useI18n();
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
      setError(extractErrorMessage(err, t("novel.assetGraph.loading")));
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

  const layout = useMemo(() => buildLayout(graph, t), [graph, t]);
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
        setDetailError(extractErrorMessage(err, t("novel.assetGraph.noSourceDetails")));
      })
      .finally(() => {
        if (detailRequestSeqRef.current === requestSeq) setDetailLoading(false);
      });

    return () => {
      if (detailRequestSeqRef.current === requestSeq) {
        detailRequestSeqRef.current += 1;
      }
    };
  }, [open, projectId, selectedNode, t]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    drawGraph(canvas, layout, graph?.edges ?? [], selectedId, t);
  }, [graph?.edges, layout, selectedId, t, themeVersion]);

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
    summary.text > 0 ? t("novel.assetGraph.textCount", { count: summary.text }) : null,
    summary.image > 0 ? t("novel.assetGraph.imageCount", { count: summary.image }) : null,
    summary.video > 0 ? t("novel.assetGraph.videoCount", { count: summary.video }) : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <>
      <Modal
        isOpen={open}
        onClose={onClose}
        title={t("novel.assetGraph.title")}
        subtitle={t("novel.assetGraph.subtitle")}
        icon={<Network size={15} />}
        maxWidthClassName="max-w-[min(1280px,96vw)]"
        bodyClassName="!overflow-hidden p-0"
        headerExtra={
          <button
            type="button"
            onClick={loadGraph}
            disabled={loading}
            className="pyqt-btn-icon !h-9 !w-9 sm:!h-8 sm:!w-8"
            title={t("novel.assetGraph.refresh")}
            aria-label={t("novel.assetGraph.refreshAria")}
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
              ariaLabel={t("novel.assetGraph.viewAria")}
              value={mobilePane}
              onChange={setMobilePane}
              options={[
                { id: "graph", label: t("novel.assetGraph.graph") },
                { id: "details", label: t("novel.assetGraph.details") },
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
                  {t("novel.assetGraph.loading")}
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
                {summaryText || t("novel.assetGraph.emptyAssets")}
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
}> = ({ zoom, onZoomIn, onZoomOut, onReset }) => {
  const { locale, t } = useI18n();
  const zoomPercent = formatNumber(Math.round(zoom * 100), locale);
  return (
  <div className="absolute left-3 top-3 z-10 flex w-max items-center gap-1 rounded-2xl border border-book-border/45 bg-book-bg-paper/90 p-1 shadow-[0_8px_24px_-16px_rgb(0_0_0/0.45)] backdrop-blur-xl">
    <button
      type="button"
      onClick={onZoomOut}
      className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
      title={t("novel.assetGraph.zoomOut")}
      aria-label={t("novel.assetGraph.zoomOutAria")}
    >
      <ZoomOut size={14} />
    </button>
    {Math.abs(zoom - 1) > 0.01 ? (
      <button
        type="button"
        onClick={onReset}
        className="inline-flex h-9 min-w-14 items-center justify-center gap-1 rounded-xl px-2 text-caption font-medium tabular-nums text-book-text-muted transition hover:bg-book-bg-tertiary hover:text-book-text-main sm:h-8"
        title={t("novel.assetGraph.resetZoom")}
        aria-label={t("novel.assetGraph.resetZoomAria", { percent: zoomPercent })}
      >
        <Maximize2 size={12} />
        {zoomPercent}%
      </button>
    ) : null}
    <button
      type="button"
      onClick={onZoomIn}
      className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
      title={t("novel.assetGraph.zoomIn")}
      aria-label={t("novel.assetGraph.zoomInAria")}
    >
      <ZoomIn size={14} />
    </button>
  </div>
  );
};

const AssetNodeIndex: React.FC<{
  nodes: PositionedNode[];
  selectedId: string | null;
  onSelect: (nodeId: string) => void;
}> = ({ nodes, selectedId, onSelect }) => {
  const { t } = useI18n();
  const visibleNodes = nodes.filter(
    (node) => node.id !== "project" && node.count > 0,
  );
  const videoNodes = visibleNodes.filter(isVideoChainNode);
  const primaryNodes = primaryAssetIndexNodes(visibleNodes, selectedId);
  if (!primaryNodes.length) return null;

  return (
    <section className="mt-4">
      <div className="flex items-center justify-between gap-3 px-1">
        <h3 className="text-xs font-semibold text-book-text-main">
          {videoNodes.length ? t("novel.assetGraph.videoChain") : t("novel.assetGraph.assetIndex")}
        </h3>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {t("novel.assetGraph.nodeCount", { count: primaryNodes.length })}
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
                {t("novel.assetGraph.itemCount", { count: node.count })}
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
  const { locale, t } = useI18n();
  if (!node) {
    return (
      <div className="a4n-flat-surface mt-4 px-3 py-4 text-small text-book-text-subtle">
        {t("novel.assetGraph.selectNode")}
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
  const nodeStatus = node.status ? getOperationStatusLabel(node.status, locale) : null;

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
            kindLabel(node.kind, t),
            assetTypeLabel(node.asset_type, t),
            nodeStatus,
            t("novel.assetGraph.itemCount", { count: node.count }),
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
          {t("novel.assetGraph.loadingRecords")}
        </div>
      ) : error ? (
        <div className="border-t border-error/25 bg-error/5 px-3 py-3 text-small leading-relaxed text-error">
          {error}
        </div>
      ) : detail ? (
        <>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-book-border/35 px-3 py-2.5 text-small text-book-text-muted">
            <span>{t("novel.assetGraph.sourceCount", { count: records.length })}</span>
            {inputCount > 0 ? <span>{t("novel.assetGraph.generationInputCount", { count: inputCount })}</span> : null}
            {warningCount > 0 ? (
              <span className="text-warning">{t("novel.assetGraph.warningCount", { count: warningCount })}</span>
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
          {t("novel.assetGraph.noSourceDetails")}
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
  const { locale, t } = useI18n();
  if (!records.length) {
    return (
      <div className="border-t border-book-border/35 px-3 py-3 text-small text-book-text-subtle">
        {t("novel.assetGraph.noRecords")}
      </div>
    );
  }

  return (
    <section className="border-t border-book-border/35 px-3 py-3">
      <div className="text-small font-semibold text-book-text-main">
        {t("novel.assetGraph.sourceRecords")}
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
                    {t("novel.assetGraph.recordInputCount", { count: record.generation_inputs.length })}
                  </span>
                ) : null}
              </div>
              <div className="mt-0.5 truncate text-caption text-book-text-subtle">
                {[
                  originLabel(record.origin, t),
                  record.status ? getOperationStatusLabel(record.status, locale) : null,
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
  const { locale, t } = useI18n();
  if (!record) return null;
  return (
    <section className="border-t border-book-border/35 px-3 py-3">
      <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-small">
        <DetailTerm label={t("novel.assetGraph.source")} value={originLabel(record.origin, t)} />
        <DetailTerm label={t("novel.assetGraph.asset")} value={assetTypeLabel(record.asset_kind, t)} />
        <DetailTerm
          label={t("novel.assetGraph.status")}
          value={
            record.status ? getOperationStatusLabel(record.status, locale) : t("novel.assetGraph.unmarked")
          }
        />
      </dl>

      <RefList
        title={t("novel.assetGraph.upstream")}
        refs={record.source_refs}
        emptyText={t("novel.assetGraph.noUpstream")}
      />
      <RefList
        title={t("novel.assetGraph.mediaReferences")}
        refs={record.media_sources}
        emptyText={t("novel.assetGraph.noMediaReferences")}
      />
      <details className="mt-3 border-t border-book-border/35 pt-1 text-caption text-book-text-muted">
        <summary className="cursor-pointer py-2 hover:text-book-text-main">
          {t("novel.assetGraph.diagnostics")}
        </summary>
        <div className="space-y-2 border-l border-book-border/35 pl-3">
          <dl className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 py-1.5">
            <DetailTerm label={t("novel.assetGraph.nodeId")} value={node.id} title={node.id} />
            <DetailTerm label={t("novel.assetGraph.nodeKind")} value={node.kind} title={node.kind} />
            <DetailTerm
              label={t("novel.assetGraph.nodeAsset")}
              value={node.asset_type}
              title={node.asset_type}
            />
            {node.chapter_id != null ? (
              <DetailTerm label={t("novel.assetGraph.chapterNumber")} value={node.chapter_id} />
            ) : null}
            {node.profile_id != null ? (
              <DetailTerm label={t("novel.assetGraph.profileId")} value={node.profile_id} />
            ) : null}
            {node.record_id != null ? (
              <DetailTerm label={t("novel.assetGraph.nodeRecord")} value={node.record_id} />
            ) : null}
            <DetailTerm
              label={t("novel.assetGraph.mutationPolicy")}
              value={node.mutation_policy}
              title={node.mutation_policy}
            />
            <DetailTerm label={t("novel.assetGraph.recordId")} value={record.id} title={record.id} />
            <DetailTerm
              label={t("novel.assetGraph.assetType")}
              value={record.asset_kind}
              title={record.asset_kind}
            />
            {record.asset_id != null ? (
              <DetailTerm label={t("novel.assetGraph.assetId")} value={record.asset_id} />
            ) : null}
            <DetailTerm
              label={t("novel.assetGraph.originType")}
              value={record.origin}
              title={record.origin}
            />
            {record.status ? (
              <DetailTerm
                label={t("novel.assetGraph.rawStatus")}
                value={record.status}
                title={record.status}
              />
            ) : null}
          </dl>
          <GenerationInputAccess record={record} onOpenInputs={onOpenInputs} />
          <JsonBlock title={t("novel.assetGraph.rawResult")} value={record.result} />
          {record.request_snapshot ? (
            <JsonBlock title={t("novel.assetGraph.requestSnapshot")} value={record.request_snapshot} />
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
}> = ({ title, refs, emptyText }) => {
  const { t } = useI18n();
  return (
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
                {assetTypeLabel(ref.asset_type, t)}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <div className="mt-1 text-small text-book-text-subtle">{emptyText}</div>
      )}
    </section>
  );
};

const GenerationInputAccess: React.FC<{
  record: AssetLineageRecord;
  onOpenInputs: (record: AssetLineageRecord) => void;
}> = ({ record, onOpenInputs }) => {
  const { t } = useI18n();
  const inputs = record.generation_inputs;
  return (
    <div className="border-t border-book-border/30 py-2">
      {inputs.length ? (
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 text-caption text-book-text-muted">
            {t("novel.assetGraph.generationInputs", {
              count: inputs.length,
              status: inputStatusLabel(record.input_status, t),
            })}
          </div>
          <button
            type="button"
            onClick={() => onOpenInputs(record)}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-lg px-2 py-1 text-caption font-medium text-book-primary transition hover:bg-book-primary/10"
            title={t("novel.assetGraph.viewInputsTitle")}
          >
            <FileText size={11} />
            {t("novel.assetGraph.view")}
          </button>
        </div>
      ) : (
        <div className="text-caption leading-relaxed text-book-text-subtle">
          {generationInputEmptyText(record.input_status, t)}
        </div>
      )}
    </div>
  );
};

const PromptViewerModal: React.FC<{
  viewer: InputViewerState | null;
  onClose: () => void;
}> = ({ viewer, onClose }) => {
  const { t } = useI18n();
  if (!viewer) return null;
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={t("novel.assetGraph.generationInputTitle")}
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
                {inputGroupLabel(group, t)}
              </div>
              <div className="mt-0.5 text-caption tabular-nums text-book-text-subtle">
                {t("novel.assetGraph.inputItems", { count: inputs.length })}
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
  const { t } = useI18n();
  if (!Array.isArray(value) || !value.every(isChatMessageLike)) {
    return <StructuredValueView value={value} />;
  }
  if (!value.length) {
    return (
      <span className="text-body-sm text-book-text-subtle">{t("novel.assetGraph.emptyMessages")}</span>
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
  const { t } = useI18n();
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
          {messageRoleLabel(role, t)}
        </span>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {t("novel.assetGraph.messageNumber", { number: index + 1 })}
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
  const { t } = useI18n();
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
      return <span className="text-body-sm text-book-text-subtle">{t("novel.assetGraph.emptyArray")}</span>;
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
  const { t } = useI18n();
  if (!entries.length) {
    return <span className="text-body-sm text-book-text-subtle">{t("novel.assetGraph.emptyObject")}</span>;
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
  const { t } = useI18n();
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
            {textSummary(text, t)}
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
            {t("novel.assetGraph.messagesSummary", { count: value.length })}
          </span>
        </summary>
        <div className="border-l border-book-border/35 pb-2 pl-3">
          <MessageList value={value} />
        </div>
      </details>
    );
  }
  const summary = structuredValueSummary(value, t);
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

function structuredValueSummary(value: unknown, t: ReturnType<typeof useI18n>["t"]): string | null {
  if (Array.isArray(value)) return t("novel.assetGraph.arraySummary", { count: value.length });
  if (isRecord(value)) return t("novel.assetGraph.objectSummary", { count: Object.keys(value).length });
  return null;
}

function textSummary(
  text: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const lines = text.split("\n").length;
  return lines > 1
    ? t("novel.assetGraph.textSummaryLines", { chars: text.length, lines })
    : t("novel.assetGraph.textSummaryChars", { chars: text.length });
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

function originLabel(origin: string, t: ReturnType<typeof useI18n>["t"]): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    derived_text: "novel.assetGraph.origin.derivedText",
    llm_generated: "novel.assetGraph.origin.llmGenerated",
    image_generated: "novel.assetGraph.origin.imageGenerated",
    video_generated: "novel.assetGraph.origin.videoGenerated",
    uploaded: "novel.assetGraph.origin.uploaded",
    imported: "novel.assetGraph.origin.imported",
    manual: "novel.assetGraph.origin.manual",
    extracted_frame: "novel.assetGraph.origin.extractedFrame",
    unknown: "novel.assetGraph.origin.unknown",
  };
  return t(labels[origin] ?? "novel.assetGraph.origin.other");
}

function inputStatusLabel(status: string, t: ReturnType<typeof useI18n>["t"]): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    complete: "novel.assetGraph.inputStatus.complete",
    missing: "novel.assetGraph.inputStatus.missing",
    run_without_inputs: "novel.assetGraph.inputStatus.runWithoutInputs",
    structural: "novel.assetGraph.inputStatus.structural",
    not_applicable: "novel.assetGraph.inputStatus.notApplicable",
  };
  return t(labels[status] ?? "novel.assetGraph.inputStatus.unknown");
}

function generationInputEmptyText(status: string, t: ReturnType<typeof useI18n>["t"]): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    structural: "novel.assetGraph.inputEmpty.structural",
    not_applicable: "novel.assetGraph.inputEmpty.notApplicable",
    run_without_inputs: "novel.assetGraph.inputEmpty.runWithoutInputs",
    missing: "novel.assetGraph.inputEmpty.missing",
  };
  return t(labels[status] ?? "novel.assetGraph.inputEmpty.missing");
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

function inputGroupLabel(group: string, t: ReturnType<typeof useI18n>["t"]): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    prompt: "novel.assetGraph.inputGroup.prompt",
    variables: "novel.assetGraph.inputGroup.variables",
    source_text: "novel.assetGraph.inputGroup.sourceText",
    media: "novel.assetGraph.inputGroup.media",
    request: "novel.assetGraph.inputGroup.request",
    response: "novel.assetGraph.inputGroup.response",
  };
  return t(labels[group] ?? "novel.assetGraph.inputGroup.other");
}

function isChatMessageLike(value: unknown): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  return "content" in value || "role" in value;
}

function messageRoleLabel(role: string, t: ReturnType<typeof useI18n>["t"]): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    system: "novel.assetGraph.messageRole.system",
    user: "novel.assetGraph.messageRole.user",
    assistant: "novel.assetGraph.messageRole.assistant",
    tool: "novel.assetGraph.messageRole.tool",
    message: "novel.assetGraph.messageRole.message",
  };
  return t(labels[role] ?? "novel.assetGraph.messageRole.message");
}
