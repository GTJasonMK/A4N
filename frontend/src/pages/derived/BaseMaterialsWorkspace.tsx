import { lazy, Suspense, useState } from "react";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { useToast } from "../../components/feedback/Toast";
import { useProgressTask } from "../../hooks/useProgressTask";
import { useI18n } from "../../i18n";

const CharacterPortraitsPanel = lazy(() => import("../novel-detail/characters/CharacterPortraitsPanel").then((module) => ({ default: module.CharacterPortraitsPanel })));
const VideoMaterialWorkspace = lazy(() => import("../novel-detail/video/VideoMaterialWorkspace").then((module) => ({ default: module.VideoMaterialWorkspace })));

export function BaseMaterialsWorkspace({ projectId, canEdit, onChanged, onNext }: {
  projectId: number;
  canEdit: boolean;
  onChanged: () => Promise<boolean | void> | boolean | void;
  onNext: () => void;
}) {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [kind, setKind] = useState<"characters" | "scenes_objects">("characters");
  return <div className="flex min-h-0 flex-1 flex-col gap-3">
    <header className="flex flex-wrap items-start gap-3">
      <div className="min-w-0 flex-1">
        <h2 className="text-base font-semibold">{t("common.mediaAuthoring.materials")}</h2>
        <p className="mt-1 text-xs text-book-text-muted">{t("common.mediaAuthoring.materialsHint")}</p>
      </div>
      <BookButton size="sm" variant="secondary" onClick={onNext}>{t("common.mediaAuthoring.nextStage")}</BookButton>
    </header>
    <SegmentedControl ariaLabel={t("common.mediaAuthoring.materials")} value={kind} onChange={setKind} options={[
      { id: "characters", label: t("common.mediaAuthoring.characterMaterials") },
      { id: "scenes_objects", label: t("common.mediaAuthoring.sceneObjectMaterials") },
    ]} />
    <fieldset disabled={!canEdit} className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto custom-scrollbar">
      <legend className="sr-only">{t("common.mediaAuthoring.materials")}</legend>
      <Suspense fallback={<LoadingSpinner size="sm" />}>
        {kind === "characters" ? <CharacterPortraitsPanel projectId={projectId} />
          : <VideoMaterialWorkspace projectId={projectId} onToast={addToast} runProgressTask={runProgressTask} onRefreshVideoData={onChanged} />}
      </Suspense>
    </fieldset>
  </div>;
}
