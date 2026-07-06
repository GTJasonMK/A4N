import type { ReactNode } from "react";
import { AlertCircle, Check, CheckCircle2, Edit3, Eye, Loader2, Trash2, Zap } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { formatConfigTestTime } from "./testResultUtils";
import type { ConfigEditorRenderContext, ConfigItem, ConfigTestResult } from "./types";

interface ConfigEditorPanelProps<C> {
  title: string;
  creating: boolean;
  selectedItem: ConfigItem<C> | null;
  form: C;
  saving: boolean;
  activating: number | null;
  deleting: number | null;
  testing: number | null;
  previewing: boolean;
  testResult: ConfigTestResult | null;
  canTest: boolean;
  canPreviewTest: boolean;
  activateText: string;
  activeDeleteTitle: string;
  renderEditor: (
    form: C,
    setForm: (config: C) => void,
    context: ConfigEditorRenderContext,
  ) => ReactNode;
  onFormChange: (config: C) => void;
  onCancel: () => void | Promise<void>;
  onSave: () => void | Promise<void>;
  onActivate: (id: number) => void | Promise<void>;
  onDelete: (id: number) => void | Promise<void>;
  onPreviewTest: (id: number) => void | Promise<void>;
  onTest: (id: number) => void | Promise<void>;
}

export function ConfigEditorPanel<C>({
  title,
  creating,
  selectedItem,
  form,
  saving,
  activating,
  deleting,
  testing,
  previewing,
  testResult,
  canTest,
  canPreviewTest,
  activateText,
  activeDeleteTitle,
  renderEditor,
  onFormChange,
  onCancel,
  onSave,
  onActivate,
  onDelete,
  onPreviewTest,
  onTest,
}: ConfigEditorPanelProps<C>) {
  const itemActionBusy = activating != null || deleting != null || testing != null || previewing;

  return (
    <section className="pyqt-card overflow-y-auto p-3 sm:p-5 custom-scrollbar">
      {!creating && !selectedItem ? (
        <div className="h-full flex flex-col items-center justify-center text-center text-book-text-muted">
          <Edit3 size={32} className="text-book-primary/30 mb-3" />
          <p className="text-sm">
            从左侧选择一项进行编辑，
            <br />
            或点击"新增 {title}"开始。
          </p>
        </div>
      ) : (
        <>
          <div className="mb-4 flex flex-col gap-3 border-b border-book-border/60 pb-3 sm:flex-row sm:items-center sm:justify-between">
            <h3 className="text-sm font-serif text-book-text-main">
              {creating ? `新建 ${title}` : `编辑 ${title}`}
            </h3>
            <ConfigEditorActions
              selectedItem={selectedItem}
              creating={creating}
              saving={saving}
              activating={activating}
              deleting={deleting}
              testing={testing}
              previewing={previewing}
              canTest={canTest}
              canPreviewTest={canPreviewTest}
              activateText={activateText}
              activeDeleteTitle={activeDeleteTitle}
              onActivate={onActivate}
              onDelete={onDelete}
              onPreviewTest={onPreviewTest}
              onTest={onTest}
            />
          </div>

          {canTest && !creating ? (
            <ConnectionTestStatus
              selectedItem={selectedItem}
              testing={testing}
              testResult={testResult}
            />
          ) : null}

          {selectedItem?.readonly ? (
            <div className="mb-4 rounded border border-book-primary/30 bg-book-primary/5 px-3 py-2 text-xs text-book-text-muted">
              {selectedItem.readonlyReason ?? "该配置由系统维护，不能在这里编辑。"}
            </div>
          ) : null}

          <div className="flex flex-col gap-4">
            {renderEditor(
              form,
              selectedItem?.readonly ? () => undefined : onFormChange,
              {
                selectedId: selectedItem?.id ?? null,
                readonly: selectedItem?.readonly === true,
              },
            )}
          </div>

          <div className="mt-6 flex flex-col-reverse gap-2 border-t border-book-border/60 pt-4 sm:flex-row sm:items-center sm:justify-end">
            <BookButton
              variant="secondary"
              onClick={() => void onCancel()}
              disabled={saving || itemActionBusy}
            >
              取消
            </BookButton>
            {!selectedItem?.readonly ? (
              <BookButton
                variant="primary"
                onClick={() => void onSave()}
                disabled={saving || itemActionBusy}
              >
                {saving ? (
                  <span className="inline-flex items-center gap-2">
                    <Loader2 size={14} className="animate-spin" /> 保存中…
                  </span>
                ) : creating ? (
                  "创建"
                ) : (
                  "保存"
                )}
              </BookButton>
            ) : null}
          </div>
        </>
      )}
    </section>
  );
}

const ConnectionTestStatus = <C,>({
  selectedItem,
  testing,
  testResult,
}: {
  selectedItem: ConfigItem<C> | null;
  testing: number | null;
  testResult: ConfigTestResult | null;
}) => {
  const running = selectedItem != null && testing === selectedItem.id;
  if (!running && !testResult) {
    return (
      <div className="a4n-stat-tile mb-4 flex items-center gap-2 px-3 py-2 text-xs text-book-text-muted">
        <AlertCircle size={13} />
        <span>尚未测试当前已保存配置。</span>
      </div>
    );
  }

  if (running) {
    return (
      <div className="mb-4 flex items-center gap-2 rounded border border-book-primary/45 bg-book-primary/5 px-3 py-2 text-xs text-book-primary">
        <Loader2 size={13} className="animate-spin" />
        <span>正在测试当前已保存配置…</span>
      </div>
    );
  }

  if (!testResult) return null;
  const success = testResult.status === "success";
  return (
    <div
      className={`mb-4 flex items-start gap-2 rounded border px-3 py-2 text-xs ${
        success
          ? "border-success/45 bg-success-bg/60 text-success"
          : "border-error/45 bg-error-bg/60 text-error"
      }`}
    >
      {success ? (
        <CheckCircle2 size={13} className="mt-0.5 shrink-0" />
      ) : (
        <AlertCircle size={13} className="mt-0.5 shrink-0" />
      )}
      <div className="min-w-0 flex-1">
        <div className="font-semibold">
          最近测试{success ? "通过" : "失败"} ·{" "}
          {formatConfigTestTime(testResult.testedAt, { withSeconds: true })}
        </div>
        <div className="mt-0.5 break-words opacity-90">{testResult.message}</div>
      </div>
    </div>
  );
};

interface ConfigEditorActionsProps<C> {
  selectedItem: ConfigItem<C> | null;
  creating: boolean;
  saving: boolean;
  activating: number | null;
  deleting: number | null;
  testing: number | null;
  previewing: boolean;
  canTest: boolean;
  canPreviewTest: boolean;
  activateText: string;
  activeDeleteTitle: string;
  onActivate: (id: number) => void | Promise<void>;
  onDelete: (id: number) => void | Promise<void>;
  onPreviewTest: (id: number) => void | Promise<void>;
  onTest: (id: number) => void | Promise<void>;
}

function ConfigEditorActions<C>({
  selectedItem,
  creating,
  saving,
  activating,
  deleting,
  testing,
  previewing,
  canTest,
  canPreviewTest,
  activateText,
  activeDeleteTitle,
  onActivate,
  onDelete,
  onPreviewTest,
  onTest,
}: ConfigEditorActionsProps<C>) {
  if (creating || !selectedItem) return null;
  const mutationBusy = saving || activating != null || deleting != null;
  const testActionBusy = mutationBusy || testing != null || previewing;
  const readonly = selectedItem.readonly === true;

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {canTest && !readonly && (
        <>
          <button
            type="button"
            onClick={() => void onTest(selectedItem.id)}
            disabled={testActionBusy}
            className="pyqt-btn-secondary !min-h-9 !px-3 text-xs sm:!min-h-[30px]"
          >
            {testing === selectedItem.id ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              <>
                <Zap size={12} className="mr-1" />
                测试
              </>
            )}
          </button>
        </>
      )}
      {canPreviewTest && !readonly && (
        <button
          type="button"
          onClick={() => void onPreviewTest(selectedItem.id)}
          disabled={testActionBusy}
          className="pyqt-btn-secondary !min-h-9 !px-3 text-xs sm:!min-h-[30px]"
        >
          {previewing ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <>
              <Eye size={12} className="mr-1" />
              预览测试请求
            </>
          )}
        </button>
      )}
      {!selectedItem.isActive && (
        <button
          type="button"
          onClick={() => void onActivate(selectedItem.id)}
          disabled={mutationBusy}
          className="pyqt-btn-secondary !min-h-9 !px-3 text-xs sm:!min-h-[30px]"
        >
          {activating === selectedItem.id ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <>
              <Check size={12} className="mr-1" />
              {activateText}
            </>
          )}
        </button>
      )}
      {!readonly ? (
        <button
          type="button"
          onClick={() => void onDelete(selectedItem.id)}
          disabled={mutationBusy || selectedItem.isActive || selectedItem.deleteDisabled}
          title={
            selectedItem.deleteDisabled
              ? selectedItem.deleteDisabledReason ?? "该配置不可删除"
              : selectedItem.isActive
                ? activeDeleteTitle
                : ""
          }
          className="pyqt-btn-secondary !min-h-9 !px-3 text-xs !border-error/50 !text-error hover:!bg-error-bg sm:!min-h-[30px]"
        >
          {deleting === selectedItem.id ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <>
              <Trash2 size={12} className="mr-1" />
              删除
            </>
          )}
        </button>
      ) : null}
    </div>
  );
}
