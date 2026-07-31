import React from "react";
import { Braces, MapPin, Network, ScrollText } from "lucide-react";
import {
  CONFLICT_KEYS,
  CORE_KEYS,
  CULTURE_KEYS,
  FACTION_KEYS,
  formatFieldLabel,
  HISTORY_KEYS,
  isSystemKey,
  isEmptyValue,
  LOCATION_KEYS,
  MYSTERY_KEYS,
  normalizeList,
  pickFirst,
  PREMISE_KEYS,
  PROMISE_KEYS,
  summarizeExtraValue,
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
    {meta && <span className="text-small text-book-text-muted">{meta}</span>}
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
    <ul className="divide-y divide-book-border/35 border-y border-book-border/35">
      {items.map((item, index) => (
        <li
          key={`${item.title ?? item.description.slice(0, 32)}-${index}`}
          className="py-2"
        >
          {item.title ? (
            <div className="text-xs font-semibold text-book-text-main">{item.title}</div>
          ) : null}
          {item.description && (
            <div
              className={`${item.title ? "mt-1 " : ""}whitespace-pre-wrap text-small leading-relaxed text-book-text-sub`}
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
    ([key, value]) => !consumedKeys.has(key) && !isSystemKey(key) && !isEmptyValue(value),
  );

  if (entries.length === 0) return null;

  return (
    <div className="space-y-3">
      {entries.length > 0 && (
        <div>
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="额外设定"
            meta={`${entries.length} 项`}
          />
          <dl className="divide-y divide-book-border/35 border-y border-book-border/35">
            {entries.map(([key, value]) => (
              <div
                key={key}
                className="grid gap-1 py-2 md:grid-cols-[120px_minmax(0,1fr)] md:gap-3"
              >
                <dt className="text-small font-medium text-book-text-muted">
                  {formatFieldLabel(key)}
                </dt>
                <dd className="whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">
                  {summarizeExtraValue(value)}
                </dd>
              </div>
            ))}
          </dl>
        </div>
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
        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title="世界前提" />
          <TextBlock value={premise?.[1]} emptyText="世界的一句话核心设定尚未填写。" />
        </section>

        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title="核心规则" />
          <TextBlock value={core?.[1]} emptyText="世界运行规则、时代背景或能力体系尚未填写。" />
        </section>
      </div>

      <section className="min-w-0 border-t border-book-border/35 pt-3">
        <SectionTitle icon={<Braces size={13} />} title="能力/机制体系" />
        <TextBlock value={system?.[1]} emptyText="力量来源、机制规则、限制与代价尚未填写。" />
      </section>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<MapPin size={13} />}
            title="关键地点"
            meta={locationItems.length ? `${locationItems.length} 处` : undefined}
          />
          <ListBlock items={locationItems} emptyText="暂无关键地点。" />
        </section>

        <section className="min-w-0">
          <SectionTitle
            icon={<Network size={13} />}
            title="主要阵营"
            meta={factionItems.length ? `${factionItems.length} 个` : undefined}
          />
          <ListBlock items={factionItems} emptyText="暂无主要阵营。" />
        </section>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="历史脉络"
            meta={historyItems.length ? `${historyItems.length} 条` : undefined}
          />
          <ListBlock items={historyItems} emptyText="暂无历史脉络。" />
        </section>

        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title="文化与社会" />
          <TextBlock value={culture?.[1]} emptyText="暂无文化、阶层或日常生活设定。" />
        </section>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<Network size={13} />}
            title="冲突来源"
            meta={conflictItems.length ? `${conflictItems.length} 个` : undefined}
          />
          <ListBlock items={conflictItems} emptyText="暂无冲突来源。" />
        </section>

        <section className="min-w-0">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title="悬念与伏笔"
            meta={mysteryItems.length ? `${mysteryItems.length} 条` : undefined}
          />
          <ListBlock items={mysteryItems} emptyText="暂无悬念与伏笔。" />
        </section>
      </div>

      <section className="min-w-0 border-t border-book-border/35 pt-3">
        <SectionTitle
          icon={<ScrollText size={13} />}
          title="类型承诺"
          meta={promiseItems.length ? `${promiseItems.length} 条` : undefined}
        />
        <ListBlock items={promiseItems} emptyText="暂无类型承诺。" />
      </section>

      <ExtraFields obj={obj} consumedKeys={consumedKeys} />
    </div>
  );
};

export const WorldSummaryView: React.FC<{ obj: WorldObject }> = ({ obj }) => {
  const premise = pickFirst(obj, PREMISE_KEYS);
  const core = pickFirst(obj, CORE_KEYS);
  const locations = normalizeList(pickFirst(obj, LOCATION_KEYS)?.[1]);
  const factions = normalizeList(pickFirst(obj, FACTION_KEYS)?.[1]);

  return (
    <div className="grid grid-cols-1 items-start gap-3 lg:grid-cols-2">
      <section className="a4n-flat-surface min-w-0 p-4">
        <SectionTitle icon={<ScrollText size={13} />} title="世界前提" />
        <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
          {valueToText(premise?.[1]) || "世界的一句话核心设定尚未填写。"}
        </div>
      </section>
      <section className="a4n-flat-surface min-w-0 p-4">
        <SectionTitle icon={<ScrollText size={13} />} title="核心规则" />
        <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
          {valueToText(core?.[1]) || "世界运行规则、时代背景或能力体系尚未填写。"}
        </div>
      </section>
      <WorldMiniList title="关键地点" items={locations} emptyText="暂无关键地点。" />
      <WorldMiniList title="主要阵营" items={factions} emptyText="暂无主要阵营。" />
    </div>
  );
};

const WorldMiniList: React.FC<{
  title: string;
  items: WorldListItem[];
  emptyText: string;
}> = ({ title, items, emptyText }) => (
  <section className="a4n-flat-surface min-w-0 p-4">
    <div className="mb-2 flex items-baseline gap-2 border-b border-book-border/40 pb-1.5">
      <div className="text-xs font-semibold text-book-text-main">{title}</div>
      {items.length > 0 ? (
        <span className="text-small text-book-text-muted">{items.length} 项</span>
      ) : null}
    </div>
    {items.length === 0 ? (
      <div className="text-xs text-book-text-muted">{emptyText}</div>
    ) : (
      <ul className="divide-y divide-book-border/35">
        {items.slice(0, 3).map((item, index) => (
          <li
            key={`${item.title ?? item.description.slice(0, 32)}-${index}`}
            className="min-w-0 py-1.5 first:pt-0 last:pb-0"
          >
            {item.title ? (
              <div className="truncate text-xs font-semibold text-book-text-main">{item.title}</div>
            ) : null}
            {item.description ? (
              <div
                className={`${item.title ? "mt-0.5 " : ""}line-clamp-2 text-small leading-relaxed text-book-text-muted`}
              >
                {item.description}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    )}
    {items.length > 3 ? (
      <div className="mt-2 text-caption text-book-text-muted">
        另有 {items.length - 3} 项，可在完整设定中查看
      </div>
    ) : null}
  </section>
);
