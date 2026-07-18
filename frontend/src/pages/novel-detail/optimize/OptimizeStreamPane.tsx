import { StreamMessage } from "./OptimizeStreamMessage";
import { isNearBottom, type StreamItem } from "./optimizeTabUtils";

interface OptimizeStreamPaneProps {
  streamItems: StreamItem[];
  streamRef: { current: HTMLDivElement | null };
  onStickToBottomChange: (value: boolean) => void;
}

export function OptimizeStreamPane({
  streamItems,
  streamRef,
  onStickToBottomChange,
}: OptimizeStreamPaneProps) {
  return (
    <div
      ref={(element) => {
        streamRef.current = element;
      }}
      onScroll={(event) => {
        onStickToBottomChange(isNearBottom(event.currentTarget));
      }}
      className="custom-scrollbar min-h-0 min-w-0 flex-1 space-y-2 overflow-y-auto overflow-x-hidden px-2 py-3"
    >
      {streamItems.length === 0 ? (
        <div className="flex h-full min-h-[14rem] min-w-0 items-center justify-center rounded-2xl border border-dashed border-book-border/70 bg-book-bg-paper/45 px-4 text-center text-xs leading-relaxed text-book-text-sub">
          输入优化要求后，用户消息和 Agent 回复会显示为对话气泡，过程信息会收进对应气泡。
        </div>
      ) : (
        streamItems.map((item) => <StreamMessage key={item.id} item={item} />)
      )}
    </div>
  );
}
