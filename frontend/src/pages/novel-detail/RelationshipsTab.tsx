import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Network,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import { ThemedCombobox } from "../../components/ui/ThemedCombobox";
import {
  listProfiles,
  updateCharacterProfile,
  type CharacterProfile,
} from "../../api/character";
import {
  characterAttributeEntries,
  characterAttributeValueText,
} from "../../utils/characterAttributes";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";
import { profileRelationshipBaseAttributes } from "../writing-desk/CharacterProfileEditorModal";
import { CharacterDetailDialog } from "../CharacterDetailDialog";
import { useI18n, type MessageTranslator } from "../../i18n";
import {
  CurrentRelationshipsCard,
  relationshipNodeId,
  type CurrentRelationship,
} from "./relationships/RelationshipGraphPanel";

export const relationshipEditMode = (
  editable: boolean,
  timelineStarted: boolean,
): "initial" | "timeline" | "readonly" => {
  if (editable) return "initial";
  return timelineStarted ? "timeline" : "readonly";
};

interface RelationshipEditorState {
  profileId: number;
  target: string;
  description: string;
  editing: boolean;
  initialProfileId: number;
  initialTarget: string;
  initialDescription: string;
}

const syncedChapterText = (
  chapter: number,
  timelineStarted: boolean,
  t: MessageTranslator,
): string =>
  chapter > 0
    ? t("novel.relationships.syncedChapter", { chapter })
    : timelineStarted
      ? t("novel.relationships.chapterRecords")
      : t("novel.relationships.notSynced");

const currentRelationshipsFromProfiles = (
  profiles: CharacterProfile[],
): CurrentRelationship[] =>
  profiles.flatMap((profile) => {
    const characterA = profile.name.trim();
    if (!characterA) return [];
    return characterAttributeEntries(profile, "social")
      .map(([target, value]) => {
        const characterB = target.trim();
        const description = characterAttributeValueText(value);
        if (!characterB || !description) return null;
        return {
          id: `${profile.id}:${characterB}`,
          profile_id: profile.id,
          character_a: characterA,
          character_b: characterB,
          description,
          last_synced_chapter: profile.last_synced_chapter,
          timeline_started: profile.timeline_started,
          structured: typeof value !== "string",
          editable:
            profile.initial_profile_editable && typeof value === "string",
        };
      })
      .filter((item): item is CurrentRelationship => item != null);
  });

export const RelationshipsTab: React.FC<{ projectId: number }> = ({
  projectId,
}) => {
  const { t } = useI18n();
  const { addToast } = useToast();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [currentDetail, setCurrentDetail] =
    useState<CurrentRelationship | null>(null);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const loadedProjectIdRef = useRef<number | null>(null);
  const [editor, setEditor] = useState<RelationshipEditorState | null>(null);
  const [timelineRelationship, setTimelineRelationship] =
    useState<CurrentRelationship | null>(null);
  const [saving, setSaving] = useState(false);
  const activeProjectIdRef = useRef(projectId);
  const mutationSeqRef = useRef(0);
  const mutationInFlightRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const loadRelationships = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const data = await listProfiles(projectId);
        if (!isCurrent()) return;
        setProfiles(data);
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
    loadRelationships,
    t("novel.relationships.loadFailed"),
  );

  useEffect(() => {
    mutationSeqRef.current += 1;
    mutationInFlightRef.current = false;
    setCurrentDetail(null);
    setEditor(null);
    setTimelineRelationship(null);
    setSaving(false);
  }, [projectId]);

  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;
  const currentRels = useMemo(
    () => currentRelationshipsFromProfiles(displayProfiles),
    [displayProfiles],
  );
  const editableProfiles = useMemo(
    () => displayProfiles.filter((profile) => profile.initial_profile_editable),
    [displayProfiles],
  );

  useEffect(() => {
    setCurrentDetail((current) => {
      if (!current) return null;
      return (
        currentRels.find((relationship) => relationship.id === current.id) ??
        null
      );
    });
    setTimelineRelationship((current) => {
      if (!current) return null;
      return (
        currentRels.find((relationship) => relationship.id === current.id) ??
        null
      );
    });
  }, [currentRels]);

  const persistInitialRelationship = async (
    profile: CharacterProfile,
    target: string,
    description: string | null,
    successMessage: string,
  ): Promise<boolean> => {
    if (mutationInFlightRef.current) return false;
    mutationInFlightRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = mutationSeqRef.current + 1;
    mutationSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const result = await updateCharacterProfile({
        project_id: requestProjectId,
        profile_id: profile.id,
        expected_current_state: profile.current_state,
        base_attributes: profileRelationshipBaseAttributes(
          profile.current_state,
          target,
          description,
        ),
      });
      if (
        activeProjectIdRef.current !== requestProjectId ||
        mutationSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setProfiles((current) =>
        current.map((item) => (item.id === result.profile.id ? result.profile : item)),
      );
      if (!(await load({ silent: true, force: true }))) return false;
      addToast(
        result.warnings.length > 0
          ? `${successMessage}${t("common.summarySeparator")}${result.warnings.join(t("common.summarySeparator"))}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
      return true;
    } catch (error) {
      if (
        activeProjectIdRef.current === requestProjectId &&
        mutationSeqRef.current === requestSeq
      ) {
        addToast(extractErrorMessage(error, t("novel.relationships.saveFailed")), "error");
      }
      return false;
    } finally {
      if (
        activeProjectIdRef.current === requestProjectId &&
        mutationSeqRef.current === requestSeq
      ) {
        mutationInFlightRef.current = false;
        setSaving(false);
      }
    }
  };

  const openCreate = () => {
    const profile = editableProfiles[0];
    if (!profile) {
      addToast(
        displayProfiles.length === 0
          ? t("novel.relationships.noProfiles")
          : displayProfiles.every((item) => item.timeline_started)
            ? t("novel.relationships.allInTimeline")
            : t("novel.relationships.cannotEditInitial"),
        "info",
      );
      return;
    }
    setEditor({
      profileId: profile.id,
      target: "",
      description: "",
      editing: false,
      initialProfileId: profile.id,
      initialTarget: "",
      initialDescription: "",
    });
  };

  const openEdit = (relationship: CurrentRelationship) => {
    const mode = relationshipEditMode(
      relationship.editable,
      relationship.timeline_started,
    );
    if (mode !== "initial") {
      if (mode === "timeline") {
        setCurrentDetail(null);
        setTimelineRelationship(relationship);
      } else {
        addToast(
          relationship.structured
            ? t("novel.relationships.cannotEditStructured")
            : t("novel.relationships.cannotEdit"),
          "info",
        );
      }
      return;
    }
    setEditor({
      profileId: relationship.profile_id,
      target: relationship.character_b,
      description: relationship.description,
      editing: true,
      initialProfileId: relationship.profile_id,
      initialTarget: relationship.character_b,
      initialDescription: relationship.description,
    });
  };

  const saveRelationship = async () => {
    if (!editor) return;
    const profile = displayProfiles.find((item) => item.id === editor.profileId);
    const target = editor.target.trim();
    const description = editor.description.trim();
    if (!profile) {
      addToast(t("novel.relationships.missingOwner"), "error");
      return;
    }
    if (!target || !description) {
      addToast(t("novel.relationships.fieldsRequired"), "error");
      return;
    }
    if (relationshipNodeId(profile.name) === relationshipNodeId(target)) {
      addToast(t("novel.relationships.sameCharacter"), "error");
      return;
    }
    if (
      !editor.editing &&
      currentRels.some(
        (relationship) =>
          relationship.profile_id === profile.id &&
          relationshipNodeId(relationship.character_b) === relationshipNodeId(target),
      )
    ) {
      addToast(t("novel.relationships.duplicate"), "error");
      return;
    }
    if (
      await persistInitialRelationship(
        profile,
        target,
        description,
        editor.editing ? t("novel.relationships.updated") : t("novel.relationships.created"),
      )
    ) {
      setEditor(null);
    }
  };

  const deleteRelationship = async (relationship: CurrentRelationship) => {
    if (!relationship.editable) {
      addToast(
        relationship.timeline_started
          ? t("novel.relationships.timelineDelete")
          : relationship.structured
            ? t("novel.relationships.structuredDelete")
            : t("novel.relationships.cannotDelete"),
        "info",
      );
      return;
    }
    const confirmed = await confirmDialog({
      title: t("novel.relationships.deleteTitle"),
      message: t("novel.relationships.deleteMessage", {
        from: relationship.character_a,
        to: relationship.character_b,
      }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!confirmed) return;
    const profile = displayProfiles.find(
      (item) => item.id === relationship.profile_id,
    );
    if (!profile) {
      addToast(t("novel.relationships.missingOwner"), "error");
      return;
    }
    if (
      await persistInitialRelationship(
        profile,
        relationship.character_b,
        null,
        t("novel.relationships.deleted"),
      )
    ) {
      setCurrentDetail(null);
      setEditor(null);
    }
  };

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("common.loading")}
      </div>
    );
  }

  const timelineProfile = timelineRelationship
    ? (displayProfiles.find(
        (profile) => profile.id === timelineRelationship.profile_id,
      ) ?? null)
    : null;

  return (
    <div className="content-wide flex min-h-0 flex-col gap-3 lg:h-full">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="text-small leading-relaxed text-book-text-muted">
          {t("novel.relationships.beforeTimelineHint")}
        </div>
        <BookButton
          variant="primary"
          size="sm"
          onClick={openCreate}
          disabled={saving || editableProfiles.length === 0}
          title={
            editableProfiles.length > 0
              ? t("novel.relationships.addInitial")
              : t("novel.relationships.noEditableProfiles")
          }
        >
          <Plus size={12} /> {t("novel.relationships.addInitial")}
        </BookButton>
      </div>

      <CurrentRelationshipsCard
        projectId={projectId}
        relationships={currentRels}
        onOpen={setCurrentDetail}
      />

      {currentDetail && (
        <CurrentRelationshipDetailDialog
          relationship={currentDetail}
          saving={saving}
          onEdit={() => openEdit(currentDetail)}
          onDelete={() => void deleteRelationship(currentDetail)}
          onClose={() => setCurrentDetail(null)}
        />
      )}

      <RelationshipEditorModal
        state={editor}
        profiles={editableProfiles}
        allProfiles={displayProfiles}
        saving={saving}
        onChange={setEditor}
        onSave={() => void saveRelationship()}
        onClose={async () => {
          if (saving) return;
          const dirty = editor != null && (
            editor.profileId !== editor.initialProfileId ||
            editor.target !== editor.initialTarget ||
            editor.description !== editor.initialDescription
          );
          if (dirty && !(await confirmDialog({
            title: t("common.unsavedChangesTitle"),
            message: t("common.unsavedChangesMessage", { name: t("novel.relationships.title"), action: t("common.close") }),
            confirmText: t("common.discardDraft"),
            dialogType: "warning",
          }))) return;
          setEditor(null);
        }}
      />

      <CharacterDetailDialog
        open={timelineRelationship != null && timelineProfile != null}
        onClose={() => setTimelineRelationship(null)}
        projectId={projectId}
        character={timelineProfile}
        currentChapterNumber={timelineRelationship?.last_synced_chapter}
        initialTab="manage"
        initialEventFilter={
          timelineRelationship
            ? { category: "social", key: timelineRelationship.character_b }
            : null
        }
        onChanged={() => load({ silent: true, force: true })}
      />
    </div>
  );
};

const RelationshipEditorModal: React.FC<{
  state: RelationshipEditorState | null;
  profiles: CharacterProfile[];
  allProfiles: CharacterProfile[];
  saving: boolean;
  onChange: (state: RelationshipEditorState | null) => void;
  onSave: () => void;
  onClose: () => void;
}> = ({ state, profiles, allProfiles, saving, onChange, onSave, onClose }) => {
  const { t } = useI18n();
  if (!state) return null;
  const profile = profiles.find((item) => item.id === state.profileId);
  const targetOptions = allProfiles.filter(
    (item) => item.id !== state.profileId,
  );

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={
        state.editing
          ? t("novel.relationships.editorEdit")
          : t("novel.relationships.editorCreate")
      }
      subtitle={t("novel.relationships.editorSubtitle")}
      icon={state.editing ? <Pencil size={15} /> : <Plus size={15} />}
      maxWidthClassName="max-w-lg"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <BookButton
          variant="primary"
          onClick={onSave}
          disabled={
            saving ||
            !profile ||
            !state.target.trim() ||
            !state.description.trim()
          }
        >
          {saving ? (
            <span className="inline-flex items-center gap-2">
              <LoadingSpinner size="sm" /> {t("novel.relationships.saving")}
            </span>
          ) : state.editing ? (
            t("novel.relationships.saveChanges")
          ) : (
            t("novel.relationships.save")
          )}
        </BookButton>
      }
    >
      <div className="grid gap-3">
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">
            {t("novel.relationships.owner")}
          </span>
          <ThemedSelect
            value={state.profileId}
            onChange={(selectedValue) =>
              onChange({
                ...state,
                profileId: Number(selectedValue),
              })
            }
            disabled={saving || state.editing}
            className="pyqt-input"
          >
            {profiles.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </ThemedSelect>
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">
            {t("novel.relationships.target")}
          </span>
          <ThemedCombobox
            value={state.target}
            options={targetOptions.map((item) => ({
              value: item.name,
              label: item.name,
            }))}
            onChange={(target) => onChange({ ...state, target })}
            disabled={saving || state.editing}
            placeholder={t("novel.relationships.targetPlaceholder")}
          />
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">
            {t("novel.relationships.description")}
          </span>
          <textarea
            value={state.description}
            onChange={(event) =>
              onChange({ ...state, description: event.target.value })
            }
            disabled={saving}
            className="pyqt-input min-h-28 resize-y"
            placeholder={t("novel.relationships.descriptionPlaceholder")}
          />
        </label>
      </div>
    </Modal>
  );
};

const CurrentRelationshipDetailDialog: React.FC<{
  relationship: CurrentRelationship;
  saving: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onClose: () => void;
}> = ({ relationship, saving, onEdit, onDelete, onClose }) => {
  const { t } = useI18n();
  const editMode = relationshipEditMode(
    relationship.editable,
    relationship.timeline_started,
  );
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`${relationship.character_a} / ${relationship.character_b}`}
      subtitle={t("novel.relationships.detailSubtitle", {
        status: syncedChapterText(
          relationship.last_synced_chapter,
          relationship.timeline_started,
          t,
        ),
      })}
      icon={<Network size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
      footer={
        editMode === "initial" ? (
          <div className="flex items-center gap-2">
            <BookButton variant="secondary" onClick={onEdit} disabled={saving}>
              <Pencil size={12} /> {t("common.edit")}
            </BookButton>
            <BookButton variant="danger" onClick={onDelete} disabled={saving}>
              <Trash2 size={12} /> {t("common.delete")}
            </BookButton>
          </div>
        ) : editMode === "timeline" ? (
          <BookButton variant="primary" onClick={onEdit} disabled={saving}>
            <Pencil size={12} /> {t("novel.relationships.timeline")}
          </BookButton>
        ) : undefined
      }
    >
      <div>
        <div className="mb-1 text-xs font-semibold text-book-text-main">
          {t("novel.relationships.descriptionLabel")}
        </div>
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {relationship.description}
        </div>
        {editMode !== "initial" ? (
          <div className="mt-4 rounded-xl border border-book-border/60 bg-book-bg-tertiary/45 px-3 py-2 text-small leading-relaxed text-book-text-muted">
            {relationship.timeline_started
              ? t("novel.relationships.timelineNotice")
              : relationship.structured
                ? t("novel.relationships.structuredNotice")
                : t("novel.relationships.readonlyNotice")}
          </div>
        ) : null}
      </div>
    </Modal>
  );
};
