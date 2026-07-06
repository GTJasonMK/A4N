import React from "react";
import { Braces, MapPin, Network, ScrollText } from "lucide-react";
import {
  CONFLICT_KEYS,
  CORE_KEYS,
  CULTURE_KEYS,
  FACTION_KEYS,
  formatFieldLabel,
  HISTORY_KEYS,
  isEmptyValue,
  LOCATION_KEYS,
  MYSTERY_KEYS,
  normalizeList,
  pickFirst,
  PREMISE_KEYS,
  PROMISE_KEYS,
  stringifyPretty,
  summarizeExtraValue,
  SYSTEM_KEYS,
  SYSTEM_KEYS_ORDER,
  valueToText,
  type WorldListItem,
  type WorldObject,
} from "./worldTabUtils";

const SectionTitle: React.FC<{
  icon: React.ReactNode;
  title: string;
  meta?: string;
}> = ({ icon, title, meta }) => (
  <div className="mb-2 flex items-center gap-2 border-b border-book-border/40 pb-1.5">
    <span className="text-book-primary">{icon}</span>
    <h4 className="text-xs font-semibold text-book-text-main">{title}</h4>
    {meta && <span className="text-[10px] text-book-text-muted">{meta}</span>}
  </div>
);

export const TextBlock: React.FC<{ value: unknown; emptyText?: string }> = ({
  value,
  emptyText = "暂未填写。",
}) => {
  const text = valueToText(value).trim();
  return (
    <div className="whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
      {text || <span className="text-book-text-muted">{emptyText}</span>}
    </div>
  );
};

const ListBlock: React.FC<{ items: WorldListItem[]; emptyText: string }> = ({
  items,
  emptyText,
}) => {
  if (items.length === 0) {
    return <div className="text-xs text-book-text-muted">{emptyText}</div>;
  }

  return (
    <ul className="space-y-1.5">
      {items.map((item, index) => (
        <li
          key={`${item.title ?? item.description.slice(0, 32)}-${index}`}
          className="a4n-stat-tile px-2.5 py-2"
        >
          {item.title ? (
            <div className="text-xs font-semibold text-book-text-main">{item.title}</div>
          ) : null}
          {item.description && (
            <div
              className={`${item.title ? "mt-1 " : ""}whitespace-pre-wrap text-[11px] leading-relaxed text-book-text-sub`}
            >
              {item.description}
            </div>
          )}
        </li>
      ))}
    </ul>
  );
};

const ExtraFields: React.FC<{ obj: WorldObject; consumedKeys: Set<string> }> = ({
  obj,
  consumedKeys,
}) => {
  const entries = Object.entries(obj).filter(
    ([key, value]) => !consumedKeys.has(key) && !SYSTEM_KEYS.has(key) && !isEmptyValue(value),
  );
  const systemEntries = Object.entries(obj).filter(
    ([key, value]) => !consumedKeys.has(key) && SYSTEM_KEYS.has(key) && !isEmptyValue(value),
  );

  if (entries.length === 0 && systemEntries.length === 0) return null;

  return (
    <div className="space-y-3">
      {entries.length > 0 && (
        <div>
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="额外设定"
            meta={`${entries.length} 项`}
          />
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            {entries.map(([key, value]) => (
              <div
                key={key}
                className="a4n-stat-tile px-2.5 py-2"
              >
                <div className="mb-1 text-[10px] font-semibold text-book-text-muted">
                  {formatFieldLabel(key)}
                </div>
                <div className="max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-[11px] leading-relaxed text-book-text-sub custom-scrollbar">
                  {summarizeExtraValue(value)}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {systemEntries.length > 0 && (
        <details className="a4n-page-surface px-3 py-2">
          <summary className="cursor-pointer text-xs font-semibold text-book-text-sub">
            系统记录（{systemEntries.length} 项）
          </summary>
          <div className="mt-2 grid grid-cols-1 gap-2">
            {systemEntries.map(([key, value]) => (
              <div key={key}>
                <div className="mb-1 text-[10px] text-book-text-muted">{key}</div>
                <pre className="max-h-36 overflow-y-auto rounded-sm bg-book-bg-tertiary/50 p-2 text-[10px] leading-relaxed text-book-text-sub whitespace-pre-wrap break-words custom-scrollbar">
                  {stringifyPretty(value)}
                </pre>
              </div>
            ))}
          </div>
        </details>
      )}
    </div>
  );
};

export const WorldObjectView: React.FC<{ obj: WorldObject }> = ({ obj }) => {
  const premise = pickFirst(obj, PREMISE_KEYS);
  const core = pickFirst(obj, CORE_KEYS);
  const system = pickFirst(obj, SYSTEM_KEYS_ORDER);
  const history = pickFirst(obj, HISTORY_KEYS);
  const culture = pickFirst(obj, CULTURE_KEYS);
  const conflicts = pickFirst(obj, CONFLICT_KEYS);
  const mysteries = pickFirst(obj, MYSTERY_KEYS);
  const promises = pickFirst(obj, PROMISE_KEYS);
  const locations = pickFirst(obj, LOCATION_KEYS);
  const factions = pickFirst(obj, FACTION_KEYS);
  const consumedKeys = new Set(
    [
      premise?.[0],
      core?.[0],
      system?.[0],
      history?.[0],
      culture?.[0],
      conflicts?.[0],
      mysteries?.[0],
      promises?.[0],
      locations?.[0],
      factions?.[0],
    ].filter((key): key is string => !!key),
  );
  const locationItems = normalizeList(locations?.[1]);
  const factionItems = normalizeList(factions?.[1]);
  const historyItems = normalizeList(history?.[1]);
  const conflictItems = normalizeList(conflicts?.[1]);
  const mysteryItems = normalizeList(mysteries?.[1]);
  const promiseItems = normalizeList(promises?.[1]);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <SectionTitle icon={<ScrollText size={13} />} title="世界前提" />
          <TextBlock value={premise?.[1]} emptyText="世界的一句话核心设定尚未填写。" />
        </div>

        <div className="a4n-page-surface p-3">
          <SectionTitle icon={<ScrollText size={13} />} title="核心规则" />
          <TextBlock value={core?.[1]} emptyText="世界运行规则、时代背景或能力体系尚未填写。" />
        </div>
      </div>

      <div className="a4n-page-surface p-3">
        <SectionTitle icon={<Braces size={13} />} title="能力/机制体系" />
        <TextBlock value={system?.[1]} emptyText="力量来源、机制规则、限制与代价尚未填写。" />
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <SectionTitle
            icon={<MapPin size={13} />}
            title="关键地点"
            meta={locationItems.length ? `${locationItems.length} 处` : undefined}
          />
          <ListBlock items={locationItems} emptyText="暂无关键地点。" />
        </div>

        <div className="a4n-page-surface p-3">
          <SectionTitle
            icon={<Network size={13} />}
            title="主要阵营"
            meta={factionItems.length ? `${factionItems.length} 个` : undefined}
          />
          <ListBlock items={factionItems} emptyText="暂无主要阵营。" />
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="历史脉络"
            meta={historyItems.length ? `${historyItems.length} 条` : undefined}
          />
          <ListBlock items={historyItems} emptyText="暂无历史脉络。" />
        </div>

        <div className="a4n-page-surface p-3">
          <SectionTitle icon={<ScrollText size={13} />} title="文化与社会" />
          <TextBlock value={culture?.[1]} emptyText="暂无文化、阶层或日常生活设定。" />
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <SectionTitle
            icon={<Network size={13} />}
            title="冲突来源"
            meta={conflictItems.length ? `${conflictItems.length} 个` : undefined}
          />
          <ListBlock items={conflictItems} emptyText="暂无冲突来源。" />
        </div>

        <div className="a4n-page-surface p-3">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="悬念与伏笔"
            meta={mysteryItems.length ? `${mysteryItems.length} 条` : undefined}
          />
          <ListBlock items={mysteryItems} emptyText="暂无悬念与伏笔。" />
        </div>
      </div>

      <div className="a4n-page-surface p-3">
        <SectionTitle
          icon={<ScrollText size={13} />}
          title="类型承诺"
          meta={promiseItems.length ? `${promiseItems.length} 条` : undefined}
        />
        <ListBlock items={promiseItems} emptyText="暂无类型承诺。" />
      </div>

      <ExtraFields obj={obj} consumedKeys={consumedKeys} />
    </div>
  );
};

export const WorldSummaryView: React.FC<{ obj: WorldObject }> = ({ obj }) => {
  const premise = pickFirst(obj, PREMISE_KEYS);
  const core = pickFirst(obj, CORE_KEYS);
  const locations = normalizeList(pickFirst(obj, LOCATION_KEYS)?.[1]);
  const factions = normalizeList(pickFirst(obj, FACTION_KEYS)?.[1]);
  const conflicts = normalizeList(pickFirst(obj, CONFLICT_KEYS)?.[1]);
  const mysteries = normalizeList(pickFirst(obj, MYSTERY_KEYS)?.[1]);

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="a4n-page-surface p-3">
          <SectionTitle icon={<ScrollText size={13} />} title="世界前提" />
          <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
            {valueToText(premise?.[1]) || "世界的一句话核心设定尚未填写。"}
          </div>
        </div>
        <div className="a4n-page-surface p-3">
          <SectionTitle icon={<ScrollText size={13} />} title="核心规则" />
          <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
            {valueToText(core?.[1]) || "世界运行规则、时代背景或能力体系尚未填写。"}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs md:grid-cols-4">
        <WorldCount label="地点" value={locations.length} />
        <WorldCount label="阵营" value={factions.length} />
        <WorldCount label="冲突" value={conflicts.length} />
        <WorldCount label="伏笔" value={mysteries.length} />
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <WorldMiniList title="关键地点" items={locations.slice(0, 3)} emptyText="暂无关键地点。" />
        <WorldMiniList title="主要阵营" items={factions.slice(0, 3)} emptyText="暂无主要阵营。" />
      </div>
    </div>
  );
};

const WorldCount: React.FC<{ label: string; value: number }> = ({ label, value }) => (
  <div className="a4n-stat-tile px-2.5 py-2 text-center">
    <div className="text-[10px] text-book-text-muted">{label}</div>
    <div className="font-mono text-base font-semibold text-book-primary">{value}</div>
  </div>
);

const WorldMiniList: React.FC<{
  title: string;
  items: WorldListItem[];
  emptyText: string;
}> = ({ title, items, emptyText }) => (
  <div className="a4n-page-surface p-3">
    <div className="mb-2 text-xs font-semibold text-book-text-main">{title}</div>
    {items.length === 0 ? (
      <div className="text-xs text-book-text-muted">{emptyText}</div>
    ) : (
      <ul className="space-y-1.5">
        {items.map((item, index) => (
          <li key={`${item.title ?? item.description.slice(0, 32)}-${index}`} className="min-w-0">
            {item.title ? (
              <div className="truncate text-xs font-semibold text-book-text-main">{item.title}</div>
            ) : null}
            {item.description ? (
              <div
                className={`${item.title ? "mt-0.5 " : ""}line-clamp-2 text-[11px] leading-relaxed text-book-text-muted`}
              >
                {item.description}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    )}
  </div>
);
