import React, { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown,
  ChevronRight,
  RefreshCw,
  Save, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  getLlmMaxTokensConfig,
  updateLlmMaxTokensConfig,
  type LlmMaxTokensConfig,
} from "../../../api/config";
import {
  TOKEN_GROUPS,
  maxTokensSummary,
  validateMaxTokensConfig,
} from "./maxTokensUtils";
import { LLM_MAX_TOKEN_LIMIT } from "../../../api/generated/config";

interface LlmMaxTokensPanelProps {
  modelMaxOutputTokens: number | null;
  getConfig?: () => Promise<LlmMaxTokensConfig>;
  updateConfig?: (input: LlmMaxTokensConfig) => Promise<LlmMaxTokensConfig>;
}

export const LlmMaxTokensPanel: React.FC<LlmMaxTokensPanelProps> = ({
  modelMaxOutputTokens,
  getConfig = getLlmMaxTokensConfig,
  updateConfig = updateLlmMaxTokensConfig,
}) => {
  const addToast = useToast((state) => state.addToast);
  const [form, setForm] = useState<LlmMaxTokensConfig | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const mountedRef = useRef(true);
  const dirtyRef = useRef(false);
  const cleanFormRef = useRef<LlmMaxTokensConfig | null>(null);
  const editVersionRef = useRef(0);
  const loadSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const loadingRef = useRef(false);
  const savingRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      saveSeqRef.current += 1;
      loadingRef.current = false;
      savingRef.current = false;
    };
  }, []);

  const markClean = () => {
    dirtyRef.current = false;
    setDirty(false);
  };

  const load = useCallback(async (discardDraft = false) => {
    if (loadingRef.current || savingRef.current) return;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    if (discardDraft) {
      editVersionRef.current += 1;
      markClean();
    }
    loadingRef.current = true;
    setLoading(true);
    try {
      const nextForm = await getConfig();
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      if (dirtyRef.current) {
        addToast("已保留未保存的场景输出预算草稿，刷新结果未覆盖当前编辑", "info");
        return;
      }
      setForm(nextForm);
      cleanFormRef.current = nextForm;
      markClean();
    } catch (err) {
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载场景输出预算失败"), "error");
    } finally {
      if (loadSeqRef.current === requestSeq) loadingRef.current = false;
      if (mountedRef.current && loadSeqRef.current === requestSeq) setLoading(false);
    }
  }, [addToast, getConfig]);

  useEffect(() => {
    void load();
  }, [load]);

  const handleRefresh = async () => {
    if (loadingRef.current || savingRef.current) return;
    loadingRef.current = true;
    const shouldDiscardDraft = dirtyRef.current;
    if (shouldDiscardDraft) {
      const ok = await confirmDialog({
        title: "未保存修改",
        message: "当前场景输出预算有未保存修改，刷新会放弃当前草稿。是否继续？",
        confirmText: "放弃草稿",
        dialogType: "warning",
      });
      if (!ok) {
        loadingRef.current = false;
        return;
      }
    }
    if (!mountedRef.current) {
      loadingRef.current = false;
      return;
    }
    loadingRef.current = false;
    await load(shouldDiscardDraft);
  };

  const handleSave = async () => {
    if (savingRef.current || loadingRef.current) return;
    if (!form) return;
    if (!dirtyRef.current) return;
    const err = validateMaxTokensConfig(form, modelMaxOutputTokens);
    if (err) {
      addToast(err, "error");
      return;
    }
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    const requestForm = form;
    const requestEditVersion = editVersionRef.current;
    savingRef.current = true;
    setSaving(true);
    try {
      const savedForm = await updateConfig(requestForm);
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      if (editVersionRef.current !== requestEditVersion) {
        addToast("已保存提交时的场景输出预算，当前保留后续编辑草稿", "info");
        return;
      }
      setForm(savedForm);
      cleanFormRef.current = savedForm;
      markClean();
      addToast("场景输出预算已保存", "success");
    } catch (saveErr) {
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(saveErr, "保存场景输出预算失败"), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) savingRef.current = false;
      if (mountedRef.current && saveSeqRef.current === requestSeq) setSaving(false);
    }
  };

  const validationError = form
    ? validateMaxTokensConfig(form, modelMaxOutputTokens)
    : null;

  return (
    <section aria-label="场景输出预算" className="a4n-flat-surface flex-none overflow-hidden">
      <div className="flex min-w-0 items-center gap-2 px-3 py-2.5 max-sm:flex-wrap">
        <button
          type="button"
          onClick={() => setExpanded((value) => !value)}
          aria-expanded={expanded}
          className="a4n-touch-target flex min-w-0 flex-1 items-center gap-2 text-left max-sm:basis-full"
        >
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span className="min-w-0 flex-1">
            <span className="block text-xs font-semibold text-book-text-main">
              场景输出预算
            </span>
            <span className="block truncate text-small text-book-text-muted">
              {maxTokensSummary(form, loading, modelMaxOutputTokens)}
            </span>
          </span>
        </button>
        {expanded ? (
          <div className="ml-auto flex shrink-0 items-center gap-2">
            <Dropdown
              items={[
                {
                  label: "重新载入配置",
                  icon: <RefreshCw size={13} />,
                  disabled: loading || saving,
                  onClick: () => void handleRefresh(),
                },
              ]}
            />
            <BookButton
              size="sm"
              onClick={handleSave}
              disabled={!form || !dirty || validationError != null || saving || loading}
            >
              {saving ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 保存中…
                </span>
              ) : (
                <>
                  <Save size={13} /> 保存
                </>
              )}
            </BookButton>
          </div>
        ) : null}
      </div>

      {!expanded ? null : !form ? (
        <div className="border-t border-book-border/35 px-3 py-6 text-center text-xs text-book-text-muted">
          {loading ? "正在加载场景输出预算" : "暂无场景输出预算配置"}
        </div>
      ) : (
        <div className="space-y-4 border-t border-book-border/35 p-3">
          <div className="text-small leading-relaxed text-book-text-muted">
            每个场景的输出上限不能超过默认模型的最大输出
            {modelMaxOutputTokens ? ` ${modelMaxOutputTokens}` : ""}。
          </div>
          {TOKEN_GROUPS.map((group) => (
            <div key={group.title}>
              <div className="text-xs font-medium text-book-text-sub mb-2">{group.title}</div>
              <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3">
                {group.fields.map((field) => (
                  <label key={field.key} className="block">
                    <span className="block text-small text-book-text-muted mb-1">
                      {field.label}
                    </span>
                    <BookInput
                      type="number"
                      min={1}
                      max={
                        modelMaxOutputTokens && modelMaxOutputTokens > 0
                          ? modelMaxOutputTokens
                          : LLM_MAX_TOKEN_LIMIT
                      }
                      step={1}
                      value={String(form[field.key])}
                      onChange={(event) => {
                        const nextValue = Number(event.target.value);
                        editVersionRef.current += 1;
                        setForm((current) => {
                          if (!current) return current;
                          const nextForm = {
                            ...current,
                            [field.key]: nextValue,
                          };
                          const nextDirty =
                            JSON.stringify(nextForm) !== JSON.stringify(cleanFormRef.current);
                          dirtyRef.current = nextDirty;
                          setDirty(nextDirty);
                          return nextForm;
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
            </div>
          ))}
          {dirty && validationError ? (
            <div role="status" className="text-small leading-relaxed text-warning">
              {validationError}
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
};
