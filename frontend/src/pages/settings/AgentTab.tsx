import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RefreshCw, Save, Sparkles } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  listAgentSkills,
  updateAgentSkillConfiguration,
  type AgentSkillDescriptor,
  type AgentSkillProfileView,
  type AgentSkillSettingsView,
} from "../../api/agent";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";

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
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const dirty = useMemo(() => settings?.profiles.some((profile) =>
    profile.skills.some((skill) => skill.enabled !== draft[profile.capability]?.has(skill.id)),
  ) ?? false, [draft, settings]);
  const dirtyRef = useRef(dirty);
  const tRef = useRef(t);
  const mountedRef = useRef(true);
  const refreshInFlightRef = useRef(false);
  const savingRef = useRef(false);
  dirtyRef.current = dirty;
  tRef.current = t;
  useDirty("settings-agent-skills", dirty, undefined, () => !saving);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    if (refreshInFlightRef.current) return;
    if (savingRef.current) {
      addToast(tRef.current("common.operationInProgress"), "info");
      return;
    }
    refreshInFlightRef.current = true;
    try {
      if (dirtyRef.current && !(await confirmDialog({
        title: tRef.current("common.unsavedChangesTitle"),
        message: tRef.current("common.unsavedChangesMessage", {
          name: tRef.current("settingsAgent.title"),
          action: tRef.current("settingsAgent.refresh"),
        }),
        confirmText: tRef.current("common.discardDraft"),
        dialogType: "warning",
      }))) return;
      if (!mountedRef.current) return;
      setLoading(true);
      setLoadError(null);
      try {
        const next = await listAgentSkills();
        if (!mountedRef.current) return;
        setSettings(next);
        setDraft(cloneProfiles(next));
        setActiveCapability((current) =>
          next.profiles.some((profile) => profile.capability === current)
            ? current
            : next.profiles[0]?.capability ?? "",
        );
      } catch (error) {
        if (!mountedRef.current) return;
        setLoadError({ reason: error });
      } finally {
        if (mountedRef.current) setLoading(false);
      }
    } finally {
      refreshInFlightRef.current = false;
    }
  }, [addToast]);

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
  };

  const save = async () => {
    if (!settings || !dirty || saving) return;
    const profile = settings.profiles.find((item) => item.capability === activeCapability);
    if (!profile) return;
    const submitted = activeEnabled;
    savingRef.current = true;
    setSaving(true);
    try {
      const updated = await updateAgentSkillConfiguration({
        capability: activeCapability,
        enabled_skill_ids: [...submitted],
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
      // 保存期间仍允许调整；旧回包不能覆盖更新的选择。
      setDraft((current) => current[updated.capability] !== submitted ? current : ({
        ...current,
        [updated.capability]: new Set(
          updated.skills.filter((skill) => skill.enabled).map((skill) => skill.id),
        ),
      }));
      addToast(t("settingsAgent.saved"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsAgent.saveFailed")), "error");
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <div
      className="flex h-auto min-h-0 flex-col gap-4 md:h-full"
      data-settings-dirty={dirty ? "true" : undefined}
    >
      <header className="flex shrink-0 flex-wrap items-start justify-between gap-3 border-b border-book-border/60 pb-3">
        <div className="min-w-0">
          <div className="mb-1 flex items-center gap-2">
            <Sparkles size={16} className="text-book-primary" aria-hidden="true" />
            <h2 className="a4n-section-heading">
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
        </div>
      </header>

      {loadError ? (
        <div role="alert" className="a4n-context-bar text-sm text-book-text-main">
          <span className="min-w-0 flex-1">
            {extractErrorMessage(loadError.reason, t("settingsAgent.loadFailed"))}
          </span>
          <BookButton variant="secondary" size="sm" onClick={() => void refresh()} disabled={loading}>{t("common.retry")}</BookButton>
        </div>
      ) : null}
      <div className="grid min-h-0 gap-4 md:grid-cols-[11rem_minmax(0,1fr)] md:grid-rows-[minmax(0,1fr)] md:flex-1">
        <nav aria-label={t("settingsAgent.agentTypes")} className="min-w-0">
          <div className="md:hidden">
            <ThemedSelect aria-label={t("settingsAgent.agentTypes")} value={activeCapability} onChange={setActiveCapability} className="w-full">
              {(settings?.profiles ?? []).map((profile) => (
                <option key={profile.capability} value={profile.capability} disabled={dirty && profile.capability !== activeCapability}>
                  {profileLabel(profile.capability, t)} · {draft[profile.capability]?.size ?? 0}/{profile.skills.length}
                </option>
              ))}
            </ThemedSelect>
          </div>
          <div className="hidden md:block md:space-y-1">
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

        <section className="a4n-flat-surface min-h-0 max-h-full min-w-0 self-start overflow-y-auto px-3 py-3 md:px-4">
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
                          <span className="text-sm font-semibold text-book-text-main">
                            {skill.name}
                          </span>
                        </span>
                        <span className="mt-1 block text-xs leading-5 text-book-text-sub">
                          {skill.description}
                        </span>
                        <span className="mt-1 block text-caption text-book-text-muted">{skill.id} · v{skill.version}</span>
                      </span>
                    </label>
                  );
                })}
              </div>
              <div className="mt-3 flex justify-end gap-2 border-t border-book-border/40 pt-3">
                <BookButton variant="ghost" onClick={() => settings && setDraft(cloneProfiles(settings))} disabled={!dirty || saving || loading}>{t("common.cancel")}</BookButton>
                <BookButton onClick={() => void save()} disabled={!dirty || saving || loading}>
                  <Save size={14} />
                  {saving ? t("settingsAgent.saving") : t("common.save")}
                </BookButton>
              </div>
            </>
          ) : !loadError ? (
            <div className="py-12 text-center text-xs text-book-text-muted">
              {t("settingsAgent.noSkills")}
            </div>
          ) : null}
        </section>
      </div>
    </div>
  );
};
