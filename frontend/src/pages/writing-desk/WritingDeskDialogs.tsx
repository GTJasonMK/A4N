import React from "react";
import { ScenesDialog } from "../ScenesDialog";
import { CharacterProfilesModal } from "./CharacterProfilesModal";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";

interface WritingDeskDialogsProps {
  projectId: number;
  activeNumber: number | null;
  activeChapterId: number | null;
  scenesOpen: boolean;
  previewOpen: boolean;
  profilesOpen: boolean;
  profilesRefreshKey: number;
  previewing: boolean;
  previewData: ModelRequestPreview | null;
  onScenesClose: () => void;
  onPreviewClose: () => void;
  onProfilesClose: () => void;
}

export const WritingDeskDialogs: React.FC<WritingDeskDialogsProps> = ({
  projectId,
  activeNumber,
  activeChapterId,
  scenesOpen,
  previewOpen,
  profilesOpen,
  profilesRefreshKey,
  previewing,
  previewData,
  onScenesClose,
  onPreviewClose,
  onProfilesClose,
}) => (
  <>
    <ScenesDialog
      open={scenesOpen}
      onClose={onScenesClose}
      chapterId={activeChapterId}
      chapterNumber={activeNumber}
    />

    <RequestPreviewModal
      open={previewOpen}
      loading={previewing}
      data={previewData}
      fallbackTitle={`章节生成请求预览${activeNumber != null ? ` · 第 ${activeNumber} 章` : ""}`}
      onClose={onPreviewClose}
    />

    <CharacterProfilesModal
      open={profilesOpen}
      onClose={onProfilesClose}
      projectId={projectId}
      currentChapterNumber={activeNumber}
      refreshKey={profilesRefreshKey}
    />
  </>
);
