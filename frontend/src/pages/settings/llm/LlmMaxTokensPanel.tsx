import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Loader2,
  RefreshCw,
  Save,
  SlidersHorizontal,
} from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  getLlmMaxTokensConfig,
  updateLlmMaxTokensConfig,
  type LlmMaxTokensConfig,
} from "../../../api/config";
import {
  MAX_TOKEN_LIMIT,
  TOKEN_GROUPS,
  maxTokensSummary,
  validateMaxTokensConfig,
} from "./maxTokensUtils";

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
  const mountedRef = useRef(true);
  const dirtyRef = useRef(false);
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
  };

  const markDirty = () => {
    editVersionRef.current += 1;
    dirtyRef.current = true;
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

  return (
    <section className="pyqt-card flex-none p-3">
      <div className={`flex flex-wrap items-center justify-between gap-3 ${expanded ? "mb-3" : ""}`}>
        <button
          type="button"
          onClick={() => setExpanded((value) => !value)}
          className="min-w-0 flex flex-1 items-center gap-2 text-left"
        >
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
          <SlidersHorizontal size={14} className="text-book-primary" />
          <span className="min-w-0">
            <span className="block text-sm font-serif text-book-text-main">
              场景请求输出预算
            </span>
            <span className="block truncate text-[11px] text-book-text-muted">
              {maxTokensSummary(form, loading, modelMaxOutputTokens)}
            </span>
          </span>
        </button>
        {expanded && (
          <div className="inline-flex items-center gap-2">
            <BookButton variant="secondary" size="sm" onClick={handleRefresh} disabled={loading || saving}>
              {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
              刷新
            </BookButton>
            <BookButton variant="primary" size="sm" onClick={handleSave} disabled={!form || saving}>
              {saving ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} />}
              保存
            </BookButton>
          </div>
        )}
      </div>

      {!expanded ? null : !form ? (
        <div className="py-6 text-center text-xs text-book-text-muted">
          {loading ? "正在加载场景输出预算" : "暂无场景输出预算配置"}
        </div>
      ) : (
        <div className="max-h-[38vh] space-y-4 overflow-y-auto pr-1 custom-scrollbar">
          <div className="rounded border border-book-border bg-book-bg-soft px-3 py-2 text-xs text-book-text-muted">
            每个场景的请求 max_tokens 必须小于或等于默认模型最大输出
            {modelMaxOutputTokens ? ` ${modelMaxOutputTokens}` : ""}。
          </div>
          {TOKEN_GROUPS.map((group) => (
            <div key={group.title}>
              <div className="text-xs font-medium text-book-text-sub mb-2">{group.title}</div>
              <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3">
                {group.fields.map((field) => (
                  <label key={field.key} className="block">
                    <span className="block text-[11px] text-book-text-muted mb-1">
                      {field.label}
                    </span>
                    <BookInput
                      type="number"
                      min={1}
                      max={
                        modelMaxOutputTokens && modelMaxOutputTokens > 0
                          ? modelMaxOutputTokens
                          : MAX_TOKEN_LIMIT
                      }
                      step={1}
                      value={String(form[field.key])}
                      onChange={(event) => {
                        const nextValue = Number(event.target.value);
                        markDirty();
                        setForm((current) => {
                          if (!current) return current;
                          return {
                            ...current,
                            [field.key]: nextValue,
                          };
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
};
