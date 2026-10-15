import { lazy, Suspense, useRef, useState } from "react";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { useToast } from "../../components/feedback/Toast";
import { useProgressTask } from "../../hooks/useProgressTask";
import { useI18n } from "../../i18n";
import { ProjectTextMaterialsPanel, type ProjectTextSourceSelection } from "./ProjectTextMaterialsPanel";

const CharacterPortraitsPanel = lazy(() => import("../novel-detail/characters/CharacterPortraitsPanel").then((module) => ({ default: module.CharacterPortraitsPanel })));
const VideoMaterialWorkspace = lazy(() => import("../novel-detail/video/VideoMaterialWorkspace").then((module) => ({ default: module.VideoMaterialWorkspace })));
const ImagesTab = lazy(() => import("../novel-detail/ImagesTab").then((module) => ({ default: module.ImagesTab })));

export function BaseMaterialsWorkspace({ projectId, textProjectId, canEdit, onChanged, onNext, onBeforeKindChange, showNext = true, standaloneComic = false, onTextDirtyChange, onSelectTextSource, selectedTextSource }: {
  projectId: number;
  textProjectId?: number;
  canEdit: boolean;
  onChanged: () => Promise<boolean | void> | boolean | void;
  onNext: () => void;
  onBeforeKindChange?: () => boolean | Promise<boolean>;
  showNext?: boolean;
  standaloneComic?: boolean;
  onTextDirtyChange?: (dirty: boolean) => void;
  onSelectTextSource?: (selection: ProjectTextSourceSelection) => void;
  selectedTextSource?: ProjectTextSourceSelection | null;
}) {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [kind, setKind] = useState<"characters" | "scenes_objects" | "images" | "text">(
    standaloneComic ? "images" : "characters",
  );
  const kindChangeBusyRef = useRef(false);
  const materialOptions = standaloneComic
    ? [
        { id: "images" as const, label: t("common.mediaAuthoring.images") },
        { id: "text" as const, label: t("novel.textMaterialsTitle") },
      ]
    : [
        { id: "characters" as const, label: t("common.mediaAuthoring.characterMaterials") },
        { id: "scenes_objects" as const, label: t("common.mediaAuthoring.sceneObjectMaterials") },
        { id: "text" as const, label: t("novel.textMaterialsTitle") },
      ];
  const [nexting, setNexting] = useState(false);
  const handleNext = async () => {
    if (nexting) return;
    setNexting(true);
    try {
      await onNext();
    } finally {
      setNexting(false);
    }
  };
  const handleKindChange = (nextKind: typeof kind) => {
    if (nextKind === kind || kindChangeBusyRef.current) return;
    kindChangeBusyRef.current = true;
    void (async () => {
      try {
        if (onBeforeKindChange && !await onBeforeKindChange()) return;
        if (kind === "text") onTextDirtyChange?.(false);
        setKind(nextKind);
      } finally {
        kindChangeBusyRef.current = false;
      }
    })();
  };
  return <div className="flex min-h-0 flex-1 flex-col gap-3">
    <header className="flex flex-wrap items-start gap-3">
      <div className="min-w-0 flex-1">
        <h2 className="text-base font-semibold">{t("common.mediaAuthoring.materials")}</h2>
      </div>
      {showNext ? <BookButton size="sm" variant="secondary" onClick={() => void handleNext()} disabled={nexting}>
        {nexting ? <LoadingSpinner size="sm" /> : null}
        {t("common.mediaAuthoring.nextStage")}
      </BookButton> : null}
    </header>
    <SegmentedControl ariaLabel={t("common.mediaAuthoring.materials")} value={kind} onChange={handleKindChange} options={materialOptions} />
    <fieldset disabled={!canEdit} className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto custom-scrollbar">
      <legend className="sr-only">{t("common.mediaAuthoring.materials")}</legend>
      <Suspense fallback={<LoadingSpinner size="sm" />}>
        {kind === "images" ? <ImagesTab projectId={projectId} canEditImages={canEdit} />
          : kind === "characters" ? <CharacterPortraitsPanel projectId={projectId} canEdit={canEdit} />
          : kind === "scenes_objects"
            ? <VideoMaterialWorkspace projectId={projectId} canEdit={canEdit} onToast={addToast} runProgressTask={runProgressTask} onRefreshVideoData={onChanged} />
            : <ProjectTextMaterialsPanel
                projectId={textProjectId ?? projectId}
                canEdit={canEdit}
                onDirtyChange={onTextDirtyChange}
                onSelectSource={onSelectTextSource}
                selectedSource={selectedTextSource}
              />}
      </Suspense>
    </fieldset>
  </div>;
}
