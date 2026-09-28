import type { ReactNode } from "react";
import { AlertCircle, Check, CheckCircle2, Edit3, Eye, Trash2, X, Zap } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { formatConfigTestTime } from "./testResultUtils";
import type { ConfigEditorRenderContext, ConfigItem, ConfigTestResult } from "./types";
import { useI18n } from "../../../i18n";

interface ConfigEditorPanelProps<C> {
  title: string;
  itemCount: number;
  creating: boolean;
  selectedItem: ConfigItem<C> | null;
  form: C;
  dirty: boolean;
  validationError: string | null;
  busy: boolean;
  saving: boolean;
  activating: number | null;
  deleting: number | null;
  testing: number | null;
  previewing: boolean;
  testResult: ConfigTestResult | null;
  canTest: boolean;
  canPreviewTest: boolean;
  activateText: string;
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
  itemCount,
  creating,
  selectedItem,
  form,
  dirty,
  validationError,
  busy,
  saving,
  activating,
  deleting,
  testing,
  previewing,
  testResult,
  canTest,
  canPreviewTest,
  activateText,
  renderEditor,
  onFormChange,
  onCancel,
  onSave,
  onActivate,
  onDelete,
  onPreviewTest,
  onTest,
}: ConfigEditorPanelProps<C>) {
  const { t } = useI18n();
  const itemActionBusy =
    busy || activating != null || deleting != null || testing != null || previewing;
  const empty = !creating && !selectedItem;
  const createLabel = t("settings.createConfig", { name: title });

  return (
    <section
      className={`a4n-config-editor a4n-flat-surface flex min-h-0 min-w-0 flex-col overflow-visible p-3 sm:p-4 md:overflow-hidden ${
        empty ? "min-h-[180px] self-start md:w-full" : ""
      }`}
    >
      {empty ? (
        busy && itemCount === 0 ? (
          <div role="status" className="a4n-empty-state min-h-[148px] gap-2 text-sm text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("common.loading")}
          </div>
        ) : (
          <div className="a4n-empty-state min-h-[148px]">
            <Edit3 size={24} className="text-book-primary/30" />
            <p className="text-sm">
              {itemCount === 0 ? (
                <>
                  {t("settings.noConfig", { name: title })}
                  <br />
                  {t("settings.clickCreateToStart")}
                </>
              ) : (
                <>
                  {t("settings.selectConfigToEdit", { action: createLabel })}
                  <br />
                </>
              )}
            </p>
          </div>
        )
      ) : (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="mb-4 flex min-w-0 flex-wrap items-start gap-3 border-b border-book-border/60 pb-3">
            <div className="min-w-0 flex-1 basis-40">
              <h3 className="text-sm font-semibold text-book-text-main">
                {creating ? t("settings.newConfig", { name: title }) : t("settings.editConfig", { name: title })}
              </h3>
              {selectedItem ? (
                <div className={`mt-0.5 text-small ${selectedItem.isActive ? "text-success" : "text-book-text-muted"}`}>
                  {selectedItem.isActive ? t("settings.activeConfig") : t("settings.inactiveConfig")}
                </div>
              ) : null}
            </div>
            {creating ? (
              <BookIconButton
                label={t("settings.cancelCreate")}
                variant="ghost"
                size="sm"
                onClick={() => void onCancel()}
                disabled={saving || itemActionBusy}
              >
                <X size={13} />
              </BookIconButton>
            ) : (
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
                onActivate={onActivate}
                onDelete={onDelete}
                onPreviewTest={onPreviewTest}
                onTest={onTest}
              />
            )}
          </div>

          {canTest && !creating ? (
            <ConnectionTestStatus
              selectedItem={selectedItem}
              testing={testing}
              testResult={testResult}
            />
          ) : null}

          {selectedItem?.readonly ? (
            <div className="a4n-context-bar mb-4 text-xs text-book-text-muted">
              {selectedItem.readonlyReason ?? t("settings.configNotEditable")}
            </div>
          ) : null}

          <div className="custom-scrollbar min-h-0 flex-1 overflow-visible md:overflow-y-auto md:pr-1">
            <div className="flex flex-col gap-4 pb-2">
              {renderEditor(
                form,
                selectedItem?.readonly ? () => undefined : onFormChange,
                {
                  selectedId: selectedItem?.id ?? null,
                  readonly: selectedItem?.readonly === true,
                },
              )}
            </div>
          </div>

          {!selectedItem?.readonly ? (
            <div className="mt-3 flex shrink-0 flex-col gap-2 border-t border-book-border/60 pt-3 sm:flex-row sm:items-center">
              {dirty && validationError ? (
                <div role="status" className="min-w-0 flex-1 text-small leading-relaxed text-warning">
                  {validationError}
                </div>
              ) : (
                <span className="flex-1" />
              )}
              <BookButton
                onClick={() => void onSave()}
                disabled={!dirty || validationError != null || saving || itemActionBusy}
                className="w-full sm:w-auto"
              >
                {saving ? (
                  <span className="inline-flex items-center gap-2">
                    <LoadingSpinner size="sm" /> {t("settings.saving")}
                  </span>
                ) : creating ? (
                  t("settings.create")
                ) : (
                  t("common.save")
                )}
              </BookButton>
            </div>
          ) : null}
        </div>
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
  const { locale, t } = useI18n();
  const running = selectedItem != null && testing === selectedItem.id;
  if (!running && !testResult) return null;

  if (running) {
    return (
      <div className="mb-4 flex items-center gap-2 rounded border border-book-primary/45 bg-book-primary/5 px-3 py-2 text-xs text-book-primary" role="status" aria-live="polite">
        <LoadingSpinner size="sm" />
        <span>{t("settings.testingSavedConfig")}</span>
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
      role={success ? "status" : "alert"}
    >
      {success ? (
        <CheckCircle2 size={13} className="mt-0.5 shrink-0" />
      ) : (
        <AlertCircle size={13} className="mt-0.5 shrink-0" />
      )}
      <div className="min-w-0 flex-1">
        <div className="font-semibold">
          {t("settings.latestTest", { status: success ? t("common.passed") : t("workflow.failed") })} ·{" "}
          {formatConfigTestTime(testResult.testedAt, { withSeconds: true }, locale)}
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
  onActivate,
  onDelete,
  onPreviewTest,
  onTest,
}: ConfigEditorActionsProps<C>) {
  const { t } = useI18n();
  if (creating || !selectedItem) return null;
  const mutationBusy = saving || activating != null || deleting != null;
  const actionBusy = mutationBusy || testing != null || previewing;
  const readonly = selectedItem.readonly === true;
  const moreItems: DropdownItem[] = [];

  if (canPreviewTest && !readonly && !actionBusy) {
    moreItems.push({
      label: t("settings.viewModelRequest"),
      icon: <Eye size={12} />,
      onClick: () => void onPreviewTest(selectedItem.id),
    });
  }
  if (!readonly && !selectedItem.isActive && !selectedItem.deleteDisabled && !mutationBusy) {
    if (moreItems.length > 0) moreItems.push({ type: "divider" });
    moreItems.push({
      label: t("settings.deleteConfig"),
      icon: <Trash2 size={12} />,
      danger: true,
      onClick: () => void onDelete(selectedItem.id),
    });
  }

  return (
    <div className="flex min-h-8 flex-wrap items-center gap-2">
      {canTest && !readonly ? (
        <BookButton variant="secondary" size="sm" onClick={() => void onTest(selectedItem.id)} disabled={actionBusy}>
          <Zap size={12} />{t("settings.testSavedConnection")}
        </BookButton>
      ) : null}
      {!readonly && !selectedItem.isActive ? (
        <BookButton variant="secondary" size="sm" onClick={() => void onActivate(selectedItem.id)} disabled={mutationBusy}>
          <Check size={12} />{activateText}
        </BookButton>
      ) : null}
      {testing === selectedItem.id || previewing || activating === selectedItem.id ? (
        <LoadingSpinner size="sm" className="text-book-text-muted" />
      ) : deleting === selectedItem.id ? (
        <LoadingSpinner size="sm" className="text-error" />
      ) : moreItems.length > 0 ? (
        <Dropdown items={moreItems} />
      ) : null}
    </div>
  );
}
