import React, { useEffect, useRef, useState } from "react";
import { CheckCircle2, Network as NetworkIcon, Play } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  promptRender,
  promptRenderAb,
  promptValidateVariables,
  type PromptEntry,
  type PromptRenderAbResult,
} from "../../../api/prompt";
import { PromptProjectSelect } from "./PromptProjectSelect";
import { useI18n } from "../../../i18n";

export const PromptAdvancedPanels: React.FC<{
  prompt: PromptEntry;
  reverseDependencies: string[];
}> = ({ prompt, reverseDependencies }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const mountedRef = useRef(true);
  const scopeRef = useRef({ name: prompt.name, token: 0 });
  const validateSeqRef = useRef(0);
  const validateInputVersionRef = useRef(0);
  const directRenderSeqRef = useRef(0);
  const abRenderSeqRef = useRef(0);
  const renderInputVersionRef = useRef(0);
  const validateBusyRef = useRef(false);
  const directRenderBusyRef = useRef(false);
  const abRenderBusyRef = useRef(false);

  // 变量校验
  const [varsInput, setVarsInput] = useState<string>("");
  const [validateBusy, setValidateBusy] = useState(false);
  const [missingVars, setMissingVars] = useState<string[] | null>(null);

  // 当前/原始模板对照渲染
  const [abVarsJson, setAbVarsJson] = useState<string>("{}");
  const [abProjectId, setAbProjectId] = useState<number | null>(null);
  const [abBusy, setAbBusy] = useState(false);
  const [abResult, setAbResult] = useState<PromptRenderAbResult | null>(null);

  // 直接渲染
  const [renderBusy, setRenderBusy] = useState(false);
  const [renderResult, setRenderResult] = useState<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      validateSeqRef.current += 1;
      directRenderSeqRef.current += 1;
      abRenderSeqRef.current += 1;
      validateBusyRef.current = false;
      directRenderBusyRef.current = false;
      abRenderBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    const token = scopeRef.current.token + 1;
    scopeRef.current = { name: prompt.name, token };
    validateSeqRef.current += 1;
    validateInputVersionRef.current += 1;
    directRenderSeqRef.current += 1;
    abRenderSeqRef.current += 1;
    renderInputVersionRef.current += 1;
    validateBusyRef.current = false;
    directRenderBusyRef.current = false;
    abRenderBusyRef.current = false;
    setValidateBusy(false);
    setMissingVars(null);
    setAbBusy(false);
    setAbResult(null);
    setRenderBusy(false);
    setRenderResult(null);
  }, [prompt.name, prompt.version]);

  const isCurrentScope = (target: { name: string; token: number }) =>
    mountedRef.current &&
    scopeRef.current.name === target.name &&
    scopeRef.current.token === target.token;

  const handleVarsInputChange = (value: string) => {
    validateInputVersionRef.current += 1;
    validateSeqRef.current += 1;
    validateBusyRef.current = false;
    setVarsInput(value);
    setValidateBusy(false);
    setMissingVars(null);
  };

  const resetRenderResults = () => {
    renderInputVersionRef.current += 1;
    directRenderSeqRef.current += 1;
    abRenderSeqRef.current += 1;
    directRenderBusyRef.current = false;
    abRenderBusyRef.current = false;
    setRenderBusy(false);
    setAbBusy(false);
    setRenderResult(null);
    setAbResult(null);
  };

  const handleRenderProjectChange = (value: number | null) => {
    resetRenderResults();
    setAbProjectId(value);
  };

  const handleRenderVariablesChange = (value: string) => {
    resetRenderResults();
    setAbVarsJson(value);
  };

  const handleValidate = async () => {
    if (validateBusyRef.current) return;
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    const requestInputVersion = validateInputVersionRef.current;
    const requestSeq = validateSeqRef.current + 1;
    validateSeqRef.current = requestSeq;
    const vars = varsInput
      .split(/[,\n]/)
      .map((s) => s.trim())
      .filter(Boolean);
    validateBusyRef.current = true;
    setValidateBusy(true);
    try {
      const miss = await promptValidateVariables({
        template_name: targetScope.name,
        variables: vars,
      });
      if (
        !isCurrentScope(targetScope) ||
        validateSeqRef.current !== requestSeq ||
        validateInputVersionRef.current !== requestInputVersion
      ) {
        return;
      }
      setMissingVars(miss);
      if (miss.length === 0) {
        addToast(t("settingsPrompt.validationPassed"), "success");
      } else {
        addToast(t("settingsPrompt.missingVariables", { count: miss.length }), "error");
      }
    } catch (err) {
      if (
        isCurrentScope(targetScope) &&
        validateSeqRef.current === requestSeq &&
        validateInputVersionRef.current === requestInputVersion
      ) {
        addToast(extractErrorMessage(err, t("settingsPrompt.validationFailed")), "error");
      }
    } finally {
      if (validateSeqRef.current === requestSeq) {
        validateBusyRef.current = false;
      }
      if (
        isCurrentScope(targetScope) &&
        validateSeqRef.current === requestSeq &&
        validateInputVersionRef.current === requestInputVersion
      ) {
        setValidateBusy(false);
      }
    }
  };

  const parseAbVars = (): Record<string, unknown> | null => {
    const s = abVarsJson.trim();
    if (!s) return {};
    try {
      const v = JSON.parse(s);
      if (typeof v !== "object" || v === null || Array.isArray(v)) {
        throw new Error(t("settingsPrompt.jsonObjectRequired"));
      }
      return v as Record<string, unknown>;
    } catch (err) {
      addToast(t("settingsPrompt.jsonParseFailed", { message: extractErrorMessage(err, "") }), "error");
      return null;
    }
  };

  const handleDirectRender = async () => {
    if (directRenderBusyRef.current) return;
    const vars = parseAbVars();
    if (vars === null) return;
    const pid = abProjectId;
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    const requestInputVersion = renderInputVersionRef.current;
    const requestSeq = directRenderSeqRef.current + 1;
    directRenderSeqRef.current = requestSeq;
    directRenderBusyRef.current = true;
    setRenderBusy(true);
    try {
      const r = await promptRender({
        template_name: targetScope.name,
        variables: vars,
        project_id: pid,
      });
      if (
        !isCurrentScope(targetScope) ||
        directRenderSeqRef.current !== requestSeq ||
        renderInputVersionRef.current !== requestInputVersion
      ) {
        return;
      }
      setRenderResult(r);
    } catch (err) {
      if (
        isCurrentScope(targetScope) &&
        directRenderSeqRef.current === requestSeq &&
        renderInputVersionRef.current === requestInputVersion
      ) {
        addToast(extractErrorMessage(err, t("settingsPrompt.renderFailed")), "error");
      }
    } finally {
      if (directRenderSeqRef.current === requestSeq) {
        directRenderBusyRef.current = false;
      }
      if (
        isCurrentScope(targetScope) &&
        directRenderSeqRef.current === requestSeq &&
        renderInputVersionRef.current === requestInputVersion
      ) {
        setRenderBusy(false);
      }
    }
  };

  const handleAbRender = async () => {
    if (abRenderBusyRef.current) return;
    const vars = parseAbVars();
    if (vars === null) return;
    const pid = abProjectId;
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    const requestInputVersion = renderInputVersionRef.current;
    const requestSeq = abRenderSeqRef.current + 1;
    abRenderSeqRef.current = requestSeq;
    abRenderBusyRef.current = true;
    setAbBusy(true);
    try {
      const r = await promptRenderAb({
        template_name: targetScope.name,
        variables: vars,
        project_id: pid,
      });
      if (
        !isCurrentScope(targetScope) ||
        abRenderSeqRef.current !== requestSeq ||
        renderInputVersionRef.current !== requestInputVersion
      ) {
        return;
      }
      setAbResult(r);
    } catch (err) {
      if (
        isCurrentScope(targetScope) &&
        abRenderSeqRef.current === requestSeq &&
        renderInputVersionRef.current === requestInputVersion
      ) {
        addToast(extractErrorMessage(err, t("settingsPrompt.compareRenderFailed")), "error");
      }
    } finally {
      if (abRenderSeqRef.current === requestSeq) {
        abRenderBusyRef.current = false;
      }
      if (
        isCurrentScope(targetScope) &&
        abRenderSeqRef.current === requestSeq &&
        renderInputVersionRef.current === requestInputVersion
      ) {
        setAbBusy(false);
      }
    }
  };

  return (
    <>
      <p className="py-2 text-small leading-relaxed text-book-text-muted">{t("settingsPrompt.savedTemplateNotice")}</p>
      <details className="py-2.5">
        <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
          <NetworkIcon size={11} /> {t("settingsPrompt.dependenciesPanel")}
        </summary>
        <div className="mt-2 text-small">
          <div className="grid grid-cols-2 gap-2">
            <div>
              <div className="text-book-text-muted mb-1">{t("settingsPrompt.dependenciesForward")}</div>
              {prompt.dependencies.length === 0 ? (
                <div className="italic text-book-text-muted">{t("common.none")}</div>
              ) : (
                <div className="break-words font-mono text-book-text-sub">
                  {prompt.dependencies.join("、")}
                </div>
              )}
            </div>
            <div>
              <div className="text-book-text-muted mb-1">{t("settingsPrompt.dependenciesReverse")}</div>
              {reverseDependencies.length === 0 ? (
                <div className="italic text-book-text-muted">{t("common.none")}</div>
              ) : (
                <div className="break-words font-mono text-book-text-sub">
                  {reverseDependencies.join("、")}
                </div>
              )}
            </div>
          </div>
        </div>
      </details>

      <details className="py-2.5">
        <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
          <CheckCircle2 size={11} /> {t("settingsPrompt.variableValidation")}
        </summary>
        <div className="mt-2 text-small space-y-1.5">
          <div className="text-book-text-muted">
            {t("settingsPrompt.variableValidationHint")}
          </div>
          <textarea
            aria-label={t("settingsPrompt.variableValidation")}
            value={varsInput}
            onChange={(e) => handleVarsInputChange(e.target.value)}
            className="pyqt-input w-full text-small h-16 resize-y font-mono"
            placeholder={t("settingsPrompt.variablePlaceholder")}
          />
          <div className="flex items-center gap-2">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={handleValidate}
              disabled={validateBusy}
            >
              {validateBusy ? <LoadingSpinner size="sm" /> : t("settingsPrompt.validate")}
            </BookButton>
            {missingVars !== null && missingVars.length === 0 && (
              <span className="inline-flex items-center gap-1.5 text-success"><CheckCircle2 size={14} aria-hidden="true" />{t("settingsPrompt.allVariablesCovered")}</span>
            )}
          </div>
          {missingVars !== null && missingVars.length > 0 && (
            <div className="break-words border-l border-error/40 pl-2.5 pt-1 text-error">
              {t("settingsPrompt.missingLabel")}: <span className="font-mono">{missingVars.join("、")}</span>
            </div>
          )}
        </div>
      </details>

      <details className="py-2.5">
        <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
          <Play size={11} /> {t("settingsPrompt.renderComparison")}
        </summary>
        <div className="mt-2 text-small space-y-1.5">
          <label className="grid gap-1">
            <span className="text-book-text-muted">{t("nav.projectContext")}</span>
            <PromptProjectSelect
              value={abProjectId}
              onChange={handleRenderProjectChange}
              allowEmpty
              ariaLabel={t("settingsPrompt.renderProjectContext")}
              className="w-full"
            />
          </label>
          <div className="text-book-text-muted">{t("settingsPrompt.variableJson")}</div>
          <textarea
            aria-label={t("settingsPrompt.variableJson")}
            value={abVarsJson}
            onChange={(e) => handleRenderVariablesChange(e.target.value)}
            className="pyqt-input w-full text-small h-20 resize-y font-mono"
            placeholder={t("settingsPrompt.variableJsonPlaceholder")}
          />
          <div className="flex items-center gap-2 flex-wrap">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={handleDirectRender}
              disabled={renderBusy}
            >
              {renderBusy ? <LoadingSpinner size="sm" /> : t("settingsPrompt.renderDirect")}
            </BookButton>
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleAbRender}
              disabled={abBusy}
            >
              {abBusy ? <LoadingSpinner size="sm" /> : t("settingsPrompt.renderCurrentOriginal")}
            </BookButton>
          </div>
          {renderResult !== null && (
            <div>
              <div className="text-book-text-muted">{t("settingsPrompt.directRenderResult")}</div>
              <pre className="mt-1 whitespace-pre-wrap break-words border-l border-book-border/45 pl-2.5 font-mono text-caption text-book-text-sub">
                {renderResult || t("settingsPrompt.emptyResult")}
              </pre>
            </div>
          )}
          {abResult !== null && (
            <div>
              <div className="text-book-text-muted">{t("settingsPrompt.currentTemplate")}</div>
              <pre className="mt-1 whitespace-pre-wrap break-words border-l border-book-border/45 pl-2.5 font-mono text-caption text-book-text-sub">
                {abResult.current || t("settingsPrompt.emptyResult")}
              </pre>
              <div className="text-book-text-muted mt-1.5">{t("settingsPrompt.originalTemplate")}</div>
              <pre className="mt-1 whitespace-pre-wrap break-words border-l border-book-border/45 pl-2.5 font-mono text-caption text-book-text-sub">
                {abResult.original || t("settingsPrompt.emptyResult")}
              </pre>
            </div>
          )}
        </div>
      </details>
    </>
  );
};
