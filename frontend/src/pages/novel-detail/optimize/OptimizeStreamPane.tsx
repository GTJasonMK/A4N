import { StreamMessage } from "./OptimizeStreamMessage";
import type { StreamItem } from "./optimizeTabUtils";
import { useI18n } from "../../../i18n";
import type { Ref } from "react";
import { FollowLatestButton } from "../../../components/ui/FollowLatestButton";

interface OptimizeStreamPaneProps {
  streamItems: StreamItem[];
  streamRef: Ref<HTMLDivElement>;
  isFollowingLatest: boolean;
  hasNewContent: boolean;
  onScrollToLatest: () => void;
  onStreamDrained: (taskId: string) => void;
}

export function OptimizeStreamPane({
  streamItems,
  streamRef,
  isFollowingLatest,
  hasNewContent,
  onScrollToLatest,
  onStreamDrained,
}: OptimizeStreamPaneProps) {
  const { t } = useI18n();
  return (
    <div className="relative min-h-0 min-w-0 flex-1">
      <div
        ref={streamRef}
        className="custom-scrollbar h-full min-h-0 min-w-0 space-y-2 overflow-y-auto overflow-x-hidden px-2 py-3"
        style={{ overflowAnchor: "none" }}
      >
        {streamItems.length === 0 ? (
          <div className="flex h-full min-h-[14rem] min-w-0 items-center justify-center rounded-2xl border border-dashed border-book-border/70 bg-book-bg-paper/45 px-4 text-center text-xs leading-relaxed text-book-text-sub">
            {t("optimize.streamEmpty")}
          </div>
        ) : (
          streamItems.map((item) => (
            <StreamMessage
              key={item.id}
              item={item}
              onStreamDrained={onStreamDrained}
            />
          ))
        )}
      </div>
      <FollowLatestButton
        isFollowingLatest={isFollowingLatest}
        hasNewContent={hasNewContent}
        newContentLabel={t("agent.domain.newMessages")}
        latestLabel={t("agent.domain.backToLatest")}
        onClick={onScrollToLatest}
        className="absolute bottom-3 left-1/2 z-10 -translate-x-1/2"
      />
    </div>
  );
}
