import React, { useMemo, useState } from "react";
import { ChevronRight, Images, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { ImageSizeSelect } from "../../../components/image/ImageSizeSelect";
import { useI18n } from "../../../i18n";
import type { ImageSize } from "../../../api/image";
import type {
  GalgameAssetPlan,
  GalgameAssetPlanItem,
  GalgameSpeakerProfile,
} from "../../../api/galgame";
import {
  assetPlanOutcomeLabel,
  assetSpecStatusLabel,
  buildPortraitExpressionCells,
  buildPortraitExpressionMatrixModel,
  portraitExpressionLabel,
  poseDisplayValue,
} from "./galgameTabShared";

export const AssetPlanPanel: React.FC<{
  plan: GalgameAssetPlan | null;
  speakerProfiles: GalgameSpeakerProfile[];
  loading: boolean;
  generating: boolean;
  generatingSpecId: number | null;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  disabled: boolean;
  onPlan: () => void;
  onGenerate: () => void;
  onGenerateSpec: (generationSpecId: number) => void;
}> = ({
  plan,
  speakerProfiles,
  loading,
  generating,
  generatingSpecId,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  onImageSizeChange,
  disabled,
  onPlan,
  onGenerate,
  onGenerateSpec,
}) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const planSummary = plan
    ? [
        plan.summary.background_specs > 0
          ? t("galgame.assetSummary", {
              label: t("galgame.background"),
              count: plan.summary.background_specs,
            })
          : null,
        plan.summary.portrait_expression_specs > 0
          ? t("galgame.assetSummary", {
              label: t("galgame.portraitExpressions"),
              count: plan.summary.portrait_expression_specs,
            })
          : null,
        plan.summary.event_cg_specs > 0
          ? t("galgame.assetSummary", {
              label: "CG",
              count: plan.summary.event_cg_specs,
            })
          : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : null;
  const outcomeSummary = plan
    ? [
        plan.summary.created_specs > 0
          ? t("galgame.assetSummary", {
              label: t("galgame.assetOutcome.created"),
              count: plan.summary.created_specs,
            })
          : null,
        plan.summary.updated_specs > 0
          ? t("galgame.assetSummary", {
              label: t("galgame.assetOutcome.updated"),
              count: plan.summary.updated_specs,
            })
          : null,
        plan.summary.reused_specs > 0
          ? t("galgame.assetSummary", {
              label: t("galgame.assetOutcome.reused"),
              count: plan.summary.reused_specs,
            })
          : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : null;
  return (
    <section
      aria-label={t("galgame.assetPlan")}
      className="a4n-flat-surface overflow-hidden text-xs"
    >
      <button type="button" onClick={() => setOpen(true)} aria-haspopup="dialog" className="flex w-full items-center gap-2 px-3 py-3 text-left font-medium text-book-text-main hover:bg-book-bg-tertiary/55">
        <span>{t("galgame.visualAssets")}</span>
        <span className="flex-1" />
        {loading || generating ? (
          <LoadingSpinner size="sm" />
        ) : null}
        <span className="text-small font-normal text-book-text-muted">
          {plan ? t("galgame.assetPlanPlanned") : t("galgame.assetPlanUnplanned")}
        </span>
        <ChevronRight size={14} className="shrink-0 text-book-text-muted" />
      </button>
      <Modal isOpen={open} onClose={() => setOpen(false)} title={t("galgame.visualAssets")} maxWidthClassName="max-w-4xl" footer={(
        <div className="flex w-full flex-wrap items-center justify-end gap-2">
          {plan ? (
            <Dropdown items={[{ label: t("galgame.replanVisualAssets"), icon: <RefreshCw size={13} />, onClick: onPlan, disabled }]} />
          ) : null}
        <BookButton
          variant="primary"
          size="sm"
          onClick={plan ? onGenerate : onPlan}
          disabled={disabled || imageSize == null}
        >
          {loading || generating ? (
            <LoadingSpinner size="sm" />
          ) : (
            <Images size={13} />
          )}
          {plan
            ? t("galgame.generateVisualAssets")
            : t("galgame.planVisualAssets")}
        </BookButton>
        </div>
      )}>
      <div className="space-y-4">
        <div className="max-w-xs">
          <ImageSizeSelect label={t("settingsForm.selectImageSize")} value={imageSize} sizes={imageSizes} loading={imageSizeLoading}
            error={imageSizeError} disabled={disabled} onChange={onImageSizeChange} />
        </div>
        {!plan ? (
          <div className="mt-2 text-small text-book-text-muted">
            {t("galgame.assetPlanHint")}
          </div>
        ) : (
          <div className="mt-2 space-y-2 text-small text-book-text-muted">
            {planSummary ? <p>{planSummary}</p> : null}
            {outcomeSummary ? (
              <p className="text-caption text-book-text-subtle">
                {outcomeSummary}
              </p>
            ) : null}
            <AssetPlanList title={t("galgame.background")} items={plan.backgrounds} />
            <PortraitExpressionMatrix
              items={plan.portrait_expressions}
              speakerProfiles={speakerProfiles}
              disabled={disabled}
              generatingSpecId={generatingSpecId}
              onGenerateSpec={onGenerateSpec}
            />
            <AssetPlanList title="CG" items={plan.event_cgs} />
          </div>
        )}
      </div>
      </Modal>
    </section>
  );
};

const AssetPlanList: React.FC<{
  title: string;
  items: GalgameAssetPlan["backgrounds"];
}> = ({ title, items }) => {
  const { t } = useI18n();
  if (!items.length) return null;
  return (
    <div className="border-t border-book-border pt-2">
      <div className="mb-1 font-semibold text-book-text-main">{title}</div>
      <div className="space-y-1">
        {items.map((item) => (
          <div
            key={item.canonical_key}
            className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2"
          >
            <span className="min-w-0 truncate" title={item.label}>
              {item.label}
            </span>
            <span>{assetPlanOutcomeLabel(item.outcome, t)}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

export const PortraitExpressionMatrix: React.FC<{
  items: GalgameAssetPlanItem[];
  speakerProfiles: GalgameSpeakerProfile[];
  disabled: boolean;
  generatingSpecId: number | null;
  onGenerateSpec: (generationSpecId: number) => void;
}> = ({
  items,
  speakerProfiles,
  disabled,
  generatingSpecId,
  onGenerateSpec,
}) => {
  const { t } = useI18n();
  const cells = useMemo(
    () => buildPortraitExpressionCells(items, speakerProfiles),
    [items, speakerProfiles],
  );
  const { rows, expressionValues, cellsByKey } = useMemo(
    () => buildPortraitExpressionMatrixModel(cells),
    [cells],
  );
  if (!cells.length) return null;

  return (
    <div className="border-t border-book-border pt-2">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="font-semibold text-book-text-main">{t("galgame.portraitExpressions")}</span>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {t("galgame.itemCount", { count: cells.length })}
        </span>
      </div>
      <div className="custom-scrollbar overflow-x-auto" role="region" tabIndex={0} aria-label={t("galgame.portraitExpressions")}>
        <table className="w-full min-w-[360px] border-collapse text-small">
          <thead>
            <tr className="text-left text-book-text-muted">
              <th className="border border-book-border bg-book-bg-paper px-2 py-1 font-medium">
                {t("novel.characters")}
              </th>
              {expressionValues.map((expression) => (
                <th
                  key={expression || "none"}
                  className="border border-book-border bg-book-bg-paper px-2 py-1 font-medium"
                >
                  {portraitExpressionLabel(expression, t)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map(({ rowKey, firstCell }) => {
              return (
                <tr key={rowKey}>
                  <th className="max-w-28 border border-book-border bg-book-bg-paper px-2 py-1 text-left font-medium text-book-text-main">
                    <div className="truncate" title={firstCell.profileName}>
                      {firstCell.profileName}
                    </div>
                    <div
                      className="truncate text-book-text-muted"
                      title={poseDisplayValue(firstCell.pose, t)}
                    >
                      {poseDisplayValue(firstCell.pose, t)}
                    </div>
                  </th>
                  {expressionValues.map((expression) => {
                    const cell = cellsByKey.get(`${rowKey}:${expression}`);
                    const generating =
                      generatingSpecId === cell?.item.generation_spec_id;
                    return (
                      <td
                        key={`${rowKey}:${expression || "none"}`}
                        className="border border-book-border bg-book-bg px-1 py-1 align-top"
                      >
                        {cell ? (
                          <div className="flex min-w-20 flex-col gap-1">
                            <span
                              className={`truncate ${
                                cell.item.status === "failed"
                                  ? "text-error"
                                  : "text-book-text-muted"
                              }`}
                              title={cell.item.label}
                            >
                              {assetSpecStatusLabel(cell.item.status, t)}
                            </span>
                            <BookButton
                              variant="secondary"
                              size="sm"
                              className="w-full"
                              disabled={disabled}
                              onClick={() =>
                                onGenerateSpec(cell.item.generation_spec_id)
                              }
                            >
                              {generating ? (
                                <LoadingSpinner size="sm" />
                              ) : (
                                <RefreshCw size={12} />
                              )}
                              {t("galgame.generateAsset")}
                            </BookButton>
                          </div>
                        ) : (
                          <span className="text-book-text-muted">-</span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};
