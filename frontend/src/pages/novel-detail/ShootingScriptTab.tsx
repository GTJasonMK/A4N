import React, { useEffect, useMemo, useState } from "react";
import { RefreshCw, Save, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";

import { extractErrorMessage } from "../../api";
import type { ChapterData } from "../../api/chapter";
import {
  editChapterShootingScript,
  generateChapterShootingScript,
  generateNextChapterShootingScriptUnit,
  getChapterShootingScript,
  getChapterShootingScriptUnitProgress,
  type ChapterShootingScriptUnitProgressView,
  type ChapterShootingScriptV1,
  type ChapterShootingScriptView,
} from "../../api/shootingScript";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { cancelWorkflow } from "../../api/streaming";
import {
  allocateHostTaskId,
  useProgressTask,
} from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { ScriptContent } from "./shooting-script/ScriptContent";
import { StructuredScriptEditor } from "./shooting-script/StructuredScriptEditor";
import { EmptyHint } from "./shared";

interface ShootingScriptTabProps {
  projectId: number;
  canEdit: boolean;
  chapter: ChapterData | null;
}

export function assertNextUnitProgressPreservesScenes(
  previous: ChapterShootingScriptUnitProgressView | null,
  next: ChapterShootingScriptUnitProgressView,
): void {
  const sceneCountMatches =
    next.completed_unit_count > 0 &&
    next.script.scenes.length === next.completed_unit_count;
  const preservesPrevious =
    previous == null ||
    (next.workflow_run_id === previous.workflow_run_id &&
      next.total_unit_count === previous.total_unit_count &&
      next.completed_unit_count > previous.completed_unit_count &&
      previous.script.scenes.every(
        (scene, index) =>
          JSON.stringify(scene) === JSON.stringify(next.script.scenes[index]),
      ));
  if (!sceneCountMatches || !preservesPrevious) {
    throw new Error(
      "生成下一个场景返回了倒退或改写已有场景的结果，已拒绝覆盖当前页面数据",
    );
  }
}

export const ShootingScriptTab: React.FC<ShootingScriptTabProps> = ({
  projectId,
  canEdit,
  chapter,
}) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const chapterId = chapter?.id ?? null;
  const [view, setView] = useState<ChapterShootingScriptView | null>(null);
  const [unitProgress, setUnitProgress] =
    useState<ChapterShootingScriptUnitProgressView | null>(null);
  const [scriptLoading, setScriptLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [draft, setDraft] = useState<ChapterShootingScriptV1 | null>(null);
  const [invalidatedProjectIds, setInvalidatedProjectIds] = useState<number[]>(
    [],
  );
  const isDirty = useMemo(
    () =>
      view != null &&
      draft != null &&
      JSON.stringify(view.script) !== JSON.stringify(draft),
    [draft, view],
  );

  useEffect(() => {
    if (chapterId == null) {
      setView(null);
      setUnitProgress(null);
      setDraft(null);
      setInvalidatedProjectIds([]);
      setScriptLoading(false);
      return;
    }
    let active = true;
    setView(null);
    setScriptLoading(true);
    void Promise.all([
      getChapterShootingScript(projectId, chapterId),
      getChapterShootingScriptUnitProgress(projectId, chapterId),
    ])
      .then(([value, progress]) => {
        if (!active) return;
        setView(value);
        setUnitProgress(value ? null : progress);
        setDraft(value ? structuredClone(value.script) : null);
        setInvalidatedProjectIds([]);
      })
      .catch((error) => {
        if (active) {
          addToast(extractErrorMessage(error, "加载拍摄剧本失败"), "error");
        }
      })
      .finally(() => {
        if (active) setScriptLoading(false);
      });
    return () => {
      active = false;
    };
  }, [addToast, chapterId, projectId]);

  const generate = async () => {
    if (chapterId == null || generating || saving || scriptLoading) return;
    if (view) {
      const confirmed = await confirmDialog({
        title: "重新生成拍摄剧本",
        message: isDirty
          ? "重新生成会覆盖当前拍摄剧本，你尚未保存的改编也会丢失。是否继续？"
          : "重新生成会覆盖当前拍摄剧本，并可能使依赖它的下游内容需要重新生成。是否继续？",
        confirmText: "重新生成",
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    setGenerating(true);
    try {
      const taskId = allocateHostTaskId("chapter-shooting-script", projectId);
      const generated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflow,
          pollTaskProgress: true,
          title: "生成结构化拍摄剧本",
          startMessage: "正在准备剧本改编",
          successMessage: "结构化拍摄剧本生成完成",
          errorMessage: "结构化拍摄剧本生成失败",
          total: 100,
          cancelLabel: "停止生成",
          runningHint: "生成过程中会保留已完成的阶段，可在下次继续恢复",
        },
        () => generateChapterShootingScript(projectId, chapterId, taskId),
      );
      setView(generated);
      setUnitProgress(null);
      setDraft(structuredClone(generated.script));
      setInvalidatedProjectIds([]);
      addToast("拍摄剧本生成完成", "success");
    } catch (error) {
      const message = extractErrorMessage(error, "生成拍摄剧本失败");
      addToast(
        isCancelledOperation(message) ? "拍摄剧本生成已停止" : message,
        isCancelledOperation(message) ? "info" : "error",
      );
    } finally {
      setGenerating(false);
    }
  };

  const generateNextUnit = async () => {
    if (chapterId == null || generating || saving || scriptLoading) return;
    setGenerating(true);
    try {
      const taskId = allocateHostTaskId("chapter-shooting-script-unit", projectId);
      const generated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflow,
          pollTaskProgress: true,
          title: "生成下一个拍摄场景",
          startMessage: "正在准备场景改编",
          successMessage: "拍摄场景已生成",
          errorMessage: "拍摄场景生成失败",
          total: 100,
          cancelLabel: "停止生成",
          runningHint: "已完成的场景会保存，之后可继续生成下一个",
        },
        () => generateNextChapterShootingScriptUnit(projectId, chapterId, taskId),
      );
      assertNextUnitProgressPreservesScenes(unitProgress, generated);
      if (generated.completed_record) {
        const completedView = {
          record: generated.completed_record,
          script: generated.script,
        } satisfies ChapterShootingScriptView;
        setView(completedView);
        setUnitProgress(null);
        setDraft(structuredClone(generated.script));
      } else {
        setUnitProgress(generated);
      }
      setInvalidatedProjectIds([]);
      addToast(
        generated.completed_record
          ? "全部拍摄场景已生成"
          : `已生成 ${generated.completed_unit_count}/${generated.total_unit_count} 个场景`,
        "success",
      );
    } catch (error) {
      const message = extractErrorMessage(error, "拍摄场景生成失败");
      addToast(
        isCancelledOperation(message) ? "拍摄场景生成已停止" : message,
        isCancelledOperation(message) ? "info" : "error",
      );
    } finally {
      setGenerating(false);
    }
  };

  const save = async () => {
    if (!view || !draft || chapterId == null || saving || generating || scriptLoading) return;
    setSaving(true);
    try {
      const result = await editChapterShootingScript(
        projectId,
        chapterId,
        view,
        draft,
      );
      setView(result.shooting_script);
      setDraft(structuredClone(result.shooting_script.script));
      setInvalidatedProjectIds(result.invalidated_project_ids);
      addToast(
        result.invalidated_project_ids.length > 0
          ? `拍摄剧本已保存，${result.invalidated_project_ids.length} 个下游项目已失效`
          : "拍摄剧本已保存",
        "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, "保存拍摄剧本失败"), "error");
    } finally {
      setSaving(false);
    }
  };

  if (!chapter) {
    return <EmptyHint text="请先选择已生成的章节，再进行剧本改编。" />;
  }

  const actionsDisabled = generating || saving || scriptLoading;

  return (
    <section className="min-w-0 space-y-3">
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-2 px-1">
        <div className="text-small text-book-text-muted">
          {scriptLoading
            ? "正在加载拍摄剧本…"
            : generating
              ? view
                ? "正在重新生成拍摄剧本…"
                : "正在生成拍摄剧本…"
              : saving
                ? "正在保存拍摄剧本…"
                : view
                  ? `拍摄剧本 · 第 ${view.record.revision} 版${isDirty ? " · 有未保存修改" : ""}`
                  : unitProgress
                    ? `已完成 ${unitProgress.completed_unit_count}/${unitProgress.total_unit_count} 个拍摄场景`
                  : "当前章节尚未生成拍摄剧本"}
        </div>
        {canEdit ? (
          <div className="ml-auto flex items-center gap-2">
            {view && draft ? (
              <>
                <Dropdown
                  items={[
                    {
                      label: generating ? "正在重新生成拍摄剧本…" : "重新生成拍摄剧本",
                      onClick: () => void generate(),
                      disabled: actionsDisabled,
                      icon: generating ? (
                        <LoadingSpinner size="sm" />
                      ) : (
                        <RefreshCw size={12} />
                      ),
                    },
                  ]}
                />
                <BookButton
                  size="sm"
                  onClick={() => void save()}
                  disabled={actionsDisabled || !isDirty}
                >
                  {saving ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Save size={13} />
                  )}
                  {saving ? "保存中…" : "保存改编"}
                </BookButton>
              </>
            ) : (
              <>
                {unitProgress ? (
                  <Dropdown
                    items={[
                      {
                        label: "生成全部剩余场景",
                        onClick: () => void generate(),
                        disabled: actionsDisabled,
                        icon: <RefreshCw size={12} />,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  size="sm"
                  onClick={() => void generateNextUnit()}
                  disabled={actionsDisabled}
                >
                  {generating ? <LoadingSpinner size="sm" /> : <Wand2 size={13} />}
                  {generating
                    ? "生成中…"
                    : unitProgress
                      ? "生成下一个场景"
                      : "生成第一个场景"}
                </BookButton>
              </>
            )}
          </div>
        ) : null}
      </div>
      {scriptLoading ? (
        <div className="a4n-flat-surface py-12 text-center">
          <LoadingSpinner size="md" text="加载拍摄剧本…" inline={false} />
        </div>
      ) : !view && !unitProgress ? (
        <div className="a4n-flat-surface p-3 sm:p-4">
          <EmptyHint text="生成后可按场景编辑导演意图、镜头设计与表演节拍。" />
        </div>
      ) : view ? (
        <>
          {canEdit && draft ? (
            <StructuredScriptEditor
              script={draft}
              onChange={setDraft}
              disabled={actionsDisabled}
            />
          ) : (
            <ScriptContent script={view.script} />
          )}
          {invalidatedProjectIds.length > 0 ? (
            <div className="px-1 text-small leading-relaxed text-book-text-muted">
              <p>
                保存后有 {invalidatedProjectIds.length} 个下游项目需要重新生成。
              </p>
              <details className="mt-1">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  诊断信息
                </summary>
                <p className="mt-1 break-all font-mono text-caption">
                  下游项目 ID：{invalidatedProjectIds.join("、")}
                </p>
              </details>
            </div>
          ) : null}
        </>
      ) : unitProgress ? (
        <ScriptContent script={unitProgress.script} />
      ) : null}
    </section>
  );
};
