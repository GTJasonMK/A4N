import React from "react";
import { FileText, Maximize2, ZoomIn, ZoomOut } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  type AssetGraphNode,
  type AssetGraphNodeDetail,
  type AssetLineageRecord,
  type AssetLineageRef,
  type GenerationInputSource,
} from "../../../api/project";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { formatNumber, useI18n } from "../../../i18n";
import {
  assetTypeLabel,
  isVideoChainNode,
  kindLabel,
  nodeCenter,
  primaryAssetIndexNodes,
  type PositionedNode,
} from "./graphCanvas";

export const ZoomControls: React.FC<{
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
          aria-label={t("novel.assetGraph.resetZoomAria", {
            percent: zoomPercent,
          })}
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

export const AssetNodeIndex: React.FC<{
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
          {videoNodes.length
            ? t("novel.assetGraph.videoChain")
            : t("novel.assetGraph.assetIndex")}
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

export const SelectedAssetDetails: React.FC<{
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
  const nodeStatus = node.status
    ? getOperationStatusLabel(node.status, locale)
    : null;

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
            <span>
              {t("novel.assetGraph.sourceCount", { count: records.length })}
            </span>
            {inputCount > 0 ? (
              <span>
                {t("novel.assetGraph.generationInputCount", {
                  count: inputCount,
                })}
              </span>
            ) : null}
            {warningCount > 0 ? (
              <span className="text-warning">
                {t("novel.assetGraph.warningCount", { count: warningCount })}
              </span>
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
                    {t("novel.assetGraph.recordInputCount", {
                      count: record.generation_inputs.length,
                    })}
                  </span>
                ) : null}
              </div>
              <div className="mt-0.5 truncate text-caption text-book-text-subtle">
                {[
                  originLabel(record.origin, t),
                  record.status
                    ? getOperationStatusLabel(record.status, locale)
                    : null,
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
        <DetailTerm
          label={t("novel.assetGraph.source")}
          value={originLabel(record.origin, t)}
        />
        <DetailTerm
          label={t("novel.assetGraph.asset")}
          value={assetTypeLabel(record.asset_kind, t)}
        />
        <DetailTerm
          label={t("novel.assetGraph.status")}
          value={
            record.status
              ? getOperationStatusLabel(record.status, locale)
              : t("novel.assetGraph.unmarked")
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
            <DetailTerm
              label={t("novel.assetGraph.nodeId")}
              value={node.id}
              title={node.id}
            />
            <DetailTerm
              label={t("novel.assetGraph.nodeKind")}
              value={node.kind}
              title={node.kind}
            />
            <DetailTerm
              label={t("novel.assetGraph.nodeAsset")}
              value={node.asset_type}
              title={node.asset_type}
            />
            {node.chapter_id != null ? (
              <DetailTerm
                label={t("novel.assetGraph.chapterNumber")}
                value={node.chapter_id}
              />
            ) : null}
            {node.profile_id != null ? (
              <DetailTerm
                label={t("novel.assetGraph.profileId")}
                value={node.profile_id}
              />
            ) : null}
            {node.record_id != null ? (
              <DetailTerm
                label={t("novel.assetGraph.nodeRecord")}
                value={node.record_id}
              />
            ) : null}
            <DetailTerm
              label={t("novel.assetGraph.mutationPolicy")}
              value={node.mutation_policy}
              title={node.mutation_policy}
            />
            <DetailTerm
              label={t("novel.assetGraph.recordId")}
              value={record.id}
              title={record.id}
            />
            <DetailTerm
              label={t("novel.assetGraph.assetType")}
              value={record.asset_kind}
              title={record.asset_kind}
            />
            {record.asset_id != null ? (
              <DetailTerm
                label={t("novel.assetGraph.assetId")}
                value={record.asset_id}
              />
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
          <JsonBlock
            title={t("novel.assetGraph.rawResult")}
            value={record.result}
          />
          {record.request_snapshot ? (
            <JsonBlock
              title={t("novel.assetGraph.requestSnapshot")}
              value={record.request_snapshot}
            />
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
      <div className="text-small font-semibold text-book-text-main">
        {title}
      </div>
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
