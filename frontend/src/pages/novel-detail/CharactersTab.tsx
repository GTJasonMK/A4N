import React, { useCallback, useEffect, useRef, useState } from "react";
import { UserPlus } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  createProfile,
  listProfiles,
  updateCharacterProfile,
  type CharacterProfile,
} from "../../api/character";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import {
  CharacterProfileEditorModal,
  EMPTY_PROFILE_FORM,
  profileFormBaseAttributes,
  profileFormFromProfile,
  type ProfileForm,
} from "../writing-desk/CharacterProfileEditorModal";
import { CharacterProfilesPanel } from "./characters/CharacterProfilesPanel";
import { CharacterPortraitsPanel } from "./characters/CharacterPortraitsPanel";
import type { CharactersSubTab } from "./characters/characterUtils";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";
import { useI18n } from "../../i18n";

export const CharactersTab: React.FC<{ projectId: number }> = ({ projectId }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [subTab, setSubTab] = useState<CharactersSubTab>("profile");
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const loadedProjectIdRef = useRef<number | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<CharacterProfile | null>(null);
  const [form, setForm] = useState<ProfileForm>(EMPTY_PROFILE_FORM);
  const [saving, setSaving] = useState(false);
  const activeProjectIdRef = useRef(projectId);
  const saveSeqRef = useRef(0);
  const saveInFlightRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const loadProfiles = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const nextProfiles = await listProfiles(projectId);
        if (!isCurrent()) return;
        setProfiles(nextProfiles);
        loadedProjectIdRef.current = projectId;
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          if (loadedProjectIdRef.current !== projectId) setProfiles([]);
          loadedProjectIdRef.current = projectId;
          setLoadedProjectId(projectId);
        }
        throw err;
      }
    },
    [projectId],
  );
  const { loading, load } = useChapterAnalysisLoader(
    projectId,
    loadProfiles,
    t("characterDetail.loadFailed"),
  );

  useEffect(() => {
    saveSeqRef.current += 1;
    saveInFlightRef.current = false;
    setEditorOpen(false);
    setEditingProfile(null);
    setForm(EMPTY_PROFILE_FORM);
    setSaving(false);
  }, [projectId]);

  const openCreate = () => {
    setEditingProfile(null);
    setForm(EMPTY_PROFILE_FORM);
    setEditorOpen(true);
  };

  const openEdit = (profile: CharacterProfile) => {
    setEditingProfile(profile);
    setForm(profileFormFromProfile(profile));
    setEditorOpen(true);
  };

  const handleSave = async () => {
    const name = form.name.trim();
    if (!name) {
      addToast(t("characterDetail.nameRequiredError"), "error");
      return;
    }
    if (saveInFlightRef.current) return;
    saveInFlightRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const result = editingProfile
          ? await updateCharacterProfile({
            project_id: requestProjectId,
            profile_id: editingProfile.id,
            expected_current_state: editingProfile.current_state,
            base_attributes: profileFormBaseAttributes(
              form,
              editingProfile.current_state,
            ),
          })
        : await createProfile({
            project_id: requestProjectId,
            name,
            base_attributes: profileFormBaseAttributes(form),
          });
      if (
        activeProjectIdRef.current !== requestProjectId ||
        saveSeqRef.current !== requestSeq
      ) {
        return;
      }
      if (!await load({ silent: true, force: true })) return;
      setEditorOpen(false);
      setEditingProfile(null);
      addToast(
        result.warnings.length > 0
          ? t(
              editingProfile
                ? "characterDetail.profileUpdatedWithWarnings"
                : "characterDetail.profileCreatedWithWarnings",
              {
                ...(editingProfile ? {} : { name }),
                warnings: result.warnings.join(t("common.summarySeparator")),
              },
            )
          : editingProfile
            ? t("characterDetail.profileUpdated")
            : t("characterDetail.profileCreated", { name }),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (
        activeProjectIdRef.current === requestProjectId &&
        saveSeqRef.current === requestSeq
      ) {
        addToast(
          extractErrorMessage(
            err,
            editingProfile
              ? t("characterDetail.profileSaveFailed")
              : t("characterDetail.profileCreateFailed"),
          ),
          "error",
        );
      }
    } finally {
      if (
        activeProjectIdRef.current === requestProjectId &&
        saveSeqRef.current === requestSeq
      ) {
        saveInFlightRef.current = false;
        setSaving(false);
      }
    }
  };

  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  if (displayLoading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("characterDetail.loadingProfiles")}
      </div>
    );
  }

  return (
    <div className="content-main flex min-h-0 flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <SegmentedControl
          ariaLabel={t("characterDetail.profileWorkspaceAria")}
          value={subTab}
          onChange={setSubTab}
          options={[
            { id: "profile" as const, label: t("characterDetail.profileTab") },
            { id: "portrait" as const, label: t("characterDetail.portraitTab") },
          ]}
        />
        {subTab === "profile" ? (
          <BookButton variant="primary" size="sm" onClick={openCreate} disabled={saving}>
            <UserPlus size={12} /> {t("characterDetail.createProfile")}
          </BookButton>
        ) : null}
      </div>

      {subTab === "profile" ? (
        <CharacterProfilesPanel
          projectId={projectId}
          profiles={displayProfiles}
          onChanged={() => load({ silent: true, force: true })}
          onEdit={openEdit}
        />
      ) : (
        <CharacterPortraitsPanel projectId={projectId} />
      )}

      <CharacterProfileEditorModal
        open={editorOpen}
        form={form}
        saving={saving}
        mode={editingProfile ? "edit" : "create"}
        onChange={setForm}
        onSave={handleSave}
        onClose={() => {
          setEditorOpen(false);
          setEditingProfile(null);
        }}
      />
    </div>
  );
};
