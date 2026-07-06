import React, { useMemo } from "react";
import { BookTextarea } from "../../../components/ui/BookInput";
import {
  CORE_KEYS,
  FACTION_KEYS,
  LOCATION_KEYS,
  inputTextValue,
  namedListToEditText,
  pickFirst,
  textToNamedWorldList,
  tryParseObjectDraft,
  type WorldEditMode,
  type WorldObject,
} from "./worldTabUtils";

interface WorldEditorProps {
  editMode: WorldEditMode;
  jsonDraft: string;
  textDraft: string;
  saving: boolean;
  onEditModeChange: (mode: WorldEditMode) => void;
  onJsonDraftChange: (draft: string) => void;
  onTextDraftChange: (draft: string) => void;
}

const WorldEditorTab: React.FC<{
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}> = ({ active, onClick, children }) => (
  <button
    type="button"
    data-active={active ? "1" : undefined}
    onClick={onClick}
    className="pyqt-tab !text-[11px]"
  >
    {children}
  </button>
);

export const WorldEditor: React.FC<WorldEditorProps> = ({
  editMode,
  jsonDraft,
  textDraft,
  saving,
  onEditModeChange,
  onJsonDraftChange,
  onTextDraftChange,
}) => {
  const draftObject = useMemo(() => tryParseObjectDraft(jsonDraft), [jsonDraft]);
  const coreDraft = draftObject ? pickFirst(draftObject, CORE_KEYS) : null;
  const locationsDraft = draftObject ? pickFirst(draftObject, LOCATION_KEYS) : null;
  const factionsDraft = draftObject ? pickFirst(draftObject, FACTION_KEYS) : null;
  const canStructured = Boolean(draftObject);
  const activeMode = editMode === "structured" && !canStructured ? "json" : editMode;

  const updateObjectDraft = (patch: (obj: WorldObject) => void) => {
    const base = tryParseObjectDraft(jsonDraft);
    if (!base) return;
    const next = { ...base };
    patch(next);
    onJsonDraftChange(JSON.stringify(next, null, 2));
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-2 border-b border-book-border/50 pb-2">
        <div className="flex items-center gap-1">
          <WorldEditorTab
            active={activeMode === "structured"}
            onClick={() => onEditModeChange("structured")}
          >
            结构化
          </WorldEditorTab>
          <WorldEditorTab active={activeMode === "json"} onClick={() => onEditModeChange("json")}>
            JSON
          </WorldEditorTab>
          <WorldEditorTab active={activeMode === "text"} onClick={() => onEditModeChange("text")}>
            纯文本
          </WorldEditorTab>
        </div>
        <span className="text-[11px] text-book-text-muted">
          结构化编辑会保留未显示的额外字段
        </span>
      </div>

      {activeMode === "structured" && draftObject ? (
        <StructuredWorldEditor
          coreDraft={coreDraft}
          locationsDraft={locationsDraft}
          factionsDraft={factionsDraft}
          saving={saving}
          onObjectDraftChange={updateObjectDraft}
        />
      ) : activeMode === "text" ? (
        <BookTextarea
          value={textDraft}
          onChange={(event) => onTextDraftChange(event.target.value)}
          disabled={saving}
          className="min-h-[400px] text-sm font-serif leading-relaxed"
          placeholder="用自然语言描述世界观设定…"
        />
      ) : (
        <BookTextarea
          value={jsonDraft}
          onChange={(event) => onJsonDraftChange(event.target.value)}
          disabled={saving}
          spellCheck={false}
          error={jsonDraft.trim() && !draftObject ? "请输入合法 JSON 对象" : undefined}
          className="min-h-[460px] font-mono text-xs leading-relaxed"
          placeholder='{"core_rules":"","key_locations":[],"factions":[]}'
        />
      )}
    </div>
  );
};

interface StructuredWorldEditorProps {
  coreDraft: [string, unknown] | null;
  locationsDraft: [string, unknown] | null;
  factionsDraft: [string, unknown] | null;
  saving: boolean;
  onObjectDraftChange: (patch: (obj: WorldObject) => void) => void;
}

const StructuredWorldEditor: React.FC<StructuredWorldEditorProps> = ({
  coreDraft,
  locationsDraft,
  factionsDraft,
  saving,
  onObjectDraftChange,
}) => {
  return (
    <>
      <div className="a4n-page-surface p-3">
        <BookTextarea
          label="核心规则"
          value={inputTextValue(coreDraft?.[1])}
          onChange={(event) =>
            onObjectDraftChange((obj) => {
              obj[coreDraft?.[0] ?? "core_rules"] = event.target.value;
            })
          }
          disabled={saving}
          className="min-h-[140px] text-sm font-serif leading-relaxed"
          placeholder="例如：时代背景、社会规则、能力体系、禁忌与代价…"
        />
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <BookTextarea
            label="关键地点"
            value={namedListToEditText(locationsDraft?.[1])}
            onChange={(event) =>
              onObjectDraftChange((obj) => {
                obj[locationsDraft?.[0] ?? "key_locations"] = textToNamedWorldList(
                  event.target.value,
                  locationsDraft?.[1],
                );
              })
            }
            disabled={saving}
            className="min-h-[220px] text-xs font-mono leading-relaxed"
            placeholder={"王都｜政治中心，暗流涌动\n灰港｜走私与情报交易之城\n禁林"}
          />
          <div className="mt-1.5 text-[10px] text-book-text-muted">
            每行一条，可写“名称｜描述”，支持中文竖线或英文 |。
          </div>
        </div>

        <div className="a4n-page-surface p-3">
          <BookTextarea
            label="主要阵营"
            value={namedListToEditText(factionsDraft?.[1])}
            onChange={(event) =>
              onObjectDraftChange((obj) => {
                obj[factionsDraft?.[0] ?? "factions"] = textToNamedWorldList(
                  event.target.value,
                  factionsDraft?.[1],
                );
              })
            }
            disabled={saving}
            className="min-h-[220px] text-xs font-mono leading-relaxed"
            placeholder={"学院派｜重视秩序与传承\n流亡者｜被放逐的旧贵族残党"}
          />
          <div className="mt-1.5 text-[10px] text-book-text-muted">
            复杂字段请切换到 JSON 模式继续调整。
          </div>
        </div>
      </div>
    </>
  );
};
