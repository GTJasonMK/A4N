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

export const CharactersTab: React.FC<{ projectId: number }> = ({ projectId }) => {
  const { addToast } = useToast();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [subTab, setSubTab] = useState<CharactersSubTab>("profile");
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
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
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          setProfiles([]);
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
    "加载角色失败",
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
      addToast("请填写角色姓名", "error");
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
      setEditorOpen(false);
      setEditingProfile(null);
      addToast(
        result.warnings.length > 0
          ? `${editingProfile ? "角色初始档案已更新" : `已创建角色：${name}`}；${result.warnings.join("；")}`
          : editingProfile
            ? "角色初始档案已更新"
            : `已创建角色：${name}`,
        result.warnings.length > 0 ? "info" : "success",
      );
      await load({ silent: true, force: true });
    } catch (err) {
      if (
        activeProjectIdRef.current === requestProjectId &&
        saveSeqRef.current === requestSeq
      ) {
        addToast(
          extractErrorMessage(
            err,
            editingProfile ? "更新角色初始档案失败" : "创建角色失败",
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
        <LoadingSpinner size="sm" /> 加载角色…
      </div>
    );
  }

  return (
    <div className="content-main flex min-h-0 flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <SegmentedControl
          ariaLabel="人物工作区"
          value={subTab}
          onChange={setSubTab}
          options={[
            { id: "profile" as const, label: "档案" },
            { id: "portrait" as const, label: "立绘" },
          ]}
        />
        {subTab === "profile" ? (
          <BookButton variant="primary" size="sm" onClick={openCreate} disabled={saving}>
            <UserPlus size={12} /> 新建角色
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
