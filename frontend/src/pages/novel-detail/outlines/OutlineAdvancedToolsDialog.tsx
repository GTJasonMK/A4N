import { useCallback, useEffect, useState } from "react";
import { Eye, Loader2, SlidersHorizontal } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { OutlineAdvancedPanel } from "./OutlineAdvancedPanel";
import {
  DELETE_LATEST_CHAPTER_OUTLINES_ACTION,
  DELETE_LATEST_PARTS_ACTION,
  type NumberInputValue,
} from "./outlineTabUtils";
import type { ChapterOutline, PartOutline } from "../../../api/outline";

interface OutlineAdvancedToolsDialogProps {
  open: boolean;
  projectId: number;
  parts: PartOutline[];
  outlines: ChapterOutline[];
  partCount: NumberInputValue;
  chaptersPerPart: NumberInputValue;
  delPartCount: number;
  delChapterCount: number;
  delActing: string | null;
  regenLastPart: boolean;
  runProgressTask: RunProgressTask;
  disabled?: boolean;
  onClose: () => void;
  onDelPartCountChange: (value: number) => void;
  onDelChapterCountChange: (value: number) => void;
  onRegenLastPart: () => void;
  onPreviewRegenLastPart: () => void;
  onDeleteLatestParts: () => void;
  onDeleteLatestOutlines: () => void;
  onChanged: () => void | Promise<void>;
}

export function OutlineAdvancedToolsDialog({
  open,
  projectId,
  parts,
  outlines,
  partCount,
  chaptersPerPart,
  delPartCount,
  delChapterCount,
  delActing,
  regenLastPart,
  runProgressTask,
  disabled = false,
  onClose,
  onDelPartCountChange,
  onDelChapterCountChange,
  onRegenLastPart,
  onPreviewRegenLastPart,
  onDeleteLatestParts,
  onDeleteLatestOutlines,
  onChanged,
}: OutlineAdvancedToolsDialogProps) {
  const [advancedBusy, setAdvancedBusy] = useState(false);
  const deletingParts = delActing === DELETE_LATEST_PARTS_ACTION;
  const deletingChapterOutlines =
    delActing === DELETE_LATEST_CHAPTER_OUTLINES_ACTION;
  const actionBusy = advancedBusy || regenLastPart || delActing != null;
  const maintenanceBusy = disabled || advancedBusy || regenLastPart || delActing != null;
  const advancedPanelDisabled = disabled || regenLastPart || delActing != null;

  useEffect(() => {
    setAdvancedBusy(false);
  }, [projectId]);

  const handleAdvancedBusyChange = useCallback((busy: boolean) => {
    setAdvancedBusy(busy);
  }, []);

  return (
    <Modal
      isOpen={open}
      onClose={() => {
        if (!actionBusy) onClose();
      }}
      title="大纲高级工具"
      subtitle={`共 ${parts.length} 部、${outlines.length} 个章纲 · 生成、诊断与维护`}
      icon={<SlidersHorizontal size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!actionBusy}
      closeOnEscape={!actionBusy}
      showCloseButton={!actionBusy}
    >
      <div className="space-y-4">
        <OutlineAdvancedPanel
          projectId={projectId}
          parts={parts}
          partCount={typeof partCount === "number" ? partCount : null}
          chaptersPerPart={typeof chaptersPerPart === "number" ? chaptersPerPart : null}
          runProgressTask={runProgressTask}
          disabled={advancedPanelDisabled}
          onChanged={onChanged}
          onBusyChange={handleAdvancedBusyChange}
        />

        <details className="group" aria-label="末尾维护与删除">
          <summary className="a4n-flat-surface flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
            <span className="font-medium text-book-text-main">末尾维护与删除</span>
            <span className="min-w-0 flex-1 truncate text-right text-[11px] text-book-text-muted">
              重写最后一部、删除最近内容
            </span>
          </summary>
          <div className="mt-2 grid gap-2 text-xs">
            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">重写最后一部</h3>
                <p className="mt-1 text-[11px] leading-relaxed text-book-text-muted">
                  保留更早分部，只重写故事末尾的阶段规划。
                </p>
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                {!regenLastPart ? (
                  <Dropdown
                    items={[
                      {
                        label: "查看模型请求",
                        icon: <Eye size={12} />,
                        onClick: onPreviewRegenLastPart,
                        disabled: maintenanceBusy,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onRegenLastPart}
                  disabled={maintenanceBusy}
                  className="min-w-0 flex-1 !text-xs sm:flex-none"
                >
                  {regenLastPart ? (
                    <span className="inline-flex items-center gap-1.5">
                      <Loader2 size={12} className="animate-spin" /> 重写中…
                    </span>
                  ) : (
                    "重写最后一部"
                  )}
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">删除最近分部</h3>
                <p className="mt-1 text-[11px] leading-relaxed text-book-text-muted">
                  从故事末尾删除指定数量的分部，操作前会再次确认。
                </p>
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                <label className="inline-flex items-center gap-1.5 text-book-text-muted">
                  <input
                    type="number"
                    min={1}
                    max={Math.max(parts.length, 1)}
                    value={delPartCount}
                    disabled={maintenanceBusy}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      onDelPartCountChange(
                        Number.isFinite(value)
                          ? Math.min(Math.max(Math.trunc(value), 1), Math.max(parts.length, 1))
                          : 1,
                      );
                    }}
                    className="pyqt-input !w-16"
                  />
                  部
                </label>
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={onDeleteLatestParts}
                  disabled={maintenanceBusy || deletingParts || parts.length === 0}
                  className="!text-xs"
                >
                  {deletingParts ? (
                    <span className="inline-flex items-center gap-1.5">
                      <Loader2 size={12} className="animate-spin" /> 删除中…
                    </span>
                  ) : (
                    "删除"
                  )}
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">删除最近章纲</h3>
                <p className="mt-1 text-[11px] leading-relaxed text-book-text-muted">
                  删除后，对应章节会回到尚未规划的状态。
                </p>
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                <label className="inline-flex items-center gap-1.5 text-book-text-muted">
                  <input
                    type="number"
                    min={1}
                    max={Math.max(outlines.length, 1)}
                    value={delChapterCount}
                    disabled={maintenanceBusy}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      onDelChapterCountChange(
                        Number.isFinite(value)
                          ? Math.min(Math.max(Math.trunc(value), 1), Math.max(outlines.length, 1))
                          : 1,
                      );
                    }}
                    className="pyqt-input !w-16"
                  />
                  章
                </label>
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={onDeleteLatestOutlines}
                  disabled={maintenanceBusy || deletingChapterOutlines || outlines.length === 0}
                  className="!text-xs"
                >
                  {deletingChapterOutlines ? (
                    <span className="inline-flex items-center gap-1.5">
                      <Loader2 size={12} className="animate-spin" /> 删除中…
                    </span>
                  ) : (
                    "删除"
                  )}
                </BookButton>
              </div>
            </section>
          </div>
        </details>
      </div>
    </Modal>
  );
}
