import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Network, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  getProjectAssetDependencyGraph,
  getProjectAssetDependencyGraphNodeDetail,
  type AssetDependencyGraph,
  type AssetGraphNodeDetail,
} from "../../../api/project";
import { extractErrorMessage } from "../../../api";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { THEME_APPLIED_EVENT } from "../../../theme/activeTheme";
import { useI18n } from "../../../i18n";
import {
  buildLayout,
  clamp,
  drawGraph,
  hitTestNode,
  nodeCenter,
  preferredInitialNodeId,
  summarizeGraph,
  type PositionedNode,
} from "./graphCanvas";
import {
  AssetNodeIndex,
  SelectedAssetDetails,
  ZoomControls,
} from "./assetGraphDetails";
import {
  PromptViewerModal,
  type InputViewerState,
} from "./assetGraphInputViewer";

interface AssetDependencyGraphDialogProps {
  open: boolean;
  projectId: number;
  onClose: () => void;
}

const MIN_ZOOM = 0.35;
const MAX_ZOOM = 2.25;
const ZOOM_STEP = 0.15;
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
        setDetailError(
          extractErrorMessage(err, t("novel.assetGraph.noSourceDetails")),
        );
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
    summary.text > 0
      ? t("novel.assetGraph.textCount", { count: summary.text })
      : null,
    summary.image > 0
      ? t("novel.assetGraph.imageCount", { count: summary.image })
      : null,
    summary.video > 0
      ? t("novel.assetGraph.videoCount", { count: summary.video })
      : null,
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
            {loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={14} />}
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
