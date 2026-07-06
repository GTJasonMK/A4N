import { useCallback, useEffect, useState } from "react";
import { Eye, Loader2, SlidersHorizontal } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
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
      onClose={onClose}
      title="大纲高级工具"
      subtitle="进度查询、维护操作、部级章纲批处理和节拍映射统一收纳在这里。"
      icon={<SlidersHorizontal size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
    >
      <div className="space-y-5">
        <section className="space-y-3 text-xs">
          <h3 className="font-serif text-sm text-book-text-main">维护操作</h3>
          <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
            <div className="space-y-2 rounded-sm border border-book-border/40 bg-book-bg-paper/25 p-3">
              <div className="font-semibold text-book-text-main">重写最后一部</div>
              <p className="text-[11px] leading-relaxed text-book-text-muted">
                用于主线收束不自然时快速重写末尾分部。
              </p>
              <div className="flex flex-wrap gap-2">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onPreviewRegenLastPart}
                  disabled={maintenanceBusy}
                >
                  <span className="inline-flex items-center gap-1.5">
                    <Eye size={12} /> 预览提示词
                  </span>
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onRegenLastPart}
                  disabled={maintenanceBusy}
                >
                  {regenLastPart ? (
                    <span className="inline-flex items-center gap-2">
                      <Loader2 size={12} className="animate-spin" /> 重写中…
                    </span>
                  ) : (
                    "重新生成最后一部"
                  )}
                </BookButton>
              </div>
            </div>

            <div className="space-y-2 rounded-sm border border-error/30 bg-error-bg/30 p-3">
              <div className="font-semibold text-error">删除最近分部</div>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min={1}
                  max={parts.length}
                  value={delPartCount}
                  disabled={maintenanceBusy}
                  onChange={(event) => onDelPartCountChange(Number(event.target.value))}
                  className="pyqt-input !px-2 !py-1 w-16"
                />
                <span className="text-book-text-muted">部</span>
                <button
                  type="button"
                  onClick={onDeleteLatestParts}
                  disabled={maintenanceBusy || deletingParts || parts.length === 0}
                  className="pyqt-btn-secondary !min-h-9 !border-error/50 !px-2.5 text-xs !text-error hover:!bg-error-bg sm:!min-h-[28px]"
                >
                  {deletingParts ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    "删除"
                  )}
                </button>
              </div>
            </div>

            <div className="space-y-2 rounded-sm border border-error/30 bg-error-bg/30 p-3">
              <div className="font-semibold text-error">删除最近章纲</div>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min={1}
                  max={outlines.length}
                  value={delChapterCount}
                  disabled={maintenanceBusy}
                  onChange={(event) => onDelChapterCountChange(Number(event.target.value))}
                  className="pyqt-input !px-2 !py-1 w-16"
                />
                <span className="text-book-text-muted">章</span>
                <button
                  type="button"
                  onClick={onDeleteLatestOutlines}
                  disabled={maintenanceBusy || deletingChapterOutlines || outlines.length === 0}
                  className="pyqt-btn-secondary !min-h-9 !border-error/50 !px-2.5 text-xs !text-error hover:!bg-error-bg sm:!min-h-[28px]"
                >
                  {deletingChapterOutlines ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    "删除"
                  )}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section className="border-t border-book-border/40 pt-4">
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
        </section>
      </div>
    </Modal>
  );
}
