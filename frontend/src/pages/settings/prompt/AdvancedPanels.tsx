import React, { useEffect, useRef, useState } from "react";
import { CheckCircle2, Loader2, Network as NetworkIcon, Play } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  promptDependencyGraph,
  promptGetDependencies,
  promptGetReverseDependencies,
  promptRender,
  promptRenderAb,
  promptValidateVariables,
  type PromptDependencyGraph,
  type PromptEntry,
  type PromptRenderAbResult,
} from "../../../api/prompt";

export const PromptAdvancedPanels: React.FC<{ prompt: PromptEntry }> = ({ prompt }) => {
  const { addToast } = useToast();
  const mountedRef = useRef(true);
  const scopeRef = useRef({ name: prompt.name, token: 0 });
  const graphSeqRef = useRef(0);
  const validateSeqRef = useRef(0);
  const validateInputVersionRef = useRef(0);
  const directRenderSeqRef = useRef(0);
  const abRenderSeqRef = useRef(0);
  const renderInputVersionRef = useRef(0);
  const graphLoadingRef = useRef(false);
  const validateBusyRef = useRef(false);
  const directRenderBusyRef = useRef(false);
  const abRenderBusyRef = useRef(false);

  // 依赖图
  const [graphLoading, setGraphLoading] = useState(false);
  const [graph, setGraph] = useState<PromptDependencyGraph | null>(null);
  const [deps, setDeps] = useState<string[]>([]);
  const [reverseDeps, setReverseDeps] = useState<string[]>([]);

  // 变量校验
  const [varsInput, setVarsInput] = useState<string>("");
  const [validateBusy, setValidateBusy] = useState(false);
  const [missingVars, setMissingVars] = useState<string[] | null>(null);

  // 当前/原始模板对照渲染
  const [abVarsJson, setAbVarsJson] = useState<string>("{}");
  const [abProjectId, setAbProjectId] = useState<string>("");
  const [abBusy, setAbBusy] = useState(false);
  const [abResult, setAbResult] = useState<PromptRenderAbResult | null>(null);

  // 直接渲染
  const [renderBusy, setRenderBusy] = useState(false);
  const [renderResult, setRenderResult] = useState<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      graphSeqRef.current += 1;
      validateSeqRef.current += 1;
      directRenderSeqRef.current += 1;
      abRenderSeqRef.current += 1;
      graphLoadingRef.current = false;
      validateBusyRef.current = false;
      directRenderBusyRef.current = false;
      abRenderBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    const token = scopeRef.current.token + 1;
    scopeRef.current = { name: prompt.name, token };
    graphSeqRef.current += 1;
    validateSeqRef.current += 1;
    validateInputVersionRef.current += 1;
    directRenderSeqRef.current += 1;
    abRenderSeqRef.current += 1;
    renderInputVersionRef.current += 1;
    graphLoadingRef.current = false;
    validateBusyRef.current = false;
    directRenderBusyRef.current = false;
    abRenderBusyRef.current = false;
    setGraphLoading(false);
    setGraph(null);
    setDeps([]);
    setReverseDeps([]);
    setValidateBusy(false);
    setMissingVars(null);
    setAbBusy(false);
    setAbResult(null);
    setRenderBusy(false);
    setRenderResult(null);
  }, [prompt.name]);

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

  const handleRenderInputChange = (field: "project" | "vars", value: string) => {
    renderInputVersionRef.current += 1;
    directRenderSeqRef.current += 1;
    abRenderSeqRef.current += 1;
    directRenderBusyRef.current = false;
    abRenderBusyRef.current = false;
    if (field === "project") {
      setAbProjectId(value);
    } else {
      setAbVarsJson(value);
    }
    setRenderBusy(false);
    setAbBusy(false);
    setRenderResult(null);
    setAbResult(null);
  };

  const loadGraph = async () => {
    if (graphLoadingRef.current) return;
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    const requestSeq = graphSeqRef.current + 1;
    graphSeqRef.current = requestSeq;
    graphLoadingRef.current = true;
    setGraphLoading(true);
    try {
      const [g, d, rd] = await Promise.all([
        promptDependencyGraph(),
        promptGetDependencies(targetScope.name),
        promptGetReverseDependencies(targetScope.name),
      ]);
      if (!isCurrentScope(targetScope) || graphSeqRef.current !== requestSeq) return;
      setGraph(g);
      setDeps(d);
      setReverseDeps(rd);
    } catch (err) {
      if (isCurrentScope(targetScope) && graphSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "加载依赖图失败"), "error");
      }
    } finally {
      if (graphSeqRef.current === requestSeq) {
        graphLoadingRef.current = false;
      }
      if (isCurrentScope(targetScope) && graphSeqRef.current === requestSeq) {
        setGraphLoading(false);
      }
    }
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
        addToast("校验通过：无缺失变量", "success");
      } else {
        addToast(`缺失 ${miss.length} 个变量`, "error");
      }
    } catch (err) {
      if (
        isCurrentScope(targetScope) &&
        validateSeqRef.current === requestSeq &&
        validateInputVersionRef.current === requestInputVersion
      ) {
        addToast(extractErrorMessage(err, "变量校验失败"), "error");
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
        throw new Error("必须是 JSON 对象");
      }
      return v as Record<string, unknown>;
    } catch (err) {
      addToast(`JSON 解析失败：${extractErrorMessage(err, "")}`, "error");
      return null;
    }
  };

  const parseProjectId = (): number | null | undefined => {
    const value = abProjectId.trim();
    if (!value) return null;
    const projectId = Number(value);
    if (!Number.isInteger(projectId) || projectId <= 0) {
      addToast("project_id 必须是正整数", "error");
      return undefined;
    }
    return projectId;
  };

  const handleDirectRender = async () => {
    if (directRenderBusyRef.current) return;
    const vars = parseAbVars();
    if (vars === null) return;
    const pid = parseProjectId();
    if (pid === undefined) return;
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
        addToast(extractErrorMessage(err, "渲染失败"), "error");
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
    const pid = parseProjectId();
    if (pid === undefined) return;
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
        addToast(extractErrorMessage(err, "对照渲染失败"), "error");
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
      <details className="pyqt-card p-2.5">
        <summary
          className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5"
          onClick={(e) => {
            if (!graph && !graphLoading) {
              e.preventDefault();
              void loadGraph();
              const el = e.currentTarget.parentElement as HTMLDetailsElement | null;
              if (el) el.open = true;
            }
          }}
        >
          <NetworkIcon size={11} /> 依赖图 / 反向依赖
        </summary>
        <div className="mt-2 text-[11px]">
          {graphLoading ? (
            <div className="text-book-text-muted inline-flex items-center gap-1.5">
              <Loader2 size={10} className="animate-spin" /> 加载中…
            </div>
          ) : (
            <>
              <div className="flex items-center gap-2 mb-1.5">
                <BookButton variant="secondary" onClick={loadGraph}>
                  刷新
                </BookButton>
                {graph && (
                  <span className="text-book-text-muted">
                    总节点 {graph.nodes.length} / 总边 {graph.edges.length}
                  </span>
                )}
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <div className="text-book-text-muted mb-1">依赖（→）</div>
                  {deps.length === 0 ? (
                    <div className="italic text-book-text-muted">无</div>
                  ) : (
                    <div className="flex flex-wrap gap-1">
                      {deps.map((d) => (
                        <span
                          key={d}
                          className="pyqt-pill !text-[10px] !border-book-primary/40 !text-book-primary"
                        >
                          →{d}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
                <div>
                  <div className="text-book-text-muted mb-1">反向依赖（←）</div>
                  {reverseDeps.length === 0 ? (
                    <div className="italic text-book-text-muted">无</div>
                  ) : (
                    <div className="flex flex-wrap gap-1">
                      {reverseDeps.map((d) => (
                        <span
                          key={d}
                          className="pyqt-pill !text-[10px] !border-warning/40 !text-warning"
                        >
                          ←{d}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </>
          )}
        </div>
      </details>

      <details className="pyqt-card p-2.5">
        <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
          <CheckCircle2 size={11} /> 变量校验
        </summary>
        <div className="mt-2 text-[11px] space-y-1.5">
          <div className="text-book-text-muted">
            输入要传入的变量名（逗号或换行分隔），检查是否覆盖模板所有占位符。
          </div>
          <textarea
            value={varsInput}
            onChange={(e) => handleVarsInputChange(e.target.value)}
            className="pyqt-input w-full text-[11px] h-16 resize-y font-mono"
            placeholder="如：title, chapter_number, character_name"
          />
          <div className="flex items-center gap-2">
            <BookButton variant="secondary" onClick={handleValidate} disabled={validateBusy}>
              {validateBusy ? (
                <Loader2 size={11} className="animate-spin" />
              ) : (
                "校验"
              )}
            </BookButton>
            {missingVars !== null && missingVars.length === 0 && (
              <span className="text-success">✓ 所有占位符已覆盖</span>
            )}
          </div>
          {missingVars !== null && missingVars.length > 0 && (
            <div className="flex flex-wrap gap-1 pt-1">
              <span className="text-error">缺失：</span>
              {missingVars.map((m) => (
                <span
                  key={m}
                  className="pyqt-pill !text-[10px] !border-error/50 !text-error"
                >
                  {m}
                </span>
              ))}
            </div>
          )}
        </div>
      </details>

      <details className="pyqt-card p-2.5">
        <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
          <Play size={11} /> 渲染对照
        </summary>
        <div className="mt-2 text-[11px] space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="text-book-text-muted shrink-0">project_id（可空）</span>
            <input
              value={abProjectId}
              onChange={(e) => handleRenderInputChange("project", e.target.value)}
              placeholder="如 1"
              className="pyqt-input !py-1 !px-2 w-20"
            />
          </div>
          <div className="text-book-text-muted">变量 JSON 对象</div>
          <textarea
            value={abVarsJson}
            onChange={(e) => handleRenderInputChange("vars", e.target.value)}
            className="pyqt-input w-full text-[11px] h-20 resize-y font-mono"
            placeholder='{"title": "示例", "chapter": 1}'
          />
          <div className="flex items-center gap-2 flex-wrap">
            <BookButton variant="secondary" onClick={handleDirectRender} disabled={renderBusy}>
              {renderBusy ? (
                <Loader2 size={11} className="animate-spin" />
              ) : (
                "直接渲染"
              )}
            </BookButton>
            <BookButton variant="primary" onClick={handleAbRender} disabled={abBusy}>
              {abBusy ? (
                <Loader2 size={11} className="animate-spin" />
              ) : (
                "当前/原始对照"
              )}
            </BookButton>
          </div>
          {renderResult !== null && (
            <div>
              <div className="text-book-text-muted">直接渲染结果</div>
              <pre className="rounded border border-book-border/40 bg-book-bg-tertiary/40 p-2 text-[10px] font-mono whitespace-pre-wrap break-words max-h-44 overflow-y-auto custom-scrollbar">
                {renderResult || "（空）"}
              </pre>
            </div>
          )}
          {abResult !== null && (
            <div>
              <div className="text-book-text-muted">当前模板</div>
              <pre className="rounded border border-book-border/40 bg-book-bg-tertiary/40 p-2 text-[10px] font-mono whitespace-pre-wrap break-words max-h-44 overflow-y-auto custom-scrollbar">
                {abResult.version_a || "（空）"}
              </pre>
              <div className="text-book-text-muted mt-1.5">原始模板</div>
              <pre className="rounded border border-book-border/40 bg-book-bg-tertiary/40 p-2 text-[10px] font-mono whitespace-pre-wrap break-words max-h-44 overflow-y-auto custom-scrollbar">
                {abResult.version_b || "（空）"}
              </pre>
            </div>
          )}
        </div>
      </details>
    </>
  );
};
