import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Check, RefreshCw, Save, Sparkles } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  listAgentSkills,
  updateAgentSkillConfiguration,
  type AgentSkillDescriptor,
  type AgentSkillProfileView,
  type AgentSkillSettingsView,
} from "../../api/agent";
import { useToast } from "../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { useI18n } from "../../i18n";

function profileLabel(
  capability: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    main: "settingsAgent.profiles.main",
    general: "settingsAgent.profiles.general",
    content_optimization: "settingsAgent.profiles.contentOptimization",
    canvas: "settingsAgent.profiles.canvas",
    director: "settingsAgent.profiles.director",
  };
  const key = labels[capability];
  return key ? t(key) : capability;
}

function cloneProfiles(settings: AgentSkillSettingsView): Record<string, Set<string>> {
  return Object.fromEntries(
    settings.profiles.map((profile) => [
      profile.capability,
      new Set(profile.skills.filter((skill) => skill.enabled).map((skill) => skill.id)),
    ]),
  );
}

export const AgentTab: React.FC = () => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [settings, setSettings] = useState<AgentSkillSettingsView | null>(null);
  const [draft, setDraft] = useState<Record<string, Set<string>>>({});
  const [activeCapability, setActiveCapability] = useState<string>("main");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await listAgentSkills();
      setSettings(next);
      setDraft(cloneProfiles(next));
      setActiveCapability((current) =>
        next.profiles.some((profile) => profile.capability === current)
          ? current
          : next.profiles[0]?.capability ?? "",
      );
      setDirty(false);
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsAgent.loadFailed")), "error");
    } finally {
      setLoading(false);
    }
  }, [addToast, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const activeProfile = useMemo<AgentSkillProfileView | null>(
    () => settings?.profiles.find((profile) => profile.capability === activeCapability) ?? null,
    [activeCapability, settings],
  );
  const activeEnabled = draft[activeCapability] ?? new Set<string>();

  const toggleSkill = (skill: AgentSkillDescriptor) => {
    const next = new Set(activeEnabled);
    if (next.has(skill.id)) next.delete(skill.id);
    else next.add(skill.id);
    setDraft((current) => ({ ...current, [activeCapability]: next }));
    setDirty(true);
  };

  const save = async () => {
    if (!settings || !dirty || saving) return;
    const profile = settings.profiles.find((item) => item.capability === activeCapability);
    if (!profile) return;
    setSaving(true);
    try {
      const updated = await updateAgentSkillConfiguration({
        capability: activeCapability,
        enabled_skill_ids: [...activeEnabled],
      });
      setSettings((current) =>
        current
          ? {
              profiles: current.profiles.map((item) =>
                item.capability === updated.capability ? updated : item,
              ),
            }
          : current,
      );
      setDraft((current) => ({
        ...current,
        [updated.capability]: new Set(
          updated.skills.filter((skill) => skill.enabled).map((skill) => skill.id),
        ),
      }));
      setDirty(false);
      addToast(t("settingsAgent.saved"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsAgent.saveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex h-auto min-h-0 flex-col gap-4 md:h-full">
      <header className="flex shrink-0 flex-wrap items-start justify-between gap-3 border-b border-book-border/60 pb-3">
        <div className="min-w-0">
          <div className="mb-1 flex items-center gap-2">
            <Sparkles size={16} className="text-book-primary" aria-hidden="true" />
            <h2 className="font-serif text-lg font-semibold text-book-text-main">
              {t("settingsAgent.title")}
            </h2>
          </div>
          <p className="max-w-2xl text-xs leading-5 text-book-text-muted">
            {t("settingsAgent.description")}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <BookIconButton
            variant="ghost"
            label={t("settingsAgent.refresh")}
            title={t("settingsAgent.refresh")}
            onClick={() => void refresh()}
            disabled={loading || saving || dirty}
          >
            <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
          <BookButton
            variant="primary"
            onClick={() => void save()}
            disabled={!dirty || saving || loading}
          >
            <Save size={14} />
            {saving ? t("settingsAgent.saving") : t("common.save")}
          </BookButton>
        </div>
      </header>

      <div className="grid min-h-0 gap-4 md:grid-cols-[11rem_minmax(0,1fr)] md:flex-1">
        <nav aria-label={t("settingsAgent.agentTypes")} className="min-w-0">
          <div className="flex gap-1 overflow-x-auto pb-1 md:block md:space-y-1 md:overflow-visible">
            {(settings?.profiles ?? []).map((profile) => {
              const capability = profile.capability;
              const enabledCount = draft[capability]?.size ?? 0;
              return (
                <button
                  key={capability}
                  type="button"
                  className={`flex flex-none items-center gap-2 border-b-2 px-2.5 py-2 text-left text-xs transition-colors md:min-w-0 md:w-full md:border-b-0 md:border-l-2 ${
                    capability === activeCapability
                      ? "border-book-primary bg-book-primary/8 font-semibold text-book-primary md:border-l-book-primary"
                      : "border-transparent text-book-text-sub hover:bg-book-bg-paper md:hover:border-book-border"
                  }`}
                  aria-current={capability === activeCapability ? "page" : undefined}
                  disabled={dirty && capability !== activeCapability}
                  onClick={() => setActiveCapability(capability)}
                >
                  <span className="whitespace-nowrap md:min-w-0 md:truncate">{profileLabel(capability, t)}</span>
                  <span className="ml-auto text-[10px] text-book-text-muted">
                    {profile ? `${enabledCount}/${profile.skills.length}` : "-"}
                  </span>
                </button>
              );
            })}
          </div>
        </nav>

        <section className="min-w-0 overflow-y-auto rounded-md border border-book-border/60 bg-book-bg-paper/55 px-3 py-3 md:px-4">
          {loading ? (
            <div className="py-12 text-center text-xs text-book-text-muted">
              {t("settingsAgent.loading")}
            </div>
          ) : activeProfile ? (
            <>
              <div className="mb-3 flex flex-wrap items-baseline justify-between gap-2 border-b border-book-border/40 pb-2">
                <div>
                  <h3 className="text-sm font-semibold text-book-text-main">
                    {profileLabel(activeProfile.capability, t)}
                  </h3>
                  <p className="mt-1 text-[11px] text-book-text-muted">
                    {t("settingsAgent.profileHint")}
                  </p>
                </div>
                <span className="text-[11px] text-book-text-muted">
                  {t("settingsAgent.enabledCount", {
                    count: activeEnabled.size,
                    total: activeProfile.skills.length,
                  })}
                </span>
              </div>
              <div className="divide-y divide-book-border/45">
                {activeProfile.skills.map((skill) => {
                  const enabled = activeEnabled.has(skill.id);
                  return (
                    <label
                      key={skill.id}
                      className="flex cursor-pointer items-start gap-3 py-3 first:pt-1 last:pb-1"
                    >
                      <input
                        type="checkbox"
                        checked={enabled}
                        onChange={() => toggleSkill(skill)}
                        className="mt-0.5 h-4 w-4 shrink-0 accent-book-primary"
                      />
                      <span className="min-w-0 flex-1">
                        <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                          <span className="text-xs font-semibold text-book-text-main">
                            {skill.name}
                          </span>
                          <span className="text-[10px] text-book-text-muted">
                            {skill.id} · v{skill.version}
                          </span>
                          {enabled && (
                            <Check size={12} className="text-book-success" aria-label={t("common.enabled")} />
                          )}
                        </span>
                        <span className="mt-1 block text-xs leading-5 text-book-text-sub">
                          {skill.description}
                        </span>
                      </span>
                    </label>
                  );
                })}
              </div>
            </>
          ) : (
            <div className="py-12 text-center text-xs text-book-text-muted">
              {t("settingsAgent.noSkills")}
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
