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
import { formatNumber, useI18n } from "../../../i18n";

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
  emptyText,
}) => {
  const { t } = useI18n();
  const text = valueToText(value, t).trim();
  return (
    <div className="whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
      {text || (
        <span className="text-book-text-muted">
          {emptyText ?? t("novel.world.defaultEmpty")}
        </span>
      )}
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
  const { locale, t } = useI18n();
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
            title={t("novel.world.extraSettings")}
            meta={t("novel.world.itemCount", {
              count: formatNumber(entries.length, locale),
            })}
          />
          <dl className="divide-y divide-book-border/35 border-y border-book-border/35">
            {entries.map(([key, value]) => (
              <div
                key={key}
                className="grid gap-1 py-2 md:grid-cols-[120px_minmax(0,1fr)] md:gap-3"
              >
                <dt className="text-small font-medium text-book-text-muted">
                  {formatFieldLabel(key, t)}
                </dt>
                <dd className="whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">
                  {summarizeExtraValue(value, t)}
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
  const { locale, t } = useI18n();
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
  const locationItems = normalizeList(locations?.[1], t);
  const factionItems = normalizeList(factions?.[1], t);
  const historyItems = normalizeList(history?.[1], t);
  const conflictItems = normalizeList(conflicts?.[1], t);
  const mysteryItems = normalizeList(mysteries?.[1], t);
  const promiseItems = normalizeList(promises?.[1], t);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title={t("novel.world.premise")} />
          <TextBlock value={premise?.[1]} emptyText={t("novel.world.premiseEmpty")} />
        </section>

        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title={t("novel.world.coreRules")} />
          <TextBlock value={core?.[1]} emptyText={t("novel.world.coreEmpty")} />
        </section>
      </div>

      <section className="min-w-0 border-t border-book-border/35 pt-3">
        <SectionTitle icon={<Braces size={13} />} title={t("novel.world.system")} />
        <TextBlock value={system?.[1]} emptyText={t("novel.world.systemEmpty")} />
      </section>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<MapPin size={13} />}
            title={t("novel.world.locations")}
            meta={locationItems.length ? t("novel.world.itemCount", { count: formatNumber(locationItems.length, locale) }) : undefined}
          />
          <ListBlock items={locationItems} emptyText={t("novel.world.locationsEmpty")} />
        </section>

        <section className="min-w-0">
          <SectionTitle
            icon={<Network size={13} />}
            title={t("novel.world.factions")}
            meta={factionItems.length ? t("novel.world.itemCount", { count: formatNumber(factionItems.length, locale) }) : undefined}
          />
          <ListBlock items={factionItems} emptyText={t("novel.world.factionsEmpty")} />
        </section>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title={t("novel.world.history")}
            meta={historyItems.length ? t("novel.world.itemCount", { count: formatNumber(historyItems.length, locale) }) : undefined}
          />
          <ListBlock items={historyItems} emptyText={t("novel.world.historyEmpty")} />
        </section>

        <section className="min-w-0">
          <SectionTitle icon={<ScrollText size={13} />} title={t("novel.world.culture")} />
          <TextBlock value={culture?.[1]} emptyText={t("novel.world.cultureEmpty")} />
        </section>
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <section className="min-w-0">
          <SectionTitle
            icon={<Network size={13} />}
            title={t("novel.world.conflicts")}
            meta={conflictItems.length ? t("novel.world.itemCount", { count: formatNumber(conflictItems.length, locale) }) : undefined}
          />
          <ListBlock items={conflictItems} emptyText={t("novel.world.conflictsEmpty")} />
        </section>

        <section className="min-w-0">
          <SectionTitle
            icon={<ScrollText size={13} />}
            title={t("novel.world.mysteries")}
            meta={mysteryItems.length ? t("novel.world.itemCount", { count: formatNumber(mysteryItems.length, locale) }) : undefined}
          />
          <ListBlock items={mysteryItems} emptyText={t("novel.world.mysteriesEmpty")} />
        </section>
      </div>

      <section className="min-w-0 border-t border-book-border/35 pt-3">
        <SectionTitle
          icon={<ScrollText size={13} />}
          title={t("novel.world.promises")}
          meta={promiseItems.length ? t("novel.world.itemCount", { count: formatNumber(promiseItems.length, locale) }) : undefined}
        />
        <ListBlock items={promiseItems} emptyText={t("novel.world.promisesEmpty")} />
      </section>

      <ExtraFields obj={obj} consumedKeys={consumedKeys} />
    </div>
  );
};

export const WorldSummaryView: React.FC<{ obj: WorldObject }> = ({ obj }) => {
  const { t } = useI18n();
  const premise = pickFirst(obj, PREMISE_KEYS);
  const core = pickFirst(obj, CORE_KEYS);
  const locations = normalizeList(pickFirst(obj, LOCATION_KEYS)?.[1], t);
  const factions = normalizeList(pickFirst(obj, FACTION_KEYS)?.[1], t);

  return (
    <div className="grid grid-cols-1 items-start gap-3 lg:grid-cols-2">
      <section className="a4n-flat-surface min-w-0 p-4">
        <SectionTitle icon={<ScrollText size={13} />} title={t("novel.world.premise")} />
        <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
          {valueToText(premise?.[1], t) || t("novel.world.premiseEmpty")}
        </div>
      </section>
      <section className="a4n-flat-surface min-w-0 p-4">
        <SectionTitle icon={<ScrollText size={13} />} title={t("novel.world.coreRules")} />
        <div className="line-clamp-5 text-xs leading-relaxed text-book-text-sub">
          {valueToText(core?.[1], t) || t("novel.world.coreEmpty")}
        </div>
      </section>
      <WorldMiniList title={t("novel.world.locations")} items={locations} emptyText={t("novel.world.locationsEmpty")} />
      <WorldMiniList title={t("novel.world.factions")} items={factions} emptyText={t("novel.world.factionsEmpty")} />
    </div>
  );
};

const WorldMiniList: React.FC<{
  title: string;
  items: WorldListItem[];
  emptyText: string;
}> = ({ title, items, emptyText }) => {
  const { locale, t } = useI18n();
  return (
    <section className="a4n-flat-surface min-w-0 p-4">
      <div className="mb-2 flex items-baseline gap-2 border-b border-book-border/40 pb-1.5">
        <div className="text-xs font-semibold text-book-text-main">{title}</div>
        {items.length > 0 ? (
          <span className="text-small text-book-text-muted">
            {t("novel.world.itemCount", { count: formatNumber(items.length, locale) })}
          </span>
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
          {t("novel.world.moreItems", {
            count: formatNumber(items.length - 3, locale),
          })}
        </div>
      ) : null}
    </section>
  );
};
