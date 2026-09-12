import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { Modal } from "../../components/ui/Modal";
import { ConfigEditorPanel } from "./config-list/ConfigEditorPanel";
import { ConfigListSidebar } from "./config-list/ConfigListSidebar";
import { ConfigListToolbar } from "./config-list/ConfigListToolbar";
import { FormField } from "./config-list/FormField";
import type { ConfigListPanelProps, ConfigTestResult } from "./config-list/types";
import { useI18n } from "../../i18n";

export type { ConfigItem } from "./config-list/types";
export { FormField };

/**
 * 通用配置列表面板
 *
 * 左侧负责配置选择，右侧负责当前项编辑；业务读写由调用方注入。
 */
export function ConfigListPanel<C>({
  title,
  items,
  loading,
  onRefresh,
  onCreate,
  onUpdate,
  onDelete,
  onActivate,
  onTest,
  onPreviewTest,
  formatItem,
  renderEditor,
  defaultForm,
  validate,
  activeLabel,
  activateText,
  activateSuccessMessage,
  activateErrorMessage,
}: ConfigListPanelProps<C>) {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const resolvedActiveLabel = activeLabel ?? t("settings.currentDefault");
  const resolvedActivateText = activateText ?? t("settings.setDefault");
  const resolvedActivateSuccessMessage = activateSuccessMessage ?? t("settings.setDefaultSuccess");
  const resolvedActivateErrorMessage = activateErrorMessage ?? t("settings.setDefaultFailed");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [form, setForm] = useState<C>(defaultForm);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [activating, setActivating] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<number | null>(null);
  const [testing, setTesting] = useState<number | null>(null);
  const [testResults, setTestResults] = useState<Record<number, ConfigTestResult>>({});
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [mobileEditorOpen, setMobileEditorOpen] = useState(false);
  const [isDesktop, setIsDesktop] = useState(
    () => typeof window === "undefined" || window.matchMedia("(min-width: 768px)").matches,
  );

  const selectedItem = items.find((item) => item.id === selectedId) ?? null;
  const mountedRef = useRef(true);
  const itemsRef = useRef(items);
  const selectedIdRef = useRef(selectedId);
  const creatingRef = useRef(creating);
  const dirtyRef = useRef(dirty);
  const editVersionRef = useRef(0);
  const saveSeqRef = useRef(0);
  const refreshSeqRef = useRef(0);
  const activateSeqRef = useRef(0);
  const deleteSeqRef = useRef(0);
  const testSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const savingRef = useRef(false);
  const refreshingRef = useRef(false);
  const draftTransitionRef = useRef(false);
  const activatingRef = useRef<number | null>(null);
  const deletingRef = useRef<number | null>(null);
  const testingRef = useRef<number | null>(null);
  const previewLoadingRef = useRef(false);

  itemsRef.current = items;
  selectedIdRef.current = selectedId;
  creatingRef.current = creating;
  dirtyRef.current = dirty;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      saveSeqRef.current += 1;
      refreshSeqRef.current += 1;
      activateSeqRef.current += 1;
      deleteSeqRef.current += 1;
      testSeqRef.current += 1;
      previewSeqRef.current += 1;
      savingRef.current = false;
      refreshingRef.current = false;
      draftTransitionRef.current = false;
      activatingRef.current = null;
      deletingRef.current = null;
      testingRef.current = null;
      previewLoadingRef.current = false;
    };
  }, []);

  const hasItem = (id: number) => itemsRef.current.some((item) => item.id === id);
  const isSelectedItem = (id: number) => selectedIdRef.current === id && hasItem(id);
  const editorScope = () => (creatingRef.current ? "create" : `item:${selectedIdRef.current ?? "none"}`);
  const markClean = () => {
    dirtyRef.current = false;
    setDirty(false);
  };

  useLayoutEffect(() => {
    if (creating || dirty || selectedId != null || items.length === 0) return;
    const initialItem = items.find((item) => item.isActive) ?? items[0];
    setSelectedId(initialItem.id);
    setForm(initialItem.config);
    markClean();
  }, [creating, dirty, items, selectedId]);

  useEffect(() => {
    if (creating || dirty || !selectedItem) return;
    setForm(selectedItem.config);
    markClean();
  }, [creating, dirty, selectedItem]);

  useEffect(() => {
    if (selectedId != null && !items.some((item) => item.id === selectedId)) {
      previewSeqRef.current += 1;
      setSelectedId(null);
      setCreating(false);
      setForm(defaultForm);
      setMobileEditorOpen(false);
      markClean();
    }
  }, [defaultForm, items, selectedId]);

  useEffect(() => {
    const media = window.matchMedia("(min-width: 768px)");
    const sync = () => setIsDesktop(media.matches);
    sync();
    media.addEventListener("change", sync);
    return () => media.removeEventListener("change", sync);
  }, []);

  useEffect(() => {
    if (isDesktop || items.length <= 1) setMobileEditorOpen(false);
  }, [isDesktop, items.length]);

  const confirmDiscardDraft = async (actionText: string): Promise<boolean> => {
    if (!dirtyRef.current) return true;
    return confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: title, action: actionText }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
  };

  const resetFormToSavedState = () => {
    if (!creating && selectedItem) {
      setForm(selectedItem.config);
    } else {
      setForm(defaultForm);
    }
    markClean();
  };

  const handleStartCreate = async () => {
    if (savingRef.current || refreshingRef.current || draftTransitionRef.current) return;
    draftTransitionRef.current = true;
    try {
      if (!(await confirmDiscardDraft(t("settings.createConfig", { name: title })))) return;
      if (!mountedRef.current) return;
      setSelectedId(null);
      setCreating(true);
      setForm(defaultForm);
      markClean();
      if (!isDesktop && items.length > 1) setMobileEditorOpen(true);
    } finally {
      draftTransitionRef.current = false;
    }
  };

  const handleSelect = async (id: number) => {
    if (savingRef.current || refreshingRef.current || draftTransitionRef.current) return;
    if (!creating && id === selectedId) {
      if (!isDesktop && items.length > 1) setMobileEditorOpen(true);
      return;
    }
    draftTransitionRef.current = true;
    try {
      if (!(await confirmDiscardDraft(t("settings.switchConfig", { name: title })))) return;
      if (!mountedRef.current || !hasItem(id)) return;
      setSelectedId(id);
      setCreating(false);
      markClean();
      if (!isDesktop && items.length > 1) setMobileEditorOpen(true);
    } finally {
      draftTransitionRef.current = false;
    }
  };

  const handleRefresh = async () => {
    if (savingRef.current || refreshingRef.current || draftTransitionRef.current) return;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    refreshingRef.current = true;
    draftTransitionRef.current = true;
    try {
      if (!(await confirmDiscardDraft(t("settings.refreshList")))) return;
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      if (dirtyRef.current) {
        resetFormToSavedState();
      }
      await onRefresh();
    } finally {
      if (refreshSeqRef.current === requestSeq) {
        refreshingRef.current = false;
        draftTransitionRef.current = false;
      }
    }
  };

  const handleSave = async () => {
    if (
      savingRef.current ||
      refreshingRef.current ||
      activatingRef.current != null ||
      deletingRef.current != null ||
      testingRef.current != null ||
      previewLoadingRef.current
    ) {
      return;
    }
    if (!dirtyRef.current) return;
    const error = validate?.(form);
    if (error) {
      addToast(error, "error");
      return;
    }

    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    const requestCreating = creating;
    const requestSelectedItem = selectedItem;
    const requestScope = editorScope();
    const requestForm = form;
    const requestEditVersion = editVersionRef.current;
    savingRef.current = true;
    setSaving(true);
    try {
      if (requestCreating) {
        await onCreate(requestForm);
        if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
        if (editorScope() === requestScope && editVersionRef.current === requestEditVersion) {
          addToast(t("settings.configCreated", { name: title }), "success");
          setCreating(false);
          setMobileEditorOpen(false);
          markClean();
        } else if (editorScope() === requestScope) {
          addToast(t("settings.configCreatedWithDraft", { name: title }), "success");
        }
      } else if (requestSelectedItem) {
        await onUpdate(requestSelectedItem.id, requestForm);
        if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
        setTestResults((prev) => {
          const next = { ...prev };
          delete next[requestSelectedItem.id];
          return next;
        });
        if (editorScope() === requestScope && editVersionRef.current === requestEditVersion) {
          addToast(t("settings.configUpdated", { name: title }), "success");
          markClean();
        } else if (editorScope() === requestScope) {
          addToast(t("settings.configUpdatedWithDraft", { name: title }), "success");
        }
      }
      await onRefresh();
    } catch (err) {
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settings.saveFailed")), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) savingRef.current = false;
      if (mountedRef.current && saveSeqRef.current === requestSeq) setSaving(false);
    }
  };

  const validationError = validate?.(form) ?? null;
  const itemActionBusy =
    saving ||
    activating != null ||
    deleting != null ||
    testing != null ||
    requestPreviewLoading;

  const handleCancel = async () => {
    if (savingRef.current || refreshingRef.current || draftTransitionRef.current) return;
    draftTransitionRef.current = true;
    try {
      if (!(await confirmDiscardDraft(t("settings.cancelEdit")))) return;
      if (!mountedRef.current) return;
      setCreating(false);
      setSelectedId(null);
      setForm(defaultForm);
      setMobileEditorOpen(false);
      markClean();
    } finally {
      draftTransitionRef.current = false;
    }
  };

  const handleCloseMobileEditor = async () => {
    if (savingRef.current || refreshingRef.current || draftTransitionRef.current) return;
    draftTransitionRef.current = true;
    try {
      if (!(await confirmDiscardDraft(t("settings.cancelEdit")))) return;
      if (!mountedRef.current) return;
      if (creatingRef.current) {
        setCreating(false);
        setSelectedId(null);
        setForm(defaultForm);
        markClean();
      } else if (dirtyRef.current) {
        resetFormToSavedState();
      }
      setMobileEditorOpen(false);
    } finally {
      draftTransitionRef.current = false;
    }
  };

  const handleActivate = async (id: number) => {
    if (
      savingRef.current ||
      refreshingRef.current ||
      activatingRef.current != null ||
      deletingRef.current != null
    ) return;
    if (!hasItem(id)) return;
    const requestSeq = activateSeqRef.current + 1;
    activateSeqRef.current = requestSeq;
    activatingRef.current = id;
    setActivating(id);
    try {
      await onActivate(id);
      if (!mountedRef.current || activateSeqRef.current !== requestSeq || !hasItem(id)) return;
      addToast(resolvedActivateSuccessMessage, "success");
      await onRefresh();
    } catch (err) {
      if (!mountedRef.current || activateSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, resolvedActivateErrorMessage), "error");
    } finally {
      if (activateSeqRef.current === requestSeq) activatingRef.current = null;
      if (mountedRef.current && activateSeqRef.current === requestSeq) setActivating(null);
    }
  };

  const handleDelete = async (id: number) => {
    if (
      savingRef.current ||
      refreshingRef.current ||
      activatingRef.current != null ||
      deletingRef.current != null
    ) return;
    if (!hasItem(id)) return;
    const item = itemsRef.current.find((row) => row.id === id);
    if (item?.deleteDisabled) {
      addToast(item.deleteDisabledReason ?? t("settings.configCannotDelete"), "error");
      return;
    }
    deletingRef.current = id;
    if (dirtyRef.current && selectedIdRef.current === id) {
      const discard = await confirmDiscardDraft(t("settings.deleteConfig"));
      if (!discard) {
        if (deletingRef.current === id) deletingRef.current = null;
        return;
      }
    }
    const ok = await confirmDialog({
      title: t("settings.deleteConfigTitle", { name: title }),
      message: t("common.deleteConfirm", { name: title }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      if (deletingRef.current === id) deletingRef.current = null;
      return;
    }
    if (!mountedRef.current || !hasItem(id)) {
      if (deletingRef.current === id) deletingRef.current = null;
      return;
    }

    const requestSeq = deleteSeqRef.current + 1;
    deleteSeqRef.current = requestSeq;
    setDeleting(id);
    try {
      await onDelete(id);
      if (!mountedRef.current || deleteSeqRef.current !== requestSeq) return;
      addToast(t("settings.configDeleted"), "success");
      if (selectedIdRef.current === id) {
        setSelectedId(null);
        setForm(defaultForm);
        setMobileEditorOpen(false);
        markClean();
      }
      setTestResults((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      await onRefresh();
    } catch (err) {
      if (!mountedRef.current || deleteSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settings.deleteFailed")), "error");
    } finally {
      if (deleteSeqRef.current === requestSeq) deletingRef.current = null;
      if (mountedRef.current && deleteSeqRef.current === requestSeq) setDeleting(null);
    }
  };

  const handleTest = async (id: number) => {
    if (!onTest) return;
    if (
      savingRef.current ||
      refreshingRef.current ||
      testingRef.current != null ||
      previewLoadingRef.current
    ) return;
    if (!isSelectedItem(id)) return;
    if (selectedItem?.readonly) {
      addToast(selectedItem.readonlyReason ?? t("settings.readonlyConfigCannotTest"), "error");
      return;
    }
    if (dirtyRef.current) {
      addToast(t("settings.unsavedConfigBeforeTest"), "error");
      return;
    }
    const requestSeq = testSeqRef.current + 1;
    testSeqRef.current = requestSeq;
    testingRef.current = id;
    setTesting(id);
    try {
      await onTest(id);
      if (!mountedRef.current || testSeqRef.current !== requestSeq || !isSelectedItem(id)) return;
      setTestResults((prev) => ({
        ...prev,
        [id]: {
          status: "success",
          message: t("settings.connectionTestPassed"),
          testedAt: new Date().toISOString(),
        },
      }));
      addToast(t("settings.connectionTestPassed"), "success");
    } catch (err) {
      if (!mountedRef.current || testSeqRef.current !== requestSeq || !isSelectedItem(id)) return;
      const message = extractErrorMessage(err, t("settings.connectionTestFailed"));
      setTestResults((prev) => ({
        ...prev,
        [id]: {
          status: "error",
          message,
          testedAt: new Date().toISOString(),
        },
      }));
      addToast(message, "error");
    } finally {
      if (testSeqRef.current === requestSeq) testingRef.current = null;
      if (mountedRef.current && testSeqRef.current === requestSeq) setTesting(null);
    }
  };

  const handlePreviewTest = async (id: number) => {
    if (!onPreviewTest) return;
    if (
      savingRef.current ||
      refreshingRef.current ||
      testingRef.current != null ||
      previewLoadingRef.current
    ) return;
    if (!isSelectedItem(id)) return;
    if (selectedItem?.readonly) {
      addToast(selectedItem.readonlyReason ?? t("settings.readonlyConfigCannotPreview"), "error");
      return;
    }
    if (dirtyRef.current) {
      addToast(t("settings.unsavedConfigBeforePreview"), "error");
      return;
    }
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    previewLoadingRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await onPreviewTest(id);
      if (
        !mountedRef.current ||
        previewSeqRef.current !== requestSeq ||
        !isSelectedItem(id)
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (
        !mountedRef.current ||
        previewSeqRef.current !== requestSeq ||
        !isSelectedItem(id)
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("settings.previewTestFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (previewSeqRef.current === requestSeq) previewLoadingRef.current = false;
      if (mountedRef.current && previewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
    }
  };

  const mobileEditorMode = !isDesktop && items.length > 1;
  const mobileEditorTitle = creating
    ? t("settings.newConfig", { name: title })
    : selectedItem
      ? formatItem(selectedItem.config, selectedItem.isActive).primary
      : title;
  const editorPanel = (
    <ConfigEditorPanel
      title={title}
      itemCount={items.length}
      creating={creating}
      selectedItem={selectedItem}
      form={form}
      dirty={dirty}
      validationError={validationError}
      busy={loading}
      saving={saving}
      activating={activating}
      deleting={deleting}
      testing={testing}
      previewing={requestPreviewLoading}
      testResult={selectedItem ? testResults[selectedItem.id] ?? null : null}
      canTest={Boolean(onTest)}
      canPreviewTest={Boolean(onPreviewTest)}
      activateText={resolvedActivateText}
      renderEditor={renderEditor}
      onFormChange={(nextForm) => {
        setForm(nextForm);
        editVersionRef.current += 1;
        const baselineForm = creating
          ? defaultForm
          : selectedItem?.config ?? defaultForm;
        const nextDirty = !configFormsEqual(nextForm, baselineForm);
        dirtyRef.current = nextDirty;
        setDirty(nextDirty);
      }}
      onCancel={handleCancel}
      onSave={handleSave}
      onActivate={handleActivate}
      onDelete={handleDelete}
      onPreviewTest={handlePreviewTest}
      onTest={handleTest}
    />
  );

  return (
    <section
      aria-label={t("settings.configManagement", { name: title })}
      data-settings-dirty={dirty ? "true" : undefined}
      className="flex h-auto flex-col gap-3 md:h-full"
    >
      <ConfigListToolbar
        title={title}
        itemCount={items.length}
        loading={loading}
        busy={itemActionBusy}
        onCreate={() => void handleStartCreate()}
        onRefresh={handleRefresh}
      />

      <div
        className={`grid min-h-0 gap-3 md:flex-1 md:grid-rows-1 ${
          items.length > 1 ? "md:grid-cols-[260px_minmax(0,1fr)]" : "md:grid-cols-1"
        }`}
      >
        {items.length > 1 ? (
          <ConfigListSidebar
            title={title}
            items={items}
            loading={loading}
            busy={itemActionBusy || loading}
            selectedId={selectedId}
            creating={creating}
            formatItem={formatItem}
            testResults={testResults}
            activeLabel={resolvedActiveLabel}
            onSelect={(id) => void handleSelect(id)}
          />
        ) : null}

        {!mobileEditorMode ? editorPanel : null}
      </div>
      {mobileEditorMode ? (
        <Modal
          isOpen={mobileEditorOpen}
          onClose={() => void handleCloseMobileEditor()}
          title={mobileEditorTitle}
          maxWidthClassName="max-w-xl"
          bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
          closeOnBackdrop={!itemActionBusy}
          closeOnEscape={!itemActionBusy}
          showCloseButton={!itemActionBusy}
        >
          {editorPanel}
        </Modal>
      ) : null}
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("settings.testRequestPreview", { name: title })}
        onClose={() => {
          previewSeqRef.current += 1;
          previewLoadingRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
          setRequestPreviewData(null);
        }}
      />
    </section>
  );
}

const configFormsEqual = <C,>(left: C, right: C): boolean =>
  JSON.stringify(left) === JSON.stringify(right);
