import React, { useMemo } from "react";
import { BookTextarea } from "../../../components/ui/BookInput";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import {
  CORE_KEYS,
  FACTION_KEYS,
  LOCATION_KEYS,
  inputTextValue,
  namedListToEditText,
  pickFirst,
  textToNamedWorldList,
  tryParseObjectDraft,
  valueToText,
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
  const modeDescription = activeMode === "structured"
    ? "适合日常维护；未展示的扩展字段会原样保留。"
    : activeMode === "json"
      ? "直接编辑完整数据结构，仅建议熟悉 JSON 时使用。"
      : "使用自然语言记录；保存后世界观将以纯文本形式存储。";

  const updateObjectDraft = (patch: (obj: WorldObject) => void) => {
    const base = tryParseObjectDraft(jsonDraft);
    if (!base) return;
    const next = { ...base };
    patch(next);
    onJsonDraftChange(JSON.stringify(next, null, 2));
  };

  const handleModeChange = (mode: WorldEditMode) => {
    if (mode === "text" && !textDraft.trim() && draftObject) {
      onTextDraftChange(valueToText(draftObject));
    }
    onEditModeChange(mode);
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2 border-b border-book-border/45 pb-3">
        <SegmentedControl
          ariaLabel="世界观编辑模式"
          value={activeMode}
          onChange={handleModeChange}
          className="w-full [&_.a4n-seg__btn]:flex-1 sm:w-auto sm:[&_.a4n-seg__btn]:flex-none"
          options={[
            { id: "structured", label: "结构化", disabled: !canStructured },
            { id: "json", label: "JSON", title: "编辑完整 JSON 结构" },
            { id: "text", label: "纯文本" },
          ]}
        />
        <p className="text-small leading-relaxed text-book-text-muted">
          {modeDescription}
        </p>
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
          className="min-h-[320px] text-sm font-serif leading-relaxed sm:min-h-[400px]"
          placeholder="用自然语言描述世界观设定…"
        />
      ) : (
        <BookTextarea
          value={jsonDraft}
          onChange={(event) => onJsonDraftChange(event.target.value)}
          disabled={saving}
          spellCheck={false}
          error={jsonDraft.trim() && !draftObject ? "请输入合法 JSON 对象" : undefined}
          className="min-h-[360px] font-mono text-xs leading-relaxed sm:min-h-[460px]"
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
      <div className="border-b border-book-border/45 pb-4">
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
        <div className="border-b border-book-border/45 pb-4 lg:border-b-0 lg:border-r lg:pb-0 lg:pr-3">
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
          <div className="mt-1.5 text-small text-book-text-muted">
            每行一条，可写“名称｜描述”，支持中文竖线或英文 |。
          </div>
        </div>

        <div className="lg:pl-3">
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
          <div className="mt-1.5 text-small text-book-text-muted">
            复杂字段请切换到高级模式继续调整。
          </div>
        </div>
      </div>
    </>
  );
};
