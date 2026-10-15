import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "../../src/i18n";
import { Modal } from "../../src/components/ui/Modal";
import { ThemedSelect } from "../../src/components/ui/ThemedSelect";
import { ThemedCombobox } from "../../src/components/ui/ThemedCombobox";
import { StatusBar, StatusBarProvider, StatusBarToggle } from "../../src/components/ui/StatusBar";
import { ChapterOutlineSection } from "../../src/pages/novel-detail/outlines/ChapterOutlineSection";
import type { ChapterOutline } from "../../src/api/outline";
import "../../src/index.css";

const poseOptions = ["普通", "思考", "惊讶"].map((value) => ({ value, label: value }));
const catalogOptions = Array.from({ length: 80 }, (_, index) => ({
  value: `model-${index}`,
  label: index === 0
    ? "openai/ui-structured-long-context-instruction-tuned-tool-use-128k-2026-10-14"
    : `创作模型 ${index + 1}`,
}));
const outline: ChapterOutline = {
  id: 1, project_id: 1, chapter_number: 1, part_id: null, source_kind: null, status: "draft",
  title: "雨夜", summary: "旅人在旧书店发现一封信。", word_count_target: 2000,
  beats: null, characters_involved: null, foreshadowing_payoff: null, foreshadowing_setup: null,
  key_scenes: null, pacing_score: null, created_at: null, updated_at: null,
};
function Controls() {
  const [dialog, setDialog] = useState<string | null>(null);
  const [childOpen, setChildOpen] = useState(false);
  const [mode, setMode] = useState("narration");
  const [pose, setPose] = useState("普通");
  const [savedPoses, setSavedPoses] = useState<string[]>([]);
  const [locked, setLocked] = useState(false);
  const [selectDisabled, setSelectDisabled] = useState(false);
  const [outlineActions, setOutlineActions] = useState<string[]>([]);
  const [autoFocusOpen, setAutoFocusOpen] = useState(false);
  const [catalogModel, setCatalogModel] = useState("model-0");
  const [handoffPhase, setHandoffPhase] = useState<"detail" | "edit" | null>(null);

  const poseField = (
    <label className="flex min-w-0 flex-col gap-2">
      <span>角色姿势</span>
      <ThemedCombobox value={pose} options={poseOptions} readOnly={locked}
        onChange={setPose} onBlur={() => setSavedPoses((previous) => [...previous, pose])} />
    </label>
  );

  return (
    <div className="a4n-app-shell h-dvh overflow-y-auto bg-book-bg p-4 text-book-text-main">
      <main className="mx-auto max-w-xl space-y-4">
        <h1 className="font-serif text-lg">场景编辑与控件交互检查</h1>
        <section className="a4n-flat-surface space-y-3 p-4">
          <div className="flex flex-wrap gap-2">
            <button className="pyqt-btn-primary" onClick={() => setDialog("scene")}>编辑场景</button>
            <button className="pyqt-btn-secondary" onClick={() => setDialog("summary")}>阅读说明</button>
            <button className="pyqt-btn-secondary" onClick={() => setDialog("roving")}>选择章节</button>
            <button className="pyqt-btn-secondary" onClick={() => setAutoFocusOpen(true)}>快速命名</button>
            <button className="pyqt-btn-secondary" onClick={() => setHandoffPhase("detail")}>切换编辑弹窗</button>
          </div>
          {poseField}
          <label className="flex items-center gap-2"><input type="checkbox" checked={locked} onChange={(event) => setLocked(event.target.checked)} />只读姿势</label>
          <output data-saved-poses>{JSON.stringify(savedPoses)}</output>
          <label className="flex flex-col gap-2"><span>下一字段</span><input className="pyqt-input" /></label>
          <label className="flex flex-col gap-2">
            <span id="mode-label">场景类型</span>
            <ThemedSelect value={mode} onChange={setMode} disabled={selectDisabled}
              aria-invalid="true" aria-describedby="mode-error" className="pyqt-input">
              <option value="narration">叙事</option><option value="dialogue">对话</option>
              <option value="unavailable" disabled>不可用类型</option>
            </ThemedSelect>
          </label>
          <span id="mode-error">请选择合适的场景类型。</span>
          <button className="pyqt-btn-secondary" onClick={() => setSelectDisabled((value) => !value)}>切换类型禁用</button>
        </section>
        <section>
          <ChapterOutlineSection outlines={[outline]} canEdit chapterCount={1} generating={false}
            editingId={null} editForm={{ title: "", summary: "" }} savingId={null} regenId={null} delActing={null}
            onChapterCountChange={() => {}} onGenerate={() => {}} onPreviewGenerate={() => {}} onCreate={() => {}}
            onOpenDetail={() => setOutlineActions((items) => [...items, "detail"])}
            onOpenAction={() => setOutlineActions((items) => [...items, "actions"])}
            onCancelEdit={() => {}} onEditFormChange={() => {}} onSave={() => {}} />
          <output data-outline-actions>{JSON.stringify(outlineActions)}</output>
        </section>
        <section className="a4n-flat-surface space-y-3 p-4">
          <label className="flex flex-col gap-2"><span>长模型目录</span>
            <ThemedSelect id="long-select" value={catalogModel} onChange={setCatalogModel}>
              {catalogOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
            </ThemedSelect>
          </label>
          <label className="flex flex-col gap-2"><span>可搜索模型目录</span>
            <ThemedCombobox id="long-combobox" value={catalogModel} onChange={setCatalogModel} options={catalogOptions} />
          </label>
        </section>
      </main>
      <button id="under-orb" className="pyqt-btn-secondary fixed bottom-3 right-3 h-12 w-12" aria-label="底部主操作">保存</button>
      <StatusBarToggle />
      <StatusBar actions={<button className="a4n-status-bar-action" aria-label="主题设置">☼</button>} />

      <Modal isOpen={dialog != null} onClose={() => setDialog(null)} title={dialog === "scene" ? "编辑场景" : dialog === "summary" ? "阅读说明" : "选择章节"}>
        {dialog === "scene" ? (
          <div className="space-y-4">
            <label className="flex flex-col gap-2"><span>对白方式</span>
              <ThemedSelect value={mode} onChange={setMode} className="pyqt-input">
                <option value="narration">叙事</option><option value="dialogue">对话</option>
              </ThemedSelect>
            </label>
            {poseField}
            <button id="open-character" className="pyqt-btn-secondary" onClick={() => setChildOpen(true)}>查看角色</button>
            <button id="replace-dialog" className="pyqt-btn-secondary" onClick={() => { setChildOpen(true); setDialog(null); }}>直接编辑角色</button>
          </div>
        ) : dialog === "summary" ? (
          <details><summary>高级说明</summary><p>这些参数仅用于当前场景。</p></details>
        ) : (
          <div role="tablist" aria-label="章节"><button role="tab" aria-selected="true" tabIndex={0}>第一章</button><button role="tab" aria-selected="false" tabIndex={-1}>第二章</button></div>
        )}
      </Modal>
      <Modal isOpen={childOpen} onClose={() => setChildOpen(false)} title="角色详情"><p>林间旅人</p></Modal>
      {autoFocusOpen ? <Modal isOpen onClose={() => setAutoFocusOpen(false)} title="命名"><input aria-label="名称" className="pyqt-input" autoFocus /></Modal> : null}
      {handoffPhase ? (
        <Modal key={handoffPhase} isOpen onClose={() => setHandoffPhase(null)} title={handoffPhase === "detail" ? "动态详情" : "动态编辑"}>
          {handoffPhase === "detail" ? <button className="pyqt-btn-secondary" onClick={() => setHandoffPhase("edit")}>转到编辑器</button> : <input aria-label="编辑名称" className="pyqt-input" autoFocus />}
        </Modal>
      ) : null}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode><I18nProvider initialLocale="zh-CN"><StatusBarProvider><Controls /></StatusBarProvider></I18nProvider></React.StrictMode>,
);
