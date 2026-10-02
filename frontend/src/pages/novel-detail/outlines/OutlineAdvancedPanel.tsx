import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { useToast } from "../../../components/feedback/Toast";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  getChapterOutlinesPacing,
  getGenerationProgress,
  outlineRecordTargets,
  previewApplyBeatMappingRequest,
  previewBatchGeneratePartOutlinesRequest,
  previewContinuePartOutlinesRequest,
  previewGeneratePartChapterOutlinesRequest,
  previewRegeneratePartOutlineRequest,
  previewRegeneratePartsRequest,
  type ChapterOutlinePacing,
  type OutlineGenerationProgressSummary,
  type PartOutline,
} from "../../../api/outline";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { isCancelledOperation } from "../../../utils/cancellation";
import { nonEmptyText } from "../../../utils/text";
import {
  formatNumber,
  useI18n,
  type Locale,
  type MessageTranslator,
} from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";
import type { RunOutlineGeneration } from "./useOutlineGeneration";
import { outlineOptimizationVariables } from "./outlineTabUtils";
import { AdvancedActionRow } from "./OutlineAdvancedPanelSections";
import { OutlineProgressPacingGrid } from "./OutlineProgressPacingGrid";

interface OutlineAdvancedPanelProps {
  projectId: number;
  parts: PartOutline[];
  chaptersPerPart: number | null;
  runGeneration: RunOutlineGeneration;
  disabled?: boolean;
  onChanged: () => boolean | void | Promise<boolean | void>;
  onBusyChange?: (busy: boolean) => void;
}

const DEFAULT_BEAT_MAP_JSON =
  '{"beats": [{"chapter": 1, "beat": "起"}, {"chapter": 5, "beat": "承"}]}';

const partOptionLabel = (
  part: PartOutline,
  locale: Locale,
  t: MessageTranslator,
): string => {
  const title = nonEmptyText(part.title);
  const heading = t("outline.partSection.part", {
    part: formatNumber(part.part_number, locale),
  });
  return title ? `${heading} · ${title}` : heading;
};

export const OutlineAdvancedPanel: React.FC<OutlineAdvancedPanelProps> = ({
  projectId,
  parts,
  chaptersPerPart,
  runGeneration,
  disabled = false,
  onChanged,
  onBusyChange,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const requestOptimizationDirection = (title: string) =>
    promptDialog({
      title,
      message: t("outline.dialogs.optimizationDirectionMessage"),
      placeholder: t("outline.dialogs.optimizationDirectionPlaceholder"),
      multiline: true,
      confirmText: t("outline.dialogs.startOptimization"),
    });
  const showGenerationSuccess = (message: string, warnings: string[]) => {
    addToast(
      warnings.length > 0
        ? `${message}${t("common.summarySeparator")}${warnings.join(t("common.summarySeparator"))}`
        : message,
      warnings.length > 0 ? "info" : "success",
    );
  };
  const refreshOutlines = async (): Promise<boolean> => {
    try {
      return (await onChanged()) !== false;
    } catch {
      return false;
    }
  };
  const warningsWithRefresh = (warnings: string[], refreshed: boolean): string[] =>
    refreshed ? warnings : [...warnings, t("novel.refreshFailed")];

  const [progress, setProgress] =
    useState<OutlineGenerationProgressSummary | null>(null);
  const [progressLoading, setProgressLoading] = useState(false);
  const [autoPoll, setAutoPoll] = useState(false);

  const [pacing, setPacing] = useState<ChapterOutlinePacing | null>(null);
  const [pacingLoading, setPacingLoading] = useState(false);

  const [beatJson, setBeatJson] = useState<string>(DEFAULT_BEAT_MAP_JSON);
  const [appliedBeatJson, setAppliedBeatJson] = useState(DEFAULT_BEAT_MAP_JSON);
  const [beatBusy, setBeatBusy] = useState(false);
  const beatMapDirty = beatJson !== appliedBeatJson;

  useNovelDetailDirty(
    `outline-beat-map:${projectId}`,
    beatMapDirty || beatBusy,
    () => setBeatJson(appliedBeatJson),
    () => !beatBusy,
  );

  const [selectedPartId, setSelectedPartId] = useState<number | "">("");
  const [partGenBusy, setPartGenBusy] = useState(false);
  const [partRegenBusy, setPartRegenBusy] = useState(false);

  const [continueBusy, setContinueBusy] = useState(false);
  const [regenPartsBusy, setRegenPartsBusy] = useState(false);
  const [batchBusy, setBatchBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);

  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const disabledRef = useRef(disabled);
  const partsRef = useRef<PartOutline[]>(parts);
  const progressSeqRef = useRef(0);
  const pacingSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewBusyRef = useRef(false);
  const confirmBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;
  disabledRef.current = disabled;
  partsRef.current = parts;

  const mutationBusy =
    beatBusy ||
    partGenBusy ||
    partRegenBusy ||
    continueBusy ||
    regenPartsBusy ||
    batchBusy;
  const interactionBusy = mutationBusy || requestPreviewLoading;
  const controlsDisabled = disabled || interactionBusy;

  useEffect(() => {
    onBusyChange?.(interactionBusy);
  }, [interactionBusy, onBusyChange]);

  useEffect(() => {
    return () => {
      if (!actionBusyRef.current) {
        onBusyChange?.(false);
      }
    };
  }, [onBusyChange]);

  const showGenerationError = useCallback(
    (err: unknown, fallback: string) => {
      const message = extractErrorMessage(err, fallback);
      if (!isCancelledOperation(message)) addToast(message, "error");
    },
    [addToast],
  );

  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const isProgressCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      progressSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPacingCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && pacingSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const ensureActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) => {
      if (!isActionCurrent(requestProjectId, requestSeq)) {
        throw new Error("任务已停止");
      }
    },
    [isActionCurrent],
  );

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (disabledRef.current) {
        addToast(t("outline.advancedPanel.operationInProgress"), "error");
        return null;
      }
      if (actionBusyRef.current) {
        addToast(
          t("outline.advancedPanel.advancedOperationInProgress"),
          "error",
        );
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      onBusyChange?.(true);
      return actionSeqRef.current;
    },
    [addToast, isProjectCurrent, onBusyChange, t],
  );

  const finishAction = useCallback(
    (requestSeq: number) => {
      if (actionSeqRef.current === requestSeq) {
        actionBusyRef.current = false;
        onBusyChange?.(false);
      }
    },
    [onBusyChange],
  );

  const beginPreview = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (previewBusyRef.current) return null;
      previewBusyRef.current = true;
      previewSeqRef.current += 1;
      return previewSeqRef.current;
    },
    [isProjectCurrent],
  );

  const finishPreview = useCallback((requestSeq: number) => {
    if (previewSeqRef.current === requestSeq) {
      previewBusyRef.current = false;
    }
  }, []);

  const beginConfirmAction = useCallback(
    (requestProjectId: number): boolean => {
      if (!isProjectCurrent(requestProjectId)) return false;
      if (
        disabledRef.current ||
        actionBusyRef.current ||
        confirmBusyRef.current
      ) {
        addToast(
          t("outline.advancedPanel.advancedOperationInProgress"),
          "error",
        );
        return false;
      }
      confirmBusyRef.current = true;
      return true;
    },
    [addToast, isProjectCurrent, t],
  );

  const finishConfirmAction = useCallback(() => {
    confirmBusyRef.current = false;
  }, []);

  const selectedPart = useCallback(
    (requestProjectId: number): PartOutline | null => {
      if (typeof selectedPartId !== "number") {
        addToast(t("outline.advancedPanel.selectPart"), "error");
        return null;
      }
      const part =
        partsRef.current.find((item) => item.id === selectedPartId) ?? null;
      if (!part || part.project_id !== requestProjectId) {
        addToast(t("outline.advancedPanel.partsChanged"), "error");
        return null;
      }
      return part;
    },
    [addToast, selectedPartId, t],
  );

  const closeRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const openPreview = async (
    requestProjectId: number,
    loadPreview: () => Promise<ModelRequestPreview>,
    fallback: string,
  ) => {
    const requestSeq = beginPreview(requestProjectId);
    if (requestSeq == null) return;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await loadPreview();
      if (!isPreviewCurrent(requestProjectId, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, fallback), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, requestSeq))
        setRequestPreviewLoading(false);
      finishPreview(requestSeq);
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      progressSeqRef.current += 1;
      pacingSeqRef.current += 1;
      previewSeqRef.current += 1;
      actionSeqRef.current += 1;
      actionBusyRef.current = false;
      previewBusyRef.current = false;
      confirmBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    partsRef.current = parts;
    progressSeqRef.current += 1;
    pacingSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    confirmBusyRef.current = false;
    onBusyChange?.(false);
    setProgress(null);
    setProgressLoading(false);
    setPacing(null);
    setPacingLoading(false);
    setAutoPoll(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setBeatBusy(false);
    setPartGenBusy(false);
    setPartRegenBusy(false);
    setContinueBusy(false);
    setRegenPartsBusy(false);
    setBatchBusy(false);
    setSelectedPartId("");
    setBeatJson(DEFAULT_BEAT_MAP_JSON);
    setAppliedBeatJson(DEFAULT_BEAT_MAP_JSON);
  }, [onBusyChange, projectId]);

  useEffect(() => {
    partsRef.current = parts;
    setSelectedPartId((current) => {
      if (current === "") return current;
      return parts.some(
        (part) => part.id === current && part.project_id === projectId,
      )
        ? current
        : "";
    });
  }, [parts, projectId]);

  const loadProgress = useCallback(async () => {
    const requestProjectId = projectId;
    const requestSeq = progressSeqRef.current + 1;
    progressSeqRef.current = requestSeq;
    setProgressLoading(true);
    try {
      const result = await getGenerationProgress(requestProjectId);
      if (!isProgressCurrent(requestProjectId, requestSeq)) return;
      setProgress(result);
    } catch (err) {
      if (isProgressCurrent(requestProjectId, requestSeq)) {
        addToast(
          extractErrorMessage(err, t("outline.advancedPanel.progressFailed")),
          "error",
        );
      }
    } finally {
      if (isProgressCurrent(requestProjectId, requestSeq))
        setProgressLoading(false);
    }
  }, [addToast, isProgressCurrent, projectId, t]);

  useEffect(() => {
    if (!autoPoll || disabled) return;
    void loadProgress();
    const timer = setInterval(() => void loadProgress(), 3000);
    return () => clearInterval(timer);
  }, [autoPoll, disabled, loadProgress]);

  const loadPacing = async () => {
    const requestProjectId = projectId;
    const requestSeq = pacingSeqRef.current + 1;
    pacingSeqRef.current = requestSeq;
    setPacingLoading(true);
    try {
      const result = await getChapterOutlinesPacing(requestProjectId);
      if (!isPacingCurrent(requestProjectId, requestSeq)) return;
      setPacing(result);
    } catch (err) {
      if (isPacingCurrent(requestProjectId, requestSeq)) {
        addToast(
          extractErrorMessage(err, t("outline.advancedPanel.pacingFailed")),
          "error",
        );
      }
    } finally {
      if (isPacingCurrent(requestProjectId, requestSeq))
        setPacingLoading(false);
    }
  };

  const handleBeatMap = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    let payload: Record<string, unknown>;
    try {
      const value = JSON.parse(beatJson);
      if (typeof value !== "object" || value === null) {
        throw new Error(t("outline.advancedPanel.invalidObject"));
      }
      payload = value as Record<string, unknown>;
    } catch (err) {
      addToast(
        t("outline.advancedPanel.jsonParseFailed", {
          error: extractErrorMessage(err, ""),
        }),
        "error",
      );
      return;
    }
    const beatSheet = JSON.stringify(payload);
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setBeatBusy(true);
    let refreshAttempted = false;
    let refreshSucceeded = true;
    try {
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.applyBeat"),
          startMessage: t("outline.advancedPanel.applyingBeat"),
          successMessage: t("outline.advancedPanel.beatApplied"),
          errorMessage: t("outline.advancedPanel.applyBeatFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "apply_beat_mapping",
            part_id: requestPart.id,
            beat_sheet: beatSheet,
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshAttempted = true;
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setAppliedBeatJson(beatJson);
      showGenerationSuccess(t("outline.advancedPanel.beatApplied"), warningsWithRefresh(warnings, refreshSucceeded));
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        if (!refreshAttempted) await onChanged();
        showGenerationError(err, t("outline.advancedPanel.applyBeatFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setBeatBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewBeatMap = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    let payload: Record<string, unknown>;
    try {
      const value = JSON.parse(beatJson);
      if (typeof value !== "object" || value === null) {
        throw new Error(t("outline.advancedPanel.invalidObject"));
      }
      payload = value as Record<string, unknown>;
    } catch (err) {
      addToast(
        t("outline.advancedPanel.jsonParseFailed", {
          error: extractErrorMessage(err, ""),
        }),
        "error",
      );
      return;
    }
    const beatSheet = JSON.stringify(payload);
    await openPreview(
      requestProjectId,
      () =>
        previewApplyBeatMappingRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
          beat_sheet: beatSheet,
        }),
      t("outline.advancedPanel.previewBeatFailed"),
    );
  };

  const handleGeneratePartChapters = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setPartGenBusy(true);
    let refreshAttempted = false;
    let refreshSucceeded = true;
    try {
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.generatePartChapters", {
            part: formatNumber(requestPart.part_number, locale),
          }),
          startMessage: t("outline.advancedPanel.generatingPartChapters", {
            part: formatNumber(requestPart.part_number, locale),
          }),
          successMessage: t("outline.advancedPanel.generatedPartChapters", {
            part: formatNumber(requestPart.part_number, locale),
          }),
          errorMessage: t("outline.advancedPanel.generateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "generate_part_chapter_outlines",
            part_id: requestPart.id,
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshAttempted = true;
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      showGenerationSuccess(
        t("outline.advancedPanel.generatedPartChapters", {
          part: formatNumber(requestPart.part_number, locale),
        }),
        warningsWithRefresh(warnings, refreshSucceeded),
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        if (!refreshAttempted) await onChanged();
        showGenerationError(err, t("outline.advancedPanel.generateFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setPartGenBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewGeneratePartChapters = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    await openPreview(
      requestProjectId,
      () =>
        previewGeneratePartChapterOutlinesRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
        }),
      t("outline.advancedPanel.previewPartChaptersFailed"),
    );
  };

  const handleBatchPartOutlines = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast(t("outline.advancedPanel.noPartsToGenerate"), "error");
      return;
    }
    const foreignPart = requestParts.find(
      (part) => part.project_id !== requestProjectId,
    );
    if (foreignPart) {
      addToast(t("outline.advancedPanel.foreignPartsGenerate"), "error");
      return;
    }
    const partIds = requestParts.map((part) => part.id) as [
      number,
      ...number[],
    ];
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setBatchBusy(true);
    let refreshAttempted = false;
    let refreshSucceeded = true;
    try {
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.batchGenerate"),
          startMessage: t("outline.advancedPanel.batchGenerating", {
            count: formatNumber(partIds.length, locale),
          }),
          successMessage: t("outline.advancedPanel.batchGenerated"),
          errorMessage: t("outline.advancedPanel.batchFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "batch_generate_part_outlines",
            part_ids: partIds,
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshAttempted = true;
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      showGenerationSuccess(
        t("outline.advancedPanel.batchGenerated"),
        warningsWithRefresh(warnings, refreshSucceeded),
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        if (!refreshAttempted) await onChanged();
        showGenerationError(err, t("outline.advancedPanel.batchFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setBatchBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewBatchPartOutlines = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast(t("outline.advancedPanel.noPartsToGenerate"), "error");
      return;
    }
    const foreignPart = requestParts.find(
      (part) => part.project_id !== requestProjectId,
    );
    if (foreignPart) {
      addToast(t("outline.advancedPanel.foreignPartsPreview"), "error");
      return;
    }
    const partIds = requestParts.map((part) => part.id);
    await openPreview(
      requestProjectId,
      () =>
        previewBatchGeneratePartOutlinesRequest({
          project_id: requestProjectId,
          part_ids: partIds,
        }),
      t("outline.advancedPanel.previewBatchFailed"),
    );
  };

  const handleRegenPartOutline = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setPartRegenBusy(true);
    let refreshSucceeded = true;
    try {
      const direction = await requestOptimizationDirection(
        t("outline.dialogs.refinePart", { part: requestPart.part_number }),
      );
      if (direction === null || !isActionCurrent(requestProjectId, actionSeq))
        return;
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.regeneratePart", {
            part: formatNumber(requestPart.part_number, locale),
          }),
          startMessage: t("outline.advancedPanel.regeneratingPart", {
            part: formatNumber(requestPart.part_number, locale),
          }),
          successMessage: t("outline.advancedPanel.regeneratedPart"),
          errorMessage: t("outline.regenerateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_part_outline",
            part_id: requestPart.id,
            variables: outlineOptimizationVariables(direction),
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      showGenerationSuccess(
        t("outline.advancedPanel.regeneratedPart"),
        warningsWithRefresh(warnings, refreshSucceeded),
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        showGenerationError(err, t("outline.regenerateFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setPartRegenBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenPartOutline = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    const direction = await requestOptimizationDirection(
      t("outline.dialogs.refinePart", { part: requestPart.part_number }),
    );
    if (direction === null || !isProjectCurrent(requestProjectId)) return;
    await openPreview(
      requestProjectId,
      () =>
        previewRegeneratePartOutlineRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
          variables: outlineOptimizationVariables(direction),
        }),
      t("outline.advancedPanel.regeneratePartPreviewFailed"),
    );
  };

  const handleRegenAllParts = async () => {
    const requestProjectId = projectId;
    const requestChaptersPerPart = chaptersPerPart;
    let expectedParts: ReturnType<typeof outlineRecordTargets>;
    try {
      expectedParts = outlineRecordTargets(parts);
    } catch (error) {
      addToast(
        extractErrorMessage(
          error,
          t("outline.advancedPanel.confirmPartsFailed"),
        ),
        "error",
      );
      return;
    }
    const requestPartCount = expectedParts.length;
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: t("outline.advancedPanel.regenerateAllParts"),
      message: t("outline.advancedPanel.regenerateAllPartsMessage", {
        count: formatNumber(requestPartCount, locale),
        range:
          requestChaptersPerPart == null
            ? t("outline.advancedPanel.reuseChapterSpan")
            : t("outline.advancedPanel.repartition", {
                count: formatNumber(requestChaptersPerPart, locale),
              }),
      }),
      confirmText: t("outline.advancedPanel.regenerateConfirm"),
      dialogType: "warning",
    });
    if (!ok) {
      finishConfirmAction();
      return;
    }
    if (!isProjectCurrent(requestProjectId)) {
      finishConfirmAction();
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setRegenPartsBusy(true);
    let refreshAttempted = false;
    let refreshSucceeded = true;
    try {
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.regenerateAll"),
          startMessage: t("outline.advancedPanel.regeneratingAll"),
          successMessage: t("outline.advancedPanel.regeneratedAll"),
          errorMessage: t("outline.regenerateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_parts",
            expected_parts: expectedParts,
            chapters_per_part: requestChaptersPerPart,
            variables: null,
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshAttempted = true;
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      showGenerationSuccess(
        t("outline.advancedPanel.regeneratedAll"),
        warningsWithRefresh(warnings, refreshSucceeded),
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        if (!refreshAttempted) await onChanged();
        showGenerationError(err, t("outline.regenerateFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq))
        setRegenPartsBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenAllParts = async () => {
    const requestProjectId = projectId;
    const requestChaptersPerPart = chaptersPerPart;
    let expectedParts: ReturnType<typeof outlineRecordTargets>;
    try {
      expectedParts = outlineRecordTargets(parts);
    } catch (error) {
      addToast(
        extractErrorMessage(
          error,
          t("outline.advancedPanel.confirmPartsFailed"),
        ),
        "error",
      );
      return;
    }
    await openPreview(
      requestProjectId,
      () =>
        previewRegeneratePartsRequest({
          project_id: requestProjectId,
          expected_parts: expectedParts,
          chapters_per_part: requestChaptersPerPart,
          variables: null,
        }),
      t("outline.advancedPanel.regenerateAllPreviewFailed"),
    );
  };

  const handleContinue = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast(t("outline.advancedPanel.continueNeedPart"), "error");
      return;
    }
    const foreignPart = requestParts.find(
      (part) => part.project_id !== requestProjectId,
    );
    if (foreignPart) {
      addToast(t("outline.advancedPanel.foreignPartsContinue"), "error");
      return;
    }
    const lastPart = requestParts.reduce((latest, part) =>
      part.part_number > latest.part_number ? part : latest,
    );
    const chapterSpan = lastPart.end_chapter - lastPart.start_chapter + 1;
    if (chapterSpan <= 0) {
      addToast(t("outline.advancedPanel.invalidLastPartRange"), "error");
      return;
    }
    const startChapter = lastPart.end_chapter + 1;
    const endChapter = startChapter + chapterSpan - 1;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setContinueBusy(true);
    let refreshAttempted = false;
    let refreshSucceeded = true;
    try {
      const warnings = await runGeneration(
        {
          title: t("outline.advancedPanel.continuePart", {
            part: formatNumber(lastPart.part_number + 1, locale),
          }),
          startMessage: t("outline.advancedPanel.continuingPart", {
            part: formatNumber(lastPart.part_number + 1, locale),
            start: formatNumber(startChapter, locale),
            end: formatNumber(endChapter, locale),
          }),
          successMessage: t("outline.advancedPanel.continuedPart", {
            part: formatNumber(lastPart.part_number + 1, locale),
          }),
          errorMessage: t("outline.advancedPanel.continueFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "continue_part_outlines",
            part_number: lastPart.part_number + 1,
            start_chapter: startChapter,
            end_chapter: endChapter,
            variables: null,
          },
        },
        async (control) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          control.setProgress(t("outline.advancedPanel.refreshOutlines"), 2);
          refreshAttempted = true;
          refreshSucceeded = await refreshOutlines();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      showGenerationSuccess(
        t("outline.advancedPanel.continuedPart", {
          part: formatNumber(lastPart.part_number + 1, locale),
        }),
        warningsWithRefresh(warnings, refreshSucceeded),
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        if (!refreshAttempted) await onChanged();
        showGenerationError(err, t("outline.advancedPanel.continueFailed"));
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setContinueBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewContinue = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast(t("outline.advancedPanel.continueNeedPart"), "error");
      return;
    }
    const foreignPart = requestParts.find(
      (part) => part.project_id !== requestProjectId,
    );
    if (foreignPart) {
      addToast(t("outline.advancedPanel.foreignPartsPreview"), "error");
      return;
    }
    const lastPart = requestParts.reduce((latest, part) =>
      part.part_number > latest.part_number ? part : latest,
    );
    const chapterSpan = lastPart.end_chapter - lastPart.start_chapter + 1;
    if (chapterSpan <= 0) {
      addToast(t("outline.advancedPanel.invalidLastPartRange"), "error");
      return;
    }
    const startChapter = lastPart.end_chapter + 1;
    const endChapter = startChapter + chapterSpan - 1;
    await openPreview(
      requestProjectId,
      () =>
        previewContinuePartOutlinesRequest({
          project_id: requestProjectId,
          part_number: lastPart.part_number + 1,
          start_chapter: startChapter,
          end_chapter: endChapter,
          variables: null,
        }),
      t("outline.advancedPanel.previewContinueFailed"),
    );
  };

  return (
    <div className="space-y-4">
      <OutlineProgressPacingGrid
        progress={progress}
        progressLoading={progressLoading}
        autoPoll={autoPoll}
        onToggleAutoPoll={() => setAutoPoll((current) => !current)}
        onLoadProgress={loadProgress}
        disabled={disabled}
        controlsDisabled={controlsDisabled}
        pacing={pacing}
        pacingLoading={pacingLoading}
        onLoadPacing={loadPacing}
      />

      <section className="space-y-2">
        <div>
          <h4 className="text-sm font-medium text-book-text-main">
            {t("outline.advancedPanel.selectedPart")}
          </h4>
          <p className="mt-0.5 text-small text-book-text-muted">
            {t("outline.advancedPanel.selectedPartHint")}
          </p>
        </div>
        <ThemedSelect
          value={selectedPartId}
          onChange={(selectedValue) =>
            setSelectedPartId(
              selectedValue ? Number(selectedValue) : "",
            )
          }
          disabled={controlsDisabled}
          className="pyqt-input w-full"
        >
          <option value="">
            {t("outline.advancedPanel.selectPartPlaceholder")}
          </option>
          {parts.map((part) => (
            <option key={part.id} value={part.id}>
              {partOptionLabel(part, locale, t)}
            </option>
          ))}
        </ThemedSelect>
        <div>
          <AdvancedActionRow
            title={t("outline.advancedPanel.generateSelectedPart")}
            description={t("outline.advancedPanel.generateSelectedPartDescription")}
            actionLabel={t("outline.advancedPanel.generateSelectedPartAction")}
            actionBusyLabel={t(
              "outline.advancedPanel.generateSelectedPartBusy",
            )}
            busy={partGenBusy}
            disabled={controlsDisabled || selectedPartId === ""}
            onPreview={handlePreviewGeneratePartChapters}
            onAction={handleGeneratePartChapters}
            standalone
          />
        </div>
      </section>

      <section className="space-y-2">
        <div>
          <h4 className="text-sm font-medium text-book-text-main">
            {t("outline.advancedPanel.allParts")}
          </h4>
          <p className="mt-0.5 text-small text-book-text-muted">
            {t("outline.advancedPanel.allPartsHint")}
          </p>
        </div>
        <div className="grid gap-2">
          <AdvancedActionRow
            title={t("outline.advancedPanel.batchGenerate")}
            description={t("outline.advancedPanel.batchDescription")}
            actionLabel={t("outline.advancedPanel.batchAction")}
            actionBusyLabel={t(
              "outline.advancedPanel.generateSelectedPartBusy",
            )}
            busy={batchBusy}
            disabled={controlsDisabled || parts.length === 0}
            onPreview={handlePreviewBatchPartOutlines}
            onAction={handleBatchPartOutlines}
            standalone
          />
          <AdvancedActionRow
            title={t("outline.advancedPanel.continueAction")}
            description={t("outline.advancedPanel.continueDescription")}
            actionLabel={t("outline.advancedPanel.continueAction")}
            actionBusyLabel={t("outline.advancedPanel.continuingAction")}
            busy={continueBusy}
            disabled={controlsDisabled || parts.length === 0}
            onPreview={handlePreviewContinue}
            onAction={handleContinue}
            standalone
          />
        </div>
      </section>

      <details className="group">
        <summary className="a4n-flat-surface flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
          <span className="font-medium text-book-text-main">
            {t("outline.advancedPanel.rewriteSection")}
          </span>
          <span className="min-w-0 flex-1 truncate text-right text-small text-book-text-muted">
            {t("outline.advancedPanel.rewriteSectionHint")}
          </span>
        </summary>
        <div className="mt-2 grid gap-2">
          <AdvancedActionRow
            title={t("outline.advancedPanel.rewriteSelected")}
            description={t("outline.advancedPanel.rewriteSelectedDescription")}
            actionLabel={t("outline.advancedPanel.rewriteSelectedAction")}
            actionBusyLabel={t("outline.advancedPanel.rewriting")}
            busy={partRegenBusy}
            disabled={controlsDisabled || selectedPartId === ""}
            warning
            onPreview={handlePreviewRegenPartOutline}
            onAction={handleRegenPartOutline}
            standalone
          />
          <AdvancedActionRow
            title={t("outline.advancedPanel.rewriteAll")}
            description={t("outline.advancedPanel.rewriteAllDescription")}
            actionLabel={t("outline.advancedPanel.replan")}
            actionBusyLabel={t("outline.advancedPanel.replanning")}
            busy={regenPartsBusy}
            disabled={controlsDisabled || parts.length === 0}
            warning
            onPreview={handlePreviewRegenAllParts}
            onAction={handleRegenAllParts}
            standalone
          />
        </div>
      </details>

      <details className="a4n-flat-surface overflow-hidden">
        <summary className="cursor-pointer px-3 py-3 text-sm font-medium text-book-text-main">
          {t("outline.advancedPanel.beatMapping")}
        </summary>
        <div className="space-y-3 border-t border-book-border/40 p-3 text-xs">
          <p className="text-small leading-relaxed text-book-text-muted">
            {t("outline.advancedPanel.beatMappingHint")}
          </p>
          <textarea
            value={beatJson}
            onChange={(event) => setBeatJson(event.target.value)}
            disabled={controlsDisabled}
            className="pyqt-input h-24 w-full resize-y font-mono text-small"
          />
          <div className="flex items-center justify-end gap-2">
            {!beatBusy ? (
              <Dropdown
                items={[
                  {
                    label: t("outline.advanced.viewRequest"),
                    icon: <Eye size={12} />,
                    onClick: handlePreviewBeatMap,
                    disabled: controlsDisabled || selectedPartId === "",
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleBeatMap}
              disabled={controlsDisabled || selectedPartId === ""}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {beatBusy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" />{" "}
                  {t("outline.advancedPanel.applying")}
                </span>
              ) : (
                t("outline.advancedPanel.applyBeatAction")
              )}
            </BookButton>
          </div>
        </div>
      </details>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("outline.advancedPanel.requestPreview")}
        onClose={closeRequestPreview}
      />
    </div>
  );
};
